use std::collections::BTreeMap;

use thiserror::Error;

use crate::model::{
    ExecutedOp, JudgeDomain, JudgeResult, OpStatus, Operation, OperationType, WeaverResult,
};
use crate::parser::{
    extract_structured_metadata, parse_code_annotation_content, parse_snippet_content,
    parse_temporal_content,
};
use crate::storage::{
    EmbeddingProvider, GraphAnnotationStore, Metadata, TemporalGraphStore, VectorDocument,
    VectorStore,
};

#[derive(Debug, Error)]
pub enum WeaverError {
    #[error("{0}")]
    Message(String),
}

impl From<&str> for WeaverError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<String> for WeaverError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

pub struct Weaver<'a> {
    pub vector_store: Option<&'a mut dyn VectorStore>,
    pub code_vector_store: Option<&'a mut dyn VectorStore>,
    pub snippet_vector_store: Option<&'a mut dyn VectorStore>,
    pub temporal_graph_store: Option<&'a mut dyn TemporalGraphStore>,
    pub annotation_store: Option<&'a mut dyn GraphAnnotationStore>,
    pub embedding_provider: Option<&'a dyn EmbeddingProvider>,
}

impl<'a> Weaver<'a> {
    pub fn new() -> Self {
        Self {
            vector_store: None,
            code_vector_store: None,
            snippet_vector_store: None,
            temporal_graph_store: None,
            annotation_store: None,
            embedding_provider: None,
        }
    }

    pub fn with_vector_store(mut self, store: &'a mut dyn VectorStore) -> Self {
        self.vector_store = Some(store);
        self
    }

    pub fn with_code_vector_store(mut self, store: &'a mut dyn VectorStore) -> Self {
        self.code_vector_store = Some(store);
        self
    }

    pub fn with_snippet_vector_store(mut self, store: &'a mut dyn VectorStore) -> Self {
        self.snippet_vector_store = Some(store);
        self
    }

    pub fn with_temporal_graph_store(mut self, store: &'a mut dyn TemporalGraphStore) -> Self {
        self.temporal_graph_store = Some(store);
        self
    }

    pub fn with_annotation_store(mut self, store: &'a mut dyn GraphAnnotationStore) -> Self {
        self.annotation_store = Some(store);
        self
    }

    pub fn with_embedding_provider(mut self, provider: &'a dyn EmbeddingProvider) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    pub fn execute(
        &mut self,
        judge_result: &JudgeResult,
        domain: JudgeDomain,
        user_id: &str,
    ) -> WeaverResult {
        let mut result = WeaverResult::default();

        if judge_result.is_empty() || !judge_result.has_writes() {
            return result;
        }

        if domain.is_batched_vector_domain() {
            result
                .executed
                .extend(self.execute_batched_vector(&judge_result.operations, domain, user_id));
            return result;
        }

        for op in &judge_result.operations {
            result.executed.push(self.execute_one(op.clone(), domain, user_id));
        }

        result
    }

    fn execute_batched_vector(
        &mut self,
        operations: &[Operation],
        domain: JudgeDomain,
        user_id: &str,
    ) -> Vec<ExecutedOp> {
        let mut executed = Vec::new();
        let mut add_batch = Vec::<Operation>::new();
        let mut delete_batch = Vec::<Operation>::new();

        for op in operations {
            let current = normalize_missing_id(op.clone());

            match current.operation_type {
                OperationType::Noop => {
                    self.flush_add_batch(&mut add_batch, &mut executed, domain, user_id);
                    self.flush_delete_batch(&mut delete_batch, &mut executed);
                    executed.push(ExecutedOp::skipped(&current, None));
                }
                OperationType::Add => {
                    self.flush_delete_batch(&mut delete_batch, &mut executed);
                    add_batch.push(current);
                }
                OperationType::Delete => {
                    self.flush_add_batch(&mut add_batch, &mut executed, domain, user_id);
                    delete_batch.push(current);
                }
                OperationType::Update => {
                    self.flush_add_batch(&mut add_batch, &mut executed, domain, user_id);
                    self.flush_delete_batch(&mut delete_batch, &mut executed);
                    executed.push(self.execute_one(current, domain, user_id));
                }
            }
        }

        self.flush_add_batch(&mut add_batch, &mut executed, domain, user_id);
        self.flush_delete_batch(&mut delete_batch, &mut executed);

        executed
    }

