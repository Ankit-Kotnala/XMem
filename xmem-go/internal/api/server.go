package api

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"regexp"
	"strings"
	"sync"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/xortexai/xmem-go/internal/config"
	"github.com/xortexai/xmem-go/internal/database"
	"github.com/xortexai/xmem-go/internal/pipelines"
)

type Server struct {
	settings  config.Settings
	logger    *slog.Logger
	startedAt time.Time
	ready     bool
	initError string

	ingest    *pipelines.IngestPipeline
	retrieval *pipelines.RetrievalPipeline
	keys      database.APIKeyStore
	limiter   *RateLimiter
}

func NewServer(settings config.Settings, logger *slog.Logger, ingest *pipelines.IngestPipeline, retrieval *pipelines.RetrievalPipeline, keys database.APIKeyStore) *Server {
	return &Server{
		settings:  settings,
		logger:    logger,
		startedAt: time.Now(),
		ready:     true,
		ingest:    ingest,
		retrieval: retrieval,
		keys:      keys,
		limiter:   NewRateLimiter(settings.RateLimit, time.Minute),
	}
}

func (s *Server) Handler() http.Handler {
	router := chi.NewRouter()
	router.Get("/health", s.health)
	router.With(s.memoryMiddleware).Post("/v1/memory/ingest", s.ingestMemory)
	router.With(s.memoryMiddleware).Post("/v1/memory/batch-ingest", s.batchIngestMemory)
	router.With(s.memoryMiddleware).Post("/v1/memory/retrieve", s.retrieveMemory)
	router.With(s.memoryMiddleware).Post("/v1/memory/search", s.searchMemory)
	return s.requestContext(s.securityHeaders(s.cors(s.maxBody(router))))
}

func (s *Server) health(w http.ResponseWriter, r *http.Request) {
	start := time.Now()
	uptime := time.Since(s.startedAt).Seconds()
	status := "ready"
	code := http.StatusOK
	envelopeStatus := "ok"
	if !s.ready {
		status = "loading"
		code = http.StatusServiceUnavailable
		envelopeStatus = "error"
	}
	if s.initError != "" {
		status = "error"
		code = http.StatusServiceUnavailable
		envelopeStatus = "error"
	}
	w.Header().Set("X-Response-Time-Ms", fmt.Sprintf("%.2f", elapsedMS(start)))
	writeJSON(w, r, code, APIResponse{
		Status: envelopeStatus,
		Data: HealthResponse{
			Status:         status,
			PipelinesReady: s.ready && s.initError == "",
			Version:        "1.0.0",
			UptimeSeconds:  &uptime,
			Error:          s.initError,
		},
	})
}

func (s *Server) memoryMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		if !s.ready || s.initError != "" {
			msg := "Pipelines are still loading. Retry shortly."
			if s.initError != "" {
				msg = "Pipeline initialisation failed: " + s.initError
			}
			writeError(w, r, http.StatusServiceUnavailable, msg, start)
			return
		}
		user, code, msg := s.authenticate(r)
		if code != http.StatusOK {
			writeError(w, r, code, msg, start)
			return
		}
		allowed, remaining := s.limiter.Check(user.ID)
		r = r.WithContext(context.WithValue(r.Context(), rateRemainingKey{}, remaining))
		if !allowed {
			w.Header().Set("Retry-After", "60")
			w.Header().Set("X-RateLimit-Limit", fmt.Sprint(s.settings.RateLimit))
			w.Header().Set("X-RateLimit-Remaining", "0")
			writeError(w, r, http.StatusTooManyRequests, "Rate limit exceeded. Try again later.", start)
			return
		}
		r = r.WithContext(context.WithValue(r.Context(), userKey{}, user))
		next.ServeHTTP(w, r)
	})
}

func (s *Server) ingestMemory(w http.ResponseWriter, r *http.Request) {
	start := time.Now()
	user := userFromRequest(r)
	var req IngestRequest
	if !decodeJSON(w, r, &req, start) {
		return
	}
	if err := validateIngest(req); err != nil {
		writeError(w, r, http.StatusUnprocessableEntity, err.Error(), start)
		return
	}
	data, err := s.ingest.Run(r.Context(), req, effectiveUserID(user))
	if err != nil {
		writeError(w, r, http.StatusInternalServerError, err.Error(), start)
		return
	}
	writeData(w, r, data, start)
}

