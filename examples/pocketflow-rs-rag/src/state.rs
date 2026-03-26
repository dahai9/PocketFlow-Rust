use pocketflow_rs::ProcessState;
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum RagState {
    // Offline states
    FileLoadedError,
    DocumentsLoaded,
    DocumentsChunked,
    ChunksEmbedded,
    IndexCreated,
    // Offline error states
    DocumentLoadError,
    ChunkingError,
    EmbeddingError,
    IndexCreationError,
    // Online states
    QueryEmbedded,
    DocumentsRetrieved,
    AnswerGenerated,
    // Online error states
    QueryEmbeddingError,
    RetrievalError,
    GenerationError,
    Default,
    QueryRewriteError,
}

impl ProcessState for RagState {
    fn is_default(&self) -> bool {
        matches!(self, RagState::Default)
    }
}

impl Default for RagState {
    fn default() -> Self {
        RagState::Default
    }
}