    fn flush_add_batch(
        &mut self,
        add_batch: &mut Vec<Operation>,
        executed: &mut Vec<ExecutedOp>,
        domain: JudgeDomain,
        user_id: &str,
    ) {
        if add_batch.is_empty() {
            return;
        }

        let Some(embedding_provider) = self.embedding_provider else {
            for op in add_batch.drain(..) {
                executed.push(ExecutedOp::failed(&op, "No embedding provider attached"));
            }
            return;
        };

        let Some(store) = self.vector_store.as_deref_mut() else {
            for op in add_batch.drain(..) {
                executed.push(ExecutedOp::failed(&op, "No vector store attached"));
            }
            return;
        };

        let mut ops = Vec::new();
        let mut documents = Vec::new();

        for op in add_batch.drain(..) {
            if op.content.is_empty() {
                executed.push(ExecutedOp::skipped(&op, Some("ADD requires content".to_string())));
                continue;
            }

            match embedding_provider.embed(&op.content) {
                Ok(embedding) => {
                    let metadata = vector_metadata(domain, user_id, &op.content);
                    documents.push(VectorDocument {
                        id: None,
                        text: op.content.clone(),
                        embedding,
                        metadata,
                    });
                    ops.push(op);
                }
                Err(err) => executed.push(ExecutedOp::failed(&op, err.to_string())),
            }
        }

        if documents.is_empty() {
            return;
        }

        match store.add(documents) {
            Ok(ids) => {
                for (op, new_id) in ops.into_iter().zip(ids.into_iter().map(Some)) {
                    executed.push(ExecutedOp::success_with_new_id(&op, new_id));
                }
            }
            Err(err) => {
                for op in ops {
                    executed.push(ExecutedOp::failed(&op, err.to_string()));
                }
            }
        }
    }

    fn flush_delete_batch(
        &mut self,
        delete_batch: &mut Vec<Operation>,
        executed: &mut Vec<ExecutedOp>,
    ) {
        if delete_batch.is_empty() {
            return;
        }

        let Some(store) = self.vector_store.as_deref_mut() else {
            for op in delete_batch.drain(..) {
                executed.push(ExecutedOp::failed(&op, "No vector store attached"));
            }
            return;
        };

        let mut valid_ops = Vec::new();
        let mut ids = Vec::new();

        for op in delete_batch.drain(..) {
            if let Some(id) = op.embedding_id.clone() {
                ids.push(id);
                valid_ops.push(op);
            } else {
                executed.push(ExecutedOp::failed(&op, "DELETE missing embedding_id"));
            }
        }

        if ids.is_empty() {
            return;
        }

        match store.delete(&ids) {
            Ok(true) => {
                for op in valid_ops {
                    executed.push(ExecutedOp::success(&op));
                }
            }
            Ok(false) => {
                for op in valid_ops {
                    executed.push(ExecutedOp {
                        status: OpStatus::Failed,
                        ..ExecutedOp::success(&op)
                    });
                }
            }
            Err(err) => {
                for op in valid_ops {
                    executed.push(ExecutedOp::failed(&op, err.to_string()));
                }
            }
        }
    }

    fn execute_one(&mut self, op: Operation, domain: JudgeDomain, user_id: &str) -> ExecutedOp {
        if op.operation_type == OperationType::Noop {
            return ExecutedOp::skipped(&op, None);
        }

        if op.operation_type == OperationType::Add && op.content.is_empty() {
            return ExecutedOp::skipped(&op, Some("ADD requires content".to_string()));
        }

        let op = normalize_missing_id(op);

        match domain {
            JudgeDomain::Temporal => self.execute_temporal(op, user_id),
            JudgeDomain::Code => self.execute_code(op, user_id),
            JudgeDomain::Snippet => self.execute_snippet(op, user_id),
            _ => self.execute_vector(op, domain, user_id),
        }
    }

