use pocketflow_rs::ProcessState;
use strum::Display;
use crate::utils::pi_llm::PiLLM;
use crate::utils::session_manager::SessionManager;
use crate::utils::config::AppConfig;

#[derive(Debug, Clone, PartialEq, Default, Display)]
#[strum(serialize_all = "snake_case")]
pub enum PiState {
    #[default]
    Default,
    CheckSize,
    DoCompact,
    CallLLM,
    ExecuteTool,
    WaitForInput,
    Finished,
}

impl ProcessState for PiState {
    fn is_default(&self) -> bool {
        matches!(self, PiState::Default)
    }
}

// Global shared components between nodes
pub struct AppContext {
    pub llm: PiLLM,
    pub session_manager: SessionManager,
    pub config: AppConfig,
    pub model_name: String,
}