func (s *Server) batchIngestMemory(w http.ResponseWriter, r *http.Request) {
	start := time.Now()
	user := userFromRequest(r)
	var req BatchIngestRequest
	if !decodeJSON(w, r, &req, start) {
		return
	}
	if len(req.Items) == 0 || len(req.Items) > 100 {
		writeError(w, r, http.StatusUnprocessableEntity, "items must contain between 1 and 100 ingest requests", start)
		return
	}
	results := make([]IngestResponse, 0, len(req.Items))
	for _, item := range req.Items {
		if err := validateIngest(item); err != nil {
			writeError(w, r, http.StatusUnprocessableEntity, err.Error(), start)
			return
		}
		data, err := s.ingest.Run(r.Context(), item, effectiveUserID(user))
		if err != nil {
			writeError(w, r, http.StatusInternalServerError, err.Error(), start)
			return
		}
		results = append(results, data)
	}
	writeData(w, r, BatchIngestResponse{Results: results}, start)
}

func (s *Server) retrieveMemory(w http.ResponseWriter, r *http.Request) {
	start := time.Now()
	user := userFromRequest(r)
	var req RetrieveRequest
	if !decodeJSON(w, r, &req, start) {
		return
	}
	if err := validateRetrieve(req); err != nil {
		writeError(w, r, http.StatusUnprocessableEntity, err.Error(), start)
		return
	}
	data, err := s.retrieval.Run(r.Context(), req, effectiveUserID(user))
	if err != nil {
		writeError(w, r, http.StatusInternalServerError, err.Error(), start)
		return
	}
	writeData(w, r, data, start)
}

func (s *Server) searchMemory(w http.ResponseWriter, r *http.Request) {
	start := time.Now()
	user := userFromRequest(r)
	var req SearchRequest
	if !decodeJSON(w, r, &req, start) {
		return
	}
	if err := validateSearch(req); err != nil {
		writeError(w, r, http.StatusUnprocessableEntity, err.Error(), start)
		return
	}
	data, err := s.retrieval.Search(r.Context(), req, effectiveUserID(user))
	if err != nil {
		writeError(w, r, http.StatusInternalServerError, err.Error(), start)
		return
	}
	writeData(w, r, data, start)
}

func (s *Server) authenticate(r *http.Request) (User, int, string) {
	auth := r.Header.Get("Authorization")
	if !strings.HasPrefix(auth, "Bearer ") {
		return User{}, http.StatusUnauthorized, "Missing API key. Provide a Bearer token in the Authorization header."
	}
	token := strings.TrimSpace(strings.TrimPrefix(auth, "Bearer "))
	if token == "" {
		return User{}, http.StatusUnauthorized, "Missing API key. Provide a Bearer token in the Authorization header."
	}
	if !strings.HasPrefix(token, "xmem_") {
		if user, ok := s.validateJWT(token); ok {
			return user, http.StatusOK, ""
		}
	}
	if doc, ok := s.keys.ValidateAPIKey(token); ok {
		if userDoc, exists := s.keys.GetUserByID(doc.UserID); exists {
			return User{ID: userDoc.ID, Name: userDoc.Name, Email: userDoc.Email, Username: userDoc.Username, APIKey: map[string]any{"id": doc.ID, "scopes": doc.Scopes, "org_id": doc.OrgID, "project_id": doc.ProjectID}}, http.StatusOK, ""
		}
	}
	for _, key := range s.settings.APIKeys {
		if database.ConstantTimeEqual(token, key) {
			return User{ID: database.StaticUserID(token), Name: "Static Key User", Email: "static@xmem.ai"}, http.StatusOK, ""
		}
	}
	return User{}, http.StatusForbidden, "Invalid API key or token."
}

func (s *Server) validateJWT(token string) (User, bool) {
	parts := strings.Split(token, ".")
	if len(parts) != 3 || s.settings.JWTAlgorithm != "HS256" {
		return User{}, false
	}
	signingInput := parts[0] + "." + parts[1]
	mac := hmac.New(sha256.New, []byte(s.settings.JWTSecretKey))
	_, _ = mac.Write([]byte(signingInput))
	expected := base64.RawURLEncoding.EncodeToString(mac.Sum(nil))
	if !hmac.Equal([]byte(expected), []byte(parts[2])) {
		return User{}, false
	}
	payloadBytes, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return User{}, false
	}
	var payload map[string]any
	if err := json.Unmarshal(payloadBytes, &payload); err != nil {
		return User{}, false
	}
	if payload["type"] != "access" {
		return User{}, false
	}
	sub, _ := payload["sub"].(string)
	if sub == "" {
		return User{}, false
	}
	if userDoc, exists := s.keys.GetUserByID(sub); exists {
		return User{ID: userDoc.ID, Name: userDoc.Name, Email: userDoc.Email, Username: userDoc.Username}, true
	}
	return User{ID: sub, Name: sub, Email: ""}, true
}

func effectiveUserID(user User) string {
	if user.Username != "" {
		return user.Username
	}
	if user.Name != "" {
		return user.Name
	}
	return user.ID
}