    fn execute_vector(&mut self, op: Operation, domain: JudgeDomain, user_id: &str) -> ExecutedOp {
        let Some(embedding_provider) = self.embedding_provider else {
            return ExecutedOp::failed(&op, "No embedding provider attached");
        };

        let Some(store) = self.vector_store.as_deref_mut() else {
            return ExecutedOp::failed(&op, "No vector store attached");
        };

        match op.operation_type {
            OperationType::Add => vector_add(store, embedding_provider, &op, domain, user_id),
            OperationType::Update => vector_update(store, embedding_provider, &op, domain, user_id),
            OperationType::Delete => vector_delete(store, &op),
            OperationType::Noop => ExecutedOp::skipped(&op, None),
        }
    }

    fn execute_temporal(&mut self, op: Operation, user_id: &str) -> ExecutedOp {
        let Some(store) = self.temporal_graph_store.as_deref_mut() else {
            return ExecutedOp::failed(&op, "No temporal graph store attached");
        };

        match op.operation_type {
            OperationType::Add => {
                let mut event_data = parse_temporal_content(&op.content);
                let Some(date_str) = event_data.remove("date") else {
                    return ExecutedOp::failed(&op, "No date found in temporal content");
                };

                match store.create_event(user_id, &date_str, event_data) {
                    Ok(()) => ExecutedOp::success(&op),
                    Err(err) => ExecutedOp::failed(&op, err.to_string()),
                }
            }
            OperationType::Update => {
                let mut event_data = parse_temporal_content(&op.content);
                let Some(date_str) = event_data.remove("date") else {
                    return ExecutedOp::failed(&op, "No date found in temporal content");
                };

                match store.update_event(user_id, &date_str, event_data) {
                    Ok(()) => ExecutedOp::success(&op),
                    Err(err) => ExecutedOp::failed(&op, err.to_string()),
                }
            }
            OperationType::Delete => {
                let Some(id) = op.embedding_id.as_deref() else {
                    return ExecutedOp::failed(&op, "DELETE missing embedding_id");
                };

                match store.delete_event(user_id, id) {
                    Ok(()) => ExecutedOp::success(&op),
                    Err(err) => ExecutedOp::failed(&op, err.to_string()),
                }
            }
            OperationType::Noop => ExecutedOp::skipped(&op, None),
        }
    }

    fn execute_code(&mut self, op: Operation, user_id: &str) -> ExecutedOp {
        let Some(embedding_provider) = self.embedding_provider else {
            return ExecutedOp::failed(&op, "No embedding provider attached");
        };

        let store = match self.code_vector_store.as_deref_mut() {
            Some(store) => store,
            None => match self.vector_store.as_deref_mut() {
                Some(store) => store,
                None => return ExecutedOp::failed(&op, "No vector store for code domain"),
            },
        };

        let parsed = parse_code_annotation_content(&op.content);
        let mut metadata = Metadata::new();
        metadata.insert("user_id".to_string(), user_id.to_string());
        metadata.insert("domain".to_string(), "code".to_string());
        copy_metadata(&mut metadata, &parsed, "annotation_type");
        copy_metadata(&mut metadata, &parsed, "target_symbol");
        copy_metadata(&mut metadata, &parsed, "target_file");
        copy_metadata(&mut metadata, &parsed, "repo");
        copy_metadata(&mut metadata, &parsed, "severity");

        let executed = match op.operation_type {
            OperationType::Add => vector_add_with_metadata(store, embedding_provider, &op, metadata),
            OperationType::Update => {
                vector_update_with_metadata(store, embedding_provider, &op, metadata)
            }
            OperationType::Delete => vector_delete(store, &op),
            OperationType::Noop => ExecutedOp::skipped(&op, None),
        };

        if executed.status == OpStatus::Success && op.operation_type == OperationType::Add {
            if let Some(annotation_store) = self.annotation_store.as_deref_mut() {
                let _ = annotation_store.create_annotation(parsed);
            }
        }

        executed
    }

