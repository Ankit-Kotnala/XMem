use std::collections::BTreeMap;

use crate::weaver::WeaverError;

pub type Metadata = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq)]
pub struct VectorDocument {
    pub id: Option<String>,
    pub text: String,
    pub embedding: Vec<f32>,
    pub metadata: Metadata,
}

pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, WeaverError>;
}

pub trait VectorStore {
    fn add(&mut self, documents: Vec<VectorDocument>) -> Result<Vec<String>, WeaverError>;

    fn update(
        &mut self,
        id: &str,
        text: &str,
        embedding: Vec<f32>,
        metadata: Metadata,
    ) -> Result<bool, WeaverError>;

    fn delete(&mut self, ids: &[String]) -> Result<bool, WeaverError>;
}

pub trait TemporalGraphStore {
    fn create_event(
        &mut self,
        user_id: &str,
        date_str: &str,
        event_data: Metadata,
    ) -> Result<(), WeaverError>;

    fn update_event(
        &mut self,
        user_id: &str,
        date_str: &str,
        event_data: Metadata,
    ) -> Result<(), WeaverError>;

    fn delete_event(&mut self, user_id: &str, embedding_id: &str) -> Result<(), WeaverError>;
}

pub trait GraphAnnotationStore {
    fn create_annotation(&mut self, metadata: Metadata) -> Result<(), WeaverError>;
}
