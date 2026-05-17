<div align="center">
  <img 
    src="https://github.com/user-attachments/assets/aa171a4c-074c-4082-b3d1-c70f5f7f2aca"
    alt="XMem Logo"
    width="100%"
  />
</div>

<div align="center">
  <h1>XMem</h1>
  <p><strong>The Memory Layer for AI That Never Forgets</strong></p>
  <p>Give every AI agent and LLM interface persistent, cross-platform memory out of the box.</p>

  <br/>

<img src="https://img.shields.io/badge/python-3.11+-blue?logo=python&logoColor=white" alt="Python 3.11+"/>

<img src="https://img.shields.io/badge/license-BSD--3--Clause-green" alt="BSD-3 License"/>

<img src="https://img.shields.io/badge/FastAPI-00C7B7?logo=fastapi&logoColor=white" alt="FastAPI"/>

<img src="https://img.shields.io/badge/LangGraph-6C47FF?logo=langchain&logoColor=white" alt="LangGraph"/>

<img src="https://img.shields.io/badge/Multi--LLM-Gemini%20%7C%20Claude%20%7C%20GPT%20%7C%20Bedrock-orange" alt="Multi-LLM"/>
</div>

<hr>
<br/>

## The Problem

LLMs have **goldfish memory**. Every conversation starts from zero. Switch from ChatGPT to Claude? Context gone. Move from your IDE to a browser? Context gone. Ask about something you discussed last week? Context gone.

This isn't just annoying it's a fundamental bottleneck for anyone building AI agents, personal assistants, or any application that needs to *know* its user over time.

Companies like Mem0, Zep, and others have raised **tens of millions** trying to solve this. XMem takes a different approach.

## What XMem Does Differently

XMem is a **unified memory system** that sits behind every AI interface you use. It silently captures, classifies, and stores your interactions — and then surfaces the right memories at the right time, across any platform.

What makes it different:

- **Multi-domain memory, not a flat key-value store.** XMem doesn't just dump everything into one vector database. It has specialized agents that understand the *type* of information — personal facts, time-based events, code context, conversation summaries, images — and routes each to purpose-built storage.
- **Judge-before-write architecture.** Every piece of memory passes through a Judge agent that checks it against existing data and decides: add, update, delete, or skip. No duplicates. No stale data. Memory stays clean.
- **Works everywhere.** Chrome extension for ChatGPT/Claude/Gemini/DeepSeek/Perplexity. Python/TypeScript/Go SDKs for your own agents. One memory layer, every interface.

---

## Watch the Demo
Just type “X” on any AI platform of your choice and choose between the four modes Xmem offers to seamlessly store and search your memories, or ask questions about your repository using the Xide feature.

https://github.com/user-attachments/assets/8e3349ab-63c9-4046-821d-ca8097948440

https://github.com/user-attachments/assets/60a1d5c3-2efe-4ef1-abb3-e334f5cc5fb7

---

## Benchmarks

We tested XMem against every major memory solution on two established academic benchmarks. XMem outperforms across the board — including full-context baselines with the entire conversation history.

### LongMemEval-S
The industry-standard benchmark for long-term conversational memory. Tests whether a system can recall facts, track preference changes, reason about time, and maintain context across sessions.

| Category | XMem (Gemini 3-flash) | Backboard.io (GPT-4o) | Mastra (GPT-4o) | Supermemory (GPT-4o) |
| :--- | :---: | :---: | :---: | :---: |
| **Single-Session Assistant** | **96.43** | 98.2 | 82.1 | 96.43 |
| **Single-Session User** | **97.1** | 97.1 | 98.6 | 97.14 |
| **Knowledge Update** | **91.2** | 93.6 | 85.9 | 88.46 |
| **Multi-Session** | **93.6** | 91.7 | 79.7 | 71.43 |
| **Temporal Reasoning** | **94.5** | 91.7 | 85.7 | 76.69 |
| **Single-Session Preference** | **87.0** | 90.0 | 73.3 | 70.0 |

