use crate::state::{AppContext, PiState};
use anyhow::Result;
use pocketflow_rs::{Context, Node, ProcessResult};
use serde_json::{Value, json};
use std::sync::Arc;

pub struct CheckSizeNode {
    pub app: Arc<AppContext>,
}

#[async_trait::async_trait]
impl Node for CheckSizeNode {
    type State = PiState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let messages = context.get("messages").unwrap_or(&json!([])).clone();
        let msgs_str = serde_json::to_string(&messages).unwrap_or_default();
        let estimated_tokens = msgs_str.len() / 4;

        // Handle gracefully if config doesn't perfectly match model
        let default_compact_thresh = 100000;
        let threshold = self
            .app
            .config
            .models
            .get(&self.app.model_name)
            .map(|m| m.compact_threshold)
            .unwrap_or(default_compact_thresh);

        if self.app.config.general.auto_compact
            && estimated_tokens > threshold
            && messages.as_array().map(|a| a.len() > 3).unwrap_or(false)
        {
            println!(
                "\n[Auto Compacting History (est {} tokens > {})]...",
                estimated_tokens, threshold
            );
            Ok(json!({ "needs_compact": true, "history_str": msgs_str }))
        } else {
            Ok(json!({ "needs_compact": false }))
        }
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<PiState>> {
        let res = result.as_ref().unwrap();
        if res.get("needs_compact").and_then(|v| v.as_bool()) == Some(true) {
            context.set(
                "history_to_compact",
                res.get("history_str").unwrap().clone(),
            );
            Ok(ProcessResult::new(
                PiState::DoCompact,
                "do_compact".to_string(),
            ))
        } else {
            Ok(ProcessResult::new(PiState::CallLLM, "call_llm".to_string()))
        }
    }
}