    fn execute_snippet(&mut self, op: Operation, user_id: &str) -> ExecutedOp {
        let Some(embedding_provider) = self.embedding_provider else {
            return ExecutedOp::failed(&op, "No embedding provider attached");
        };

        let store = match self.snippet_vector_store.as_deref_mut() {
            Some(store) => store,
            None => match self.vector_store.as_deref_mut() {
                Some(store) => store,
                None => return ExecutedOp::failed(&op, "No vector store for snippet domain"),
            },
        };

        let parsed = parse_snippet_content(&op.content);
        let searchable = parsed
            .get("content")
            .cloned()
            .unwrap_or_else(|| op.content.clone());

        let mut metadata = Metadata::new();
        metadata.insert("user_id".to_string(), user_id.to_string());
        metadata.insert("domain".to_string(), "snippet".to_string());
        metadata.insert("source".to_string(), "chat".to_string());
        copy_metadata(&mut metadata, &parsed, "code_snippet");
        copy_metadata(&mut metadata, &parsed, "language");
        copy_metadata(&mut metadata, &parsed, "snippet_type");
        copy_metadata(&mut metadata, &parsed, "tags");

        let searchable_op = Operation {
            content: searchable,
            ..op
        };

        match searchable_op.operation_type {
            OperationType::Add => {
                vector_add_with_metadata(store, embedding_provider, &searchable_op, metadata)
            }
            OperationType::Update => {
                vector_update_with_metadata(store, embedding_provider, &searchable_op, metadata)
            }
            OperationType::Delete => vector_delete(store, &searchable_op),
            OperationType::Noop => ExecutedOp::skipped(&searchable_op, None),
        }
    }
}

impl<'a> Default for Weaver<'a> {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_missing_id(op: Operation) -> Operation {
    if matches!(op.operation_type, OperationType::Update | OperationType::Delete)
        && op.embedding_id.is_none()
    {
        return op.as_add();
    }

    op
}

fn vector_add(
    store: &mut dyn VectorStore,
    embedding_provider: &dyn EmbeddingProvider,
    op: &Operation,
    domain: JudgeDomain,
    user_id: &str,
) -> ExecutedOp {
    vector_add_with_metadata(
        store,
        embedding_provider,
        op,
        vector_metadata(domain, user_id, &op.content),
    )
}

fn vector_update(
    store: &mut dyn VectorStore,
    embedding_provider: &dyn EmbeddingProvider,
    op: &Operation,
    domain: JudgeDomain,
    user_id: &str,
) -> ExecutedOp {
    vector_update_with_metadata(
        store,
        embedding_provider,
        op,
        vector_metadata(domain, user_id, &op.content),
    )
}

fn vector_add_with_metadata(
    store: &mut dyn VectorStore,
    embedding_provider: &dyn EmbeddingProvider,
    op: &Operation,
    metadata: Metadata,
) -> ExecutedOp {
    match embedding_provider.embed(&op.content) {
        Ok(embedding) => {
            let documents = vec![VectorDocument {
                id: None,
                text: op.content.clone(),
                embedding,
                metadata,
            }];

            match store.add(documents) {
                Ok(ids) => ExecutedOp::success_with_new_id(op, ids.into_iter().next()),
                Err(err) => ExecutedOp::failed(op, err.to_string()),
            }
        }
        Err(err) => ExecutedOp::failed(op, err.to_string()),
    }
}

fn vector_update_with_metadata(
    store: &mut dyn VectorStore,
    embedding_provider: &dyn EmbeddingProvider,
    op: &Operation,
    metadata: Metadata,
) -> ExecutedOp {
    let Some(id) = op.embedding_id.as_deref() else {
        return vector_add_with_metadata(store, embedding_provider, op, metadata);
    };

    match embedding_provider.embed(&op.content) {
        Ok(embedding) => match store.update(id, &op.content, embedding, metadata.clone()) {
            Ok(true) => ExecutedOp::success(op),
            Ok(false) => vector_add_with_metadata(store, embedding_provider, op, metadata),
            Err(err) => ExecutedOp::failed(op, err.to_string()),
        },
        Err(err) => ExecutedOp::failed(op, err.to_string()),
    }
}

fn vector_delete(store: &mut dyn VectorStore, op: &Operation) -> ExecutedOp {
    let Some(id) = op.embedding_id.clone() else {
        return ExecutedOp::failed(op, "DELETE missing embedding_id");
    };

    match store.delete(&[id]) {
        Ok(true) => ExecutedOp::success(op),
        Ok(false) => ExecutedOp {
            status: OpStatus::Failed,
            ..ExecutedOp::success(op)
        },
        Err(err) => ExecutedOp::failed(op, err.to_string()),
    }
}

fn vector_metadata(domain: JudgeDomain, user_id: &str, content: &str) -> Metadata {
    let mut metadata = BTreeMap::new();
    metadata.insert("user_id".to_string(), user_id.to_string());
    metadata.insert("domain".to_string(), domain.as_str().to_string());
    metadata.extend(extract_structured_metadata(content));
    metadata
}

fn copy_metadata(target: &mut Metadata, source: &Metadata, key: &str) {
    if let Some(value) = source.get(key) {
        target.insert(key.to_string(), value.clone());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Default)]
    struct FakeEmbedding;

