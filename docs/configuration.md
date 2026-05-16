# Configuration

## Vector Store Provider

Cloud Pinecone remains the default:

```env
VECTOR_STORE_PROVIDER=pinecone
PINECONE_API_KEY=...
PINECONE_INDEX_NAME=xmem-index
PINECONE_NAMESPACE=default
PINECONE_DIMENSION=384
```

For local testing, switch only the provider-specific settings:

```env
VECTOR_STORE_PROVIDER=pgvector
PGVECTOR_URL=postgresql://xmem:xmem@localhost:5432/xmem
PGVECTOR_TABLE=xmem_vectors
```

```env
VECTOR_STORE_PROVIDER=chroma
CHROMA_PERSIST_DIR=.xmem/chroma
```

```env
VECTOR_STORE_PROVIDER=sqlite
SQLITE_VECTOR_PATH=.xmem/xmem_vectors.db
```

Neo4j is configured independently and is used for temporal memory plus the
scanner v1 code graph:

```env
NEO4J_URI=bolt://localhost:7687
NEO4J_USERNAME=neo4j
NEO4J_PASSWORD=local-password
```

Run the local database stack with:

```bash
docker compose -f docker/docker-compose.dev.yml up -d
```

Install local vector backends with:

```bash
pip install -e ".[local]"
```