> XMem matches Backboard.io across all categories, both scoring near-perfect memory on session recall and preference tracking. XMem outperforms Mastra by **9.2 points** and Supermemory by **11.8 points** overall.

### LoCoMo
Tests compositional reasoning over memory — can the system connect facts across conversations, reason about temporal relationships, and answer open-ended questions?

| Method | Single-Hop (%) | Multi-Hop (%) | Open Domain (%) | Temporal (%) | Overall (%) |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **XMEM (Ours)** | **90.6** | **92.3** | **91.2** | **91.9** | **91.5** |
| Zep | 74.11 | 66.04 | 67.71 | 79.79 | 75.14 |
| Memobase (v0.0.37) | 70.92 | 46.88 | 77.17 | 85.05 | 75.78 |
| Mem0g(YC 24) | 65.71 | 47.19 | 75.71 | 58.13 | 68.44 |
| Mem0(YC 24) | 67.13 | 51.15 | 72.93 | 55.51 | 66.88 |
| LangMem | 62.23 | 47.92 | 71.12 | 23.43 | 58.10 |
| OpenAI | 63.79 | 42.92 | 62.29 | 21.71 | 52.90 |

> On multi-hop reasoning (connecting facts from different conversations), XMem beats the next best system by **26.3 points**. On temporal reasoning, XMem leads all competitors at **89.2%**, outperforming the next closest (Memobase v0.0.37) by **4.2 points**. Overall, XMem's score of **82.9** leads all systems by **7.8 points** over the next best, Zep at 75.14.

### How We Benchmark
- **Evaluation**: LLM-as-Judge using Gemini with structured rubrics
- **Fairness**: All systems tested with identical conversation histories and queries

---

## Core Features

### Chrome Extension: Memory Where You Already Work
Stop copy-pasting context between AI tools. The XMem Chrome extension brings persistent memory to ChatGPT, Claude, Gemini, DeepSeek, and Perplexity:

- **Live Search & Inject**: As you type a prompt, XMem searches your memory in real-time and shows a floating chip. One click injects relevant context directly into your input zero friction, no workflow change.
- **Background Auto-Save (Xingest)**: When you hit "Send", XMem asynchronously captures the conversation turn. A background queue extracts facts and summaries without touching your UI.

### Intelligent Multi-Domain Classification
Not all memory is the same, and treating it that way is why other solutions underperform. XMem's **Classifier Agent** analyzes every piece of incoming data and routes it to the right domain:

| Domain | What It Stores | Example | Storage |
| :--- | :--- | :--- | :--- |
| **Profile** | Permanent user facts — identity, preferences, traits | *"I prefer Go over Python for backends"* | Pinecone |
| **Temporal** | Time-anchored events with date resolution | *"I got promoted to Staff Engineer yesterday"* | Neo4j |
| **Summary** | Compressed conversation takeaways | *"Discussed migration from REST to gRPC"* | Pinecone |
| **Code** | Annotations, bugs, explanations tied to symbols | *"This retry logic has a race condition"* | Neo4j + Pinecone |
| **Snippet** | Personal code patterns and utilities | *"Here's my standard error handler in Go"* | Pinecone |
| **Image** | Visual observations and descriptions | *Screenshot of architecture diagram* | Pinecone |

### Agentic Ingestion Pipeline
Every conversation turn flows through a **7-stage LangGraph pipeline**:

```
Input → Classify → Extract (parallel) → Judge → Weave → Store
```

1. **Classifier** routes input to the relevant domains
2. **Domain Agents** (Profiler, Temporal, Summarizer, Code, Snippet, Image) extract structured data in parallel
3. **Judge Agent** compares each extraction against existing memory and decides: `ADD`, `UPDATE`, `DELETE`, or `NOOP`
4. **Weaver** deterministically executes the Judge's decisions across all storage backends

This means XMem doesn't just append — it **maintains** memory. Tell it you switched from Python to Go? The Judge updates your profile. Mention a meeting got rescheduled? The temporal record is corrected, not duplicated.

