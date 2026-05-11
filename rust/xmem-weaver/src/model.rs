use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    #[serde(rename = "ADD")]
    Add,
    #[serde(rename = "UPDATE")]
    Update,
    #[serde(rename = "DELETE")]
    Delete,
    #[serde(rename = "NOOP")]
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JudgeDomain {
    #[serde(rename = "profile")]
    Profile,
    #[serde(rename = "temporal")]
    Temporal,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "code")]
    Code,
    #[serde(rename = "snippet")]
    Snippet,
}

impl JudgeDomain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Temporal => "temporal",
            Self::Summary => "summary",
            Self::Image => "image",
            Self::Code => "code",
            Self::Snippet => "snippet",
        }
    }

    pub fn is_batched_vector_domain(self) -> bool {
        matches!(self, Self::Profile | Self::Summary | Self::Image)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    #[serde(rename = "type")]
    pub operation_type: OperationType,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub embedding_id: Option<String>,
    #[serde(default)]
    pub reason: String,
}

impl Operation {
    pub fn add(content: impl Into<String>) -> Self {
        Self {
            operation_type: OperationType::Add,
            content: content.into(),
            embedding_id: None,
            reason: String::new(),
        }
    }

    pub fn update(content: impl Into<String>, embedding_id: impl Into<String>) -> Self {
        Self {
            operation_type: OperationType::Update,
            content: content.into(),
            embedding_id: Some(embedding_id.into()),
            reason: String::new(),
        }
    }

    pub fn delete(embedding_id: impl Into<String>) -> Self {
        Self {
            operation_type: OperationType::Delete,
            content: String::new(),
            embedding_id: Some(embedding_id.into()),
            reason: String::new(),
        }
    }

    pub fn noop() -> Self {
        Self {
            operation_type: OperationType::Noop,
            content: String::new(),
            embedding_id: None,
            reason: String::new(),
        }
    }

    pub fn as_add(mut self) -> Self {
        self.operation_type = OperationType::Add;
        self.embedding_id = None;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JudgeResult {
    #[serde(default)]
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub confidence: f32,
}

impl JudgeResult {
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn has_writes(&self) -> bool {
        self.operations
            .iter()
            .any(|op| op.operation_type != OperationType::Noop)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "skipped")]
    Skipped,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutedOp {
    #[serde(rename = "type")]
    pub operation_type: OperationType,
    pub status: OpStatus,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub embedding_id: Option<String>,
    #[serde(default)]
    pub new_id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

impl ExecutedOp {
    pub fn success(op: &Operation) -> Self {
        Self {
            operation_type: op.operation_type,
            status: OpStatus::Success,
            content: op.content.clone(),
            embedding_id: op.embedding_id.clone(),
            new_id: None,
            error: None,
        }
    }

    pub fn success_with_new_id(op: &Operation, new_id: Option<String>) -> Self {
        Self {
            new_id,
            ..Self::success(op)
        }
    }

    pub fn skipped(op: &Operation, error: Option<String>) -> Self {
        Self {
            operation_type: op.operation_type,
            status: OpStatus::Skipped,
            content: op.content.clone(),
            embedding_id: op.embedding_id.clone(),
            new_id: None,
            error,
        }
    }

    pub fn failed(op: &Operation, error: impl Into<String>) -> Self {
        Self {
            operation_type: op.operation_type,
            status: OpStatus::Failed,
            content: op.content.clone(),
            embedding_id: op.embedding_id.clone(),
            new_id: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WeaverResult {
    #[serde(default)]
    pub executed: Vec<ExecutedOp>,
}

impl WeaverResult {
    pub fn total(&self) -> usize {
        self.executed.len()
    }

    pub fn succeeded(&self) -> usize {
        self.executed
            .iter()
            .filter(|op| op.status == OpStatus::Success)
            .count()
    }

    pub fn skipped(&self) -> usize {
        self.executed
            .iter()
            .filter(|op| op.status == OpStatus::Skipped)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.executed
            .iter()
            .filter(|op| op.status == OpStatus::Failed)
            .count()
    }
}