func validateIngest(req IngestRequest) error {
	req.UserQuery = strings.TrimSpace(req.UserQuery)
	if req.UserQuery == "" || len(req.UserQuery) > 10000 {
		return errors.New("user_query must be between 1 and 10000 characters")
	}
	if len(req.AgentResponse) > 10000 {
		return errors.New("agent_response must be at most 10000 characters")
	}
	if !validUserID(req.UserID) {
		return errors.New("user_id is required and may contain only letters, numbers, dots, hyphens, underscores, and @")
	}
	if len(req.ImageURL) > 50000 {
		return errors.New("image_url must be at most 50000 characters")
	}
	if req.EffortLevel != "" && req.EffortLevel != "low" && req.EffortLevel != "high" {
		return errors.New("effort_level must be 'low' or 'high'")
	}
	return nil
}

func validateRetrieve(req RetrieveRequest) error {
	if strings.TrimSpace(req.Query) == "" || len(req.Query) > 5000 {
		return errors.New("query must be between 1 and 5000 characters")
	}
	if !validUserID(req.UserID) {
		return errors.New("user_id is required and may contain only letters, numbers, dots, hyphens, underscores, and @")
	}
	if req.TopK < 0 || req.TopK > 50 {
		return errors.New("top_k must be between 1 and 50")
	}
	return nil
}

func validateSearch(req SearchRequest) error {
	if strings.TrimSpace(req.Query) == "" || len(req.Query) > 5000 {
		return errors.New("query must be between 1 and 5000 characters")
	}
	if !validUserID(req.UserID) {
		return errors.New("user_id is required and may contain only letters, numbers, dots, hyphens, underscores, and @")
	}
	if req.TopK < 0 || req.TopK > 100 {
		return errors.New("top_k must be between 1 and 100")
	}
	allowed := map[string]bool{"profile": true, "temporal": true, "summary": true}
	for _, domain := range req.Domains {
		if !allowed[domain] {
			return fmt.Errorf("invalid domain %q", domain)
		}
	}
	return nil
}

func validUserID(id string) bool {
	if id == "" || len(id) > 256 {
		return false
	}
	return regexp.MustCompile(`^[\w.\-@]+$`).MatchString(id)
}

func decodeJSON(w http.ResponseWriter, r *http.Request, dst any, start time.Time) bool {
	defer r.Body.Close()
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(dst); err != nil {
		writeError(w, r, http.StatusUnprocessableEntity, err.Error(), start)
		return false
	}
	return true
}

func writeData(w http.ResponseWriter, r *http.Request, data any, start time.Time) {
	elapsed := elapsedMS(start)
	w.Header().Set("X-Response-Time-Ms", fmt.Sprintf("%.2f", elapsed))
	writeJSON(w, r, http.StatusOK, APIResponse{Status: "ok", RequestID: requestID(r), Data: data, ElapsedMS: &elapsed})
}

func writeError(w http.ResponseWriter, r *http.Request, code int, msg string, start time.Time) {
	elapsed := elapsedMS(start)
	w.Header().Set("X-Response-Time-Ms", fmt.Sprintf("%.2f", elapsed))
	writeJSON(w, r, code, APIResponse{Status: "error", RequestID: requestID(r), Error: msg, ElapsedMS: &elapsed})
}

func writeJSON(w http.ResponseWriter, r *http.Request, code int, body APIResponse) {
	if body.RequestID == "" {
		body.RequestID = requestID(r)
	}
	if remaining, ok := r.Context().Value(rateRemainingKey{}).(int); ok {
		w.Header().Set("X-RateLimit-Remaining", fmt.Sprint(remaining))
	}
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(body)
}

func elapsedMS(start time.Time) float64 {
	return float64(time.Since(start).Microseconds()) / 1000
}

type requestIDKey struct{}
type rateRemainingKey struct{}
type userKey struct{}

func requestID(r *http.Request) string {
	id, _ := r.Context().Value(requestIDKey{}).(string)
	return id
}

func userFromRequest(r *http.Request) User {
	user, _ := r.Context().Value(userKey{}).(User)
	return user
}

type RateLimiter struct {
	mu       sync.Mutex
	limit    int
	window   time.Duration
	requests map[string][]time.Time
}

func NewRateLimiter(limit int, window time.Duration) *RateLimiter {
	if limit <= 0 {
		limit = 60
	}
	return &RateLimiter{limit: limit, window: window, requests: map[string][]time.Time{}}
}

func (l *RateLimiter) Check(key string) (bool, int) {
	l.mu.Lock()
	defer l.mu.Unlock()
	now := time.Now()
	cutoff := now.Add(-l.window)
	hits := l.requests[key]
	kept := hits[:0]
	for _, hit := range hits {
		if hit.After(cutoff) {
			kept = append(kept, hit)
		}
	}
	if len(kept) >= l.limit {
		l.requests[key] = kept
		return false, 0
	}
	kept = append(kept, now)
	l.requests[key] = kept
	return true, l.limit - len(kept)
}