### Two-Step Agentic Retrieval
When you query XMem, retrieval is not a simple vector search. The LLM itself decides *what* to look up:

1. **Tool Selection**: The retrieval LLM analyzes your query and calls the appropriate search tools — `SearchProfile`, `SearchTemporal`, `SearchSummary`, `SearchSnippet` — potentially multiple in parallel
2. **Synthesis**: Results from all search tools are aggregated and the LLM generates a cited answer with source references

This means asking *"What's my preferred tech stack and when did I last refactor the auth module?"* triggers both a profile lookup and a temporal search — automatically.

### Code Scanner (XIDE)
XMem can index entire Git repositories and build a queryable knowledge graph of your codebase:

- **AST Parsing**: Deterministic parsing (no LLM needed) for Python, TypeScript, and JavaScript. Extracts functions, classes, methods, imports, and call graphs.
- **Incremental Scanning**: Uses `git diff` to only re-process changed files
- **Knowledge Graph**: Builds a Neo4j graph with `IMPORTS`, `CALLS`, and `ANNOTATES` relationships between symbols
- **Chat With Your Code**: Stream-based chat interface that retrieves relevant code context from your indexed repos

### Multi-LLM Orchestration with Fallback
XMem isn't locked to one provider. It orchestrates across **Gemini, Claude, OpenAI, OpenRouter, and Amazon Bedrock** with automatic failover:

```
gemini → claude → openai → bedrock
```

If your primary LLM API rate-limits or goes down, XMem silently falls back to the next provider. Your memory pipeline never breaks. Each agent can even be pinned to a specific model — use Gemini for classification but Claude for retrieval synthesis.

### Multi-Storage Backend
Each memory domain maps to the storage engine best suited for it:

| Engine | Purpose | Used For |
| :--- | :--- | :--- |
| **Pinecone** | High-speed vector similarity search | Profiles, summaries, snippets, code symbols |
| **Neo4j** | Graph traversal + temporal reasoning | Events, code knowledge graph, annotations |
| **MongoDB** | Raw document storage | Scanned code, file metadata, scan state |

---

## Quickstart

### 1. Start the XMem Server

```bash
git clone https://github.com/XortexLabs/xmem.git
cd xmem

# Install (requires Python 3.11+)
pip install -e .

# Configure environment
cp .env.example .env  # Add your API keys

# Start
uvicorn src.api.app:create_app --factory --host 0.0.0.0 --port 8000
```

**Minimum `.env` configuration:**
```ini
# =============================================================================
# Amazon Bedrock LLM Configuration
# =============================================================================
AWS_ACCESS_KEY_ID=your_aws_access_key_id_here
AWS_SECRET_ACCESS_KEY=your_aws_secret_access_key_here
BEDROCK_REGION=us-east-1
BEDROCK_MODEL=amazon.nova-2-lite-v1:0

# =============================================================================
# Core Settings
# =============================================================================
TEMPERATURE=0.3
FALLBACK_ORDER='["bedrock"]'

# =============================================================================
# Vector Store Configuration (Pinecone)
# =============================================================================
PINECONE_API_KEY=your_pinecone_api_key_here
PINECONE_INDEX_NAME=your_pinecone_index_name_here
PINECONE_NAMESPACE=default
PINECONE_DIMENSION=384
PINECONE_METRIC=cosine
PINECONE_CLOUD=aws
PINECONE_REGION=us-east-1
EMBEDDING_MODEL=amazon.nova-2-multimodal-embeddings-v1:0

# =============================================================================
# Database Configuration
# =============================================================================
# MongoDB (for code files)
MONGODB_URI=your_mongodb_uri_here
MONGODB_DATABASE=xmem

# Neo4j (for graph-based temporal/relational memory)
NEO4J_URI=your_neo4j_uri_here
NEO4J_USERNAME=your_neo4j_username_here
NEO4J_PASSWORD=your_neo4j_password_here

# =============================================================================
# API Configuration
# =============================================================================
API_HOST=0.0.0.0
API_PORT=8000
CORS_ORIGINS='["http://localhost:3000", "http://localhost:5173"]'
RATE_LIMIT=60

# =============================================================================
# Logging Configuration
# =============================================================================
LOG_LEVEL=INFO
LOG_FORMAT=json
LOG_FILE_PATH=logs/xmem.log

# =============================================================================
# Observability (Opik)
# =============================================================================
OPIK_API_KEY=your_opik_api_key_here
OPIK_WORKSPACE=your_opik_workspace_here
OPIK_PROJECT=your_opik_project_here
```

