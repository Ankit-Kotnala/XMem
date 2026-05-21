package main

import (
	"context"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/xortexai/xmem-go/internal/agents"
	"github.com/xortexai/xmem-go/internal/api"
	"github.com/xortexai/xmem-go/internal/config"
	"github.com/xortexai/xmem-go/internal/database"
	"github.com/xortexai/xmem-go/internal/graph"
	"github.com/xortexai/xmem-go/internal/models"
	"github.com/xortexai/xmem-go/internal/pipelines"
	"github.com/xortexai/xmem-go/internal/storage"
	"github.com/xortexai/xmem-go/internal/weaver"
)

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))
	settings, err := config.Load()
	if err != nil {
		logger.Error("configuration failed", "error", err)
		os.Exit(1)
	}

	var vectorStore storage.VectorStore = storage.NewMemoryVectorStore()
	var snippetStore storage.VectorStore = storage.NewMemoryVectorStore()
	var temporalStore graph.TemporalStore = graph.NewMemoryTemporalStore()
	var embedder storage.Embedder = storage.HashEmbedder{Dimension: settings.PineconeDimension}
	if settings.EmbeddingProvider == "openai" {
		if openAIEmbedder, err := storage.NewOpenAIEmbedder(settings); err == nil {
			embedder = openAIEmbedder
			logger.Info("using OpenAI embedder", "model", settings.OpenAIEmbeddingModel, "dimension", settings.PineconeDimension)
		} else if settings.Environment == "production" {
			logger.Error("openai embedder initialization failed", "error", err)
			os.Exit(1)
		} else {
			logger.Warn("openai embedder unavailable, using hash embedder", "error", err)
		}
	}
	model := models.NewRegistry(settings)

	if settings.VectorStoreProvider == "pinecone" {
		if pineconeStore, err := storage.NewPineconeVectorStore(context.Background(), settings, embedder, settings.PineconeNamespace); err == nil {
			vectorStore = pineconeStore
			logger.Info("using Pinecone vector store", "namespace", settings.PineconeNamespace)
		} else if settings.Environment == "production" {
			logger.Error("pinecone initialization failed", "error", err)
			os.Exit(1)
		} else {
			logger.Warn("pinecone unavailable, using memory vector store", "error", err)
		}
		if pineconeSnippets, err := storage.NewPineconeVectorStore(context.Background(), settings, embedder, settings.PineconeNamespace+"-snippets"); err == nil {
			snippetStore = pineconeSnippets
			logger.Info("using Pinecone snippet vector store", "namespace", settings.PineconeNamespace+"-snippets")
		}
	}

	if settings.Neo4jPassword != "" {
		if neoStore, err := graph.NewNeo4jTemporalStore(context.Background(), settings); err == nil {
			temporalStore = neoStore
			logger.Info("using Neo4j temporal store")
		} else if settings.Environment == "production" {
			logger.Error("neo4j initialization failed", "error", err)
			os.Exit(1)
		} else {
			logger.Warn("neo4j unavailable, using memory temporal store", "error", err)
		}
	}

	w := &weaver.Weaver{
		VectorStore:        vectorStore,
		SnippetVectorStore: snippetStore,
		Embedder:           embedder,
		TemporalStore:      temporalStore,
	}
	ingest := &pipelines.IngestPipeline{
		ModelName:  model.Name(),
		Weaver:     w,
		Classifier: agents.ClassifierAgent{},
		Profiler:   agents.ProfilerAgent{},
		Temporal:   agents.TemporalAgent{},
		Summarizer: agents.SummarizerAgent{},
		Image:      agents.ImageAgent{},
		Snippet:    agents.SnippetAgent{},
		Judge:      agents.JudgeAgent{},
	}
	retrieval := &pipelines.RetrievalPipeline{
		Model:         model,
		VectorStore:   vectorStore,
		SnippetStore:  snippetStore,
		TemporalStore: temporalStore,
	}

	var keyStore database.APIKeyStore = database.NewMemoryAPIKeyStore()
	if settings.AppStoreProvider == "mongo" {
		if mongoStore, err := database.NewMongoAPIKeyStore(context.Background(), settings); err == nil {
			keyStore = mongoStore
			logger.Info("using MongoDB API key store")
		} else if settings.Environment == "production" {
			logger.Error("mongodb initialization failed", "error", err)
			os.Exit(1)
		} else {
			logger.Warn("mongodb unavailable, using memory API key store", "error", err)
		}
	}
	server := api.NewServer(settings, logger, ingest, retrieval, keyStore)
	httpServer := &http.Server{
		Addr:              settings.Addr(),
		Handler:           server.Handler(),
		ReadHeaderTimeout: 10 * time.Second,
	}

	go func() {
		logger.Info("xmem-go listening", "addr", settings.Addr(), "service", settings.ServiceName)
		if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Error("server failed", "error", err)
			os.Exit(1)
		}
	}()

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGINT, syscall.SIGTERM)
	<-stop

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := httpServer.Shutdown(ctx); err != nil {
		logger.Error("graceful shutdown failed", "error", err)
		os.Exit(1)
	}
	logger.Info("xmem-go stopped")
}
