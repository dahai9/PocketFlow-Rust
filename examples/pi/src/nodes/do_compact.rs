use anyhow::Result;
use pocketflow_rs::{Context, Node, ProcessResult};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use crate::state::{AppContext, PiState};
use crate::utils::session_manager::AgentMessage;

pub struct DoCompactNode {
    pub app: Arc<AppContext>,
}

#[async_trait::async_trait]
impl Node for DoCompactNode {
    type State = PiState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let history_str = context.get("history_to_compact").unwrap().as_str().unwrap();
        
        let summary_prompt = json!({
            "role": "user",
            "content": format!("Summarize the entire conversation history concisely, retaining all tool outcomes and important context so it can be used to replace the history entirely:\n{}", history_str)
        });
        
        println!("Sending compaction request to LLM...");
        let mut retries = 0;
        let max_retries = 3;
        loop {
            match self.app.llm.chat_completion(vec![summary_prompt.clone()], Value::Null).await {
                Ok(summary_res) => return Ok(summary_res),
                Err(e) => {
                    retries += 1;
                    if retries > max_retries {
                        return Err(e);
                    }
                    println!("[Compaction Failed]: {}. Retrying ({}/{}) in 2 seconds...", e, retries, max_retries);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<PiState>> {
        let res = match result {
            Ok(v) => v,
            Err(e) => {
                println!("[Compaction Failed]: {}", e);
                return Ok(ProcessResult::new(PiState::CallLLM, "call_llm_fallback".to_string()));
            }
        };

        if let Some(choices) = res.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                let summary_text = choice["message"]["content"].as_str().unwrap_or("");
                
                let compact_msg = AgentMessage {
                    id: Uuid::new_v4().to_string(),
                    parent_id: None,
                    role: "system".to_string(),
                    content: format!("Previous conversation summary:\n{}", summary_text),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    clears_history: Some(true),
                };
                
                self.app.session_manager.append_message(&compact_msg)?;
                
                let messages = json!([compact_msg]);
                context.set("messages", messages);
                println!("History compressed successfully.");
            }
        }
        
        Ok(ProcessResult::new(PiState::CallLLM, "call_llm".to_string()))
    }
}