    impl EmbeddingProvider for FakeEmbedding {
        fn embed(&self, text: &str) -> Result<Vec<f32>, WeaverError> {
            Ok(vec![text.len() as f32, 0.0, 1.0])
        }
    }

    #[derive(Default)]
    struct FakeVectorStore {
        records: HashMap<String, VectorDocument>,
        next_id: usize,
        delete_calls: Vec<Vec<String>>,
    }

    impl VectorStore for FakeVectorStore {
        fn add(&mut self, documents: Vec<VectorDocument>) -> Result<Vec<String>, WeaverError> {
            let mut ids = Vec::new();
            for mut doc in documents {
                self.next_id += 1;
                let id = format!("vec-{}", self.next_id);
                doc.id = Some(id.clone());
                self.records.insert(id.clone(), doc);
                ids.push(id);
            }
            Ok(ids)
        }

        fn update(
            &mut self,
            id: &str,
            text: &str,
            embedding: Vec<f32>,
            metadata: Metadata,
        ) -> Result<bool, WeaverError> {
            if !self.records.contains_key(id) {
                return Ok(false);
            }
            self.records.insert(
                id.to_string(),
                VectorDocument {
                    id: Some(id.to_string()),
                    text: text.to_string(),
                    embedding,
                    metadata,
                },
            );
            Ok(true)
        }

        fn delete(&mut self, ids: &[String]) -> Result<bool, WeaverError> {
            self.delete_calls.push(ids.to_vec());
            for id in ids {
                self.records.remove(id);
            }
            Ok(true)
        }
    }

    #[derive(Default)]
    struct FakeTemporalStore {
        created: Vec<(String, String, Metadata)>,
        updated: Vec<(String, String, Metadata)>,
        deleted: Vec<(String, String)>,
    }

    impl TemporalGraphStore for FakeTemporalStore {
        fn create_event(
            &mut self,
            user_id: &str,
            date_str: &str,
            event_data: Metadata,
        ) -> Result<(), WeaverError> {
            self.created
                .push((user_id.to_string(), date_str.to_string(), event_data));
            Ok(())
        }

        fn update_event(
            &mut self,
            user_id: &str,
            date_str: &str,
            event_data: Metadata,
        ) -> Result<(), WeaverError> {
            self.updated
                .push((user_id.to_string(), date_str.to_string(), event_data));
            Ok(())
        }