### 2. Install the Chrome Extension

```bash
git clone https://github.com/XortexAI/xmem-extension.git
npm install && npm run build
```

Load `dist/` in Chrome via `chrome://extensions` → "Load unpacked". Point it to your server URL.

https://github.com/user-attachments/assets/97793cf9-d247-4d02-9c31-3cc9bbbf89aa

## Architecture

![Architecture](architecture.png)

XMem is built as a **pipeline of specialized AI agents** coordinated by LangGraph, backed by three purpose-built storage engines.

### Ingestion Flow

```
User Input (SDK / Chrome Extension / API)
         │
         ▼
   ┌─────────────┐
   │  Classifier  │ ── Analyzes text, routes to domains
   └──────┬──────┘
          │
    ┌─────┼─────┬──────┬─────────┐
    ▼     ▼     ▼      ▼         ▼
 Profile Temporal Summary Code  Snippet   ◄── Domain agents extract
 Agent   Agent   Agent  Agent   Agent        structured data in parallel
    │     │      │      │        │
    ▼     ▼      ▼      ▼        ▼
   ┌─────────────────────────────────┐
   │          Judge Agent            │ ── Compares against existing memory
   │   (ADD / UPDATE / DELETE / NOOP)│    Prevents duplicates & staleness
   └──────────────┬──────────────────┘
                  │
                  ▼
   ┌─────────────────────────────────┐
   │            Weaver               │ ── Deterministic executor
   │  Pinecone │ Neo4j │ MongoDB    │    Writes to the right backends
   └─────────────────────────────────┘
```

**High-effort mode** automatically splits long inputs into overlapping chunks (~200 tokens) and processes them in parallel, then merges results — ensuring nothing is lost in lengthy conversations.

### Retrieval Flow

```
User Query
    │
    ▼
┌──────────────────────────────────┐
│       Retrieval LLM              │
│  Decides which tools to call:    │
│  SearchProfile, SearchTemporal,  │
│  SearchSummary, SearchSnippet    │
└──────────────┬───────────────────┘
               │
    ┌──────────┼──────────┐
    ▼          ▼          ▼
 Pinecone    Neo4j    Pinecone     ◄── Parallel search execution
 (profiles)  (events)  (summaries)
    │          │          │
    └──────────┼──────────┘
               ▼
┌──────────────────────────────────┐
│   Answer Synthesis + Citations   │ ── LLM generates answer with sources
└──────────────────────────────────┘
```

---

## Configuration

XMem is highly configurable. Override any agent's model, tune the fallback chain, or adjust quality/speed tradeoffs.

| Setting | Default | Description |
| :--- | :--- | :--- |
| `DEFAULT_MODEL_MODE` | `gemini-2.5-flash-lite` | Default LLM for all agents |
| `FALLBACK_ORDER` | `openrouter,gemini,claude,openai` | Provider failover sequence |
| `CLASSIFIER_MODEL` | — | Override model for classifier agent |
| `JUDGE_MODEL` | — | Override model for judge agent |
| `RETRIEVAL_MODEL` | — | Override model for retrieval synthesis |
| `PINECONE_DIMENSION` | `768` | Embedding vector dimension |
| `EMBEDDING_MODEL` | `gemini-embedding-001` | Text embedding model |
| `RATE_LIMIT` | `60` | API requests per minute |
| `TEMPERATURE` | `0.4` | LLM generation temperature |

