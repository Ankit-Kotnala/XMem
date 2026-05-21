package agents

import (
	"context"
	"regexp"
	"strings"
	"time"

	"github.com/xortexai/xmem-go/internal/weaver"
)

type Classification struct {
	Source string `json:"source"`
	Query  string `json:"query"`
}

type ClassifierAgent struct{}

func (a ClassifierAgent) Run(_ context.Context, userQuery string, imageURL string) []Classification {
	lowered := strings.ToLower(userQuery)
	out := []Classification{}
	if containsAny(lowered, "i am", "i'm", "my ", "i like", "i love", "i work", "name is", "prefer") {
		out = append(out, Classification{Source: "profile", Query: userQuery})
	}
	if containsAny(lowered, "today", "tomorrow", "yesterday", "appointment", "meeting", "birthday", "on ", " at ") || dateLike(userQuery) {
		out = append(out, Classification{Source: "event", Query: userQuery})
	}
	if containsAny(lowered, "code", "function", "script", "snippet", "class ", "package ") {
		out = append(out, Classification{Source: "code", Query: userQuery})
	}
	if strings.TrimSpace(imageURL) != "" {
		out = append(out, Classification{Source: "image", Query: "Analyze this image for memory-relevant details."})
	}
	return out
}

type ProfileFact struct {
	Topic    string
	SubTopic string
	Memo     string
}

type ProfilerAgent struct{}

func (a ProfilerAgent) Run(_ context.Context, text string) []ProfileFact {
	lowered := strings.ToLower(text)
	facts := []ProfileFact{}
	if match := regexp.MustCompile(`(?i)(?:i work at|joined|company is|at)\s+([A-Z][A-Za-z0-9_.-]+)`).FindStringSubmatch(text); len(match) == 2 {
		facts = append(facts, ProfileFact{Topic: "work", SubTopic: "company", Memo: match[1]})
	}
	if match := regexp.MustCompile(`(?i)(?:my name is|i am|i'm)\s+([A-Z][A-Za-z]+)`).FindStringSubmatch(text); len(match) == 2 {
		facts = append(facts, ProfileFact{Topic: "basic_info", SubTopic: "name", Memo: match[1]})
	}
	if containsAny(lowered, "pizza", "vegetarian", "coffee", "tea") {
		facts = append(facts, ProfileFact{Topic: "food", SubTopic: "preference", Memo: sentence(text)})
	}
	if containsAny(lowered, "like", "love", "enjoy", "hobby") {
		facts = append(facts, ProfileFact{Topic: "interest", SubTopic: "general", Memo: sentence(text)})
	}
	return facts
}

type Event struct {
	Date           string
	EventName      string
	Desc           string
	Year           string
	Time           string
	DateExpression string
}

type TemporalAgent struct{}

func (a TemporalAgent) Run(_ context.Context, text string, sessionDatetime string) []Event {
	lowered := strings.ToLower(text)
	if !containsAny(lowered, "today", "tomorrow", "yesterday", "appointment", "meeting", "birthday", "demo", "launch") && !dateLike(text) {
		return nil
	}
	date, year := inferDate(lowered, sessionDatetime)
	name := inferEventName(lowered)
	return []Event{{Date: date, EventName: name, Desc: sentence(text), Year: year, Time: inferTime(text), DateExpression: inferDateExpression(lowered)}}
}

type SummarizerAgent struct{}

func (a SummarizerAgent) Run(_ context.Context, userQuery string, agentResponse string) []string {
	if len(strings.Fields(userQuery)) < 4 {
		return nil
	}
	summary := strings.TrimSpace(userQuery)
	if agentResponse != "" && agentResponse != "Acknowledged." {
		summary += " Assistant responded: " + strings.TrimSpace(agentResponse)
	}
	return []string{summary}
}

type ImageAgent struct{}

func (a ImageAgent) Run(_ context.Context, imageURL string) []string {
	if strings.TrimSpace(imageURL) == "" {
		return nil
	}
	return []string{"[Image] User attached an image: " + imageURL}
}

type SnippetAgent struct{}

func (a SnippetAgent) Run(_ context.Context, text string) []string {
	if !containsAny(strings.ToLower(text), "code", "function", "script", "snippet") {
		return nil
	}
	return []string{sentence(text) + " |  | unknown | algorithm | chat"}
}

type JudgeAgent struct{}

func (a JudgeAgent) JudgeItems(_ context.Context, domain weaver.JudgeDomain, items []string, confidence float64) weaver.JudgeResult {
	ops := make([]weaver.Operation, 0, len(items))
	for _, item := range items {
		item = strings.TrimSpace(item)
		if item == "" {
			continue
		}
		ops = append(ops, weaver.Operation{Type: weaver.OperationAdd, Content: item, Reason: "New memory item extracted."})
	}
	if confidence == 0 {
		confidence = 0.8
	}
	return weaver.JudgeResult{Operations: ops, Confidence: confidence}
}

func (a JudgeAgent) JudgeProfile(ctx context.Context, facts []ProfileFact) weaver.JudgeResult {
	items := make([]string, 0, len(facts))
	for _, fact := range facts {
		items = append(items, fact.Topic+" / "+fact.SubTopic+" = "+fact.Memo)
	}
	return a.JudgeItems(ctx, weaver.DomainProfile, items, 1.0)
}

func (a JudgeAgent) JudgeTemporal(ctx context.Context, events []Event) weaver.JudgeResult {
	items := make([]string, 0, len(events))
	for _, event := range events {
		items = append(items, strings.Join([]string{event.Date, event.EventName, event.Desc, event.Year, event.Time, event.DateExpression}, " | "))
	}
	return a.JudgeItems(ctx, weaver.DomainTemporal, items, 1.0)
}

func containsAny(text string, needles ...string) bool {
	for _, needle := range needles {
		if strings.Contains(text, needle) {
			return true
		}
	}
	return false
}

func sentence(text string) string {
	text = strings.Join(strings.Fields(text), " ")
	if len(text) > 1000 {
		return text[:1000]
	}
	return text
}

func dateLike(text string) bool {
	return regexp.MustCompile(`\b\d{1,2}[-/]\d{1,2}\b`).MatchString(text)
}

func inferDate(lowered, session string) (string, string) {
	base := time.Now()
	if session != "" {
		if parsed, err := time.Parse(time.RFC3339, session); err == nil {
			base = parsed
		}
	}
	if strings.Contains(lowered, "tomorrow") {
		base = base.Add(24 * time.Hour)
	} else if strings.Contains(lowered, "yesterday") {
		base = base.Add(-24 * time.Hour)
	}
	if match := regexp.MustCompile(`\b(\d{1,2})[-/](\d{1,2})\b`).FindStringSubmatch(lowered); len(match) == 3 {
		return match[1] + "-" + match[2], ""
	}
	return base.Format("01-02"), base.Format("2006")
}

func inferEventName(lowered string) string {
	for _, candidate := range []string{"appointment", "meeting", "birthday", "demo", "launch"} {
		if strings.Contains(lowered, candidate) {
			return strings.Title(candidate)
		}
	}
	return "Event"
}

func inferDateExpression(lowered string) string {
	for _, candidate := range []string{"today", "tomorrow", "yesterday"} {
		if strings.Contains(lowered, candidate) {
			return candidate
		}
	}
	return ""
}

func inferTime(text string) string {
	if match := regexp.MustCompile(`\b\d{1,2}:\d{2}\b`).FindString(text); match != "" {
		return match
	}
	return ""
}