        fn delete_event(&mut self, user_id: &str, embedding_id: &str) -> Result<(), WeaverError> {
            self.deleted
                .push((user_id.to_string(), embedding_id.to_string()));
            Ok(())
        }
    }

    #[test]
    fn noops_do_not_execute_writes() {
        let mut store = FakeVectorStore::default();
        let embedder = FakeEmbedding;
        let mut weaver = Weaver::new()
            .with_vector_store(&mut store)
            .with_embedding_provider(&embedder);

        let result = weaver.execute(
            &JudgeResult {
                operations: vec![Operation::noop()],
                confidence: 1.0,
            },
            JudgeDomain::Profile,
            "user-1",
        );

        assert_eq!(result.total(), 0);
    }

    #[test]
    fn profile_adds_are_batched_and_metadata_is_structured() {
        let mut store = FakeVectorStore::default();
        let embedder = FakeEmbedding;
        let mut weaver = Weaver::new()
            .with_vector_store(&mut store)
            .with_embedding_provider(&embedder);

        let result = weaver.execute(
            &JudgeResult {
                operations: vec![Operation::add("work / company = OpenAI")],
                confidence: 1.0,
            },
            JudgeDomain::Profile,
            "user-1",
        );
        drop(weaver);

        assert_eq!(result.succeeded(), 1);
        let doc = store.records.values().next().unwrap();
        assert_eq!(doc.metadata["user_id"], "user-1");
        assert_eq!(doc.metadata["domain"], "profile");
        assert_eq!(doc.metadata["main_content"], "work_company");
        assert_eq!(doc.metadata["subcontent"], "OpenAI");
    }

    #[test]
    fn profile_update_falls_back_to_add_when_target_missing() {
        let mut store = FakeVectorStore::default();
        let embedder = FakeEmbedding;
        let mut weaver = Weaver::new()
            .with_vector_store(&mut store)
            .with_embedding_provider(&embedder);

        let result = weaver.execute(
            &JudgeResult {
                operations: vec![Operation::update("work / company = OpenAI", "missing-id")],
                confidence: 1.0,
            },
            JudgeDomain::Profile,
            "user-1",
        );
        drop(weaver);

        assert_eq!(result.succeeded(), 1);
        assert_eq!(store.records.len(), 1);
    }

    #[test]
    fn temporal_add_executes_graph_create() {
        let mut graph = FakeTemporalStore::default();
        let mut weaver = Weaver::new().with_temporal_graph_store(&mut graph);

        let result = weaver.execute(
            &JudgeResult {
                operations: vec![Operation::add(
                    "04-24 | Demo | Product demo | 2026 | 10:00 | today",
                )],
                confidence: 1.0,
            },
            JudgeDomain::Temporal,
            "user-1",
        );
        drop(weaver);

        assert_eq!(result.succeeded(), 1);
        assert_eq!(graph.created[0].0, "user-1");
        assert_eq!(graph.created[0].1, "04-24");
        assert_eq!(graph.created[0].2["event_name"], "Demo");
    }

    #[test]
    fn temporal_delete_uses_embedding_id() {
        let mut graph = FakeTemporalStore::default();
        let mut weaver = Weaver::new().with_temporal_graph_store(&mut graph);

        let result = weaver.execute(
            &JudgeResult {
                operations: vec![Operation::delete("04-24_Demo")],
                confidence: 1.0,
            },
            JudgeDomain::Temporal,
            "user-1",
        );
        drop(weaver);

        assert_eq!(result.succeeded(), 1);
        assert_eq!(graph.deleted, vec![("user-1".to_string(), "04-24_Demo".to_string())]);
    }

    #[test]
    fn operation_json_matches_python_shape() {
        let op = Operation::update("work / company = OpenAI", "profile-1");
        let json = serde_json::to_value(op).unwrap();

        assert_eq!(json["type"], "UPDATE");
        assert_eq!(json["content"], "work / company = OpenAI");
        assert_eq!(json["embedding_id"], "profile-1");
    }
}
