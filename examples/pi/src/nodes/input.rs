use crate::state::{AppContext, PiState};
use crate::utils::session_manager::AgentMessage;
use anyhow::Result;
use pocketflow_rs::{Context, Node, ProcessResult};
use serde_json::{Value, json};
use std::io::{self, Write};
use std::sync::Arc;
use uuid::Uuid;

pub struct InputNode {
    pub app: Arc<AppContext>,
}

#[async_trait::async_trait]
impl Node for InputNode {
    type State = PiState;

    async fn execute(&self, _context: &Context) -> Result<Value> {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let text = input.trim().to_string();

        if text == "exit" || text == "quit" {
            return Ok(json!({ "command": "exit" }));
        }

        let id = Uuid::new_v4().to_string();

        let msg = AgentMessage {
            id: id.clone(),
            parent_id: None,
            role: "user".to_string(),
            content: text,
            name: None,
            tool_calls: None,
            tool_call_id: None,
            clears_history: None,
        };

        // Persist immediately
        self.app.session_manager.append_message(&msg)?;

        Ok(json!({ "message": msg }))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<PiState>> {
        let res = result.as_ref().unwrap();
        if res.get("command").and_then(|v| v.as_str()) == Some("exit") {
            return Ok(ProcessResult::new(
                PiState::Finished,
                "finished".to_string(),
            ));
        }

        let msg_val = res.get("message").unwrap();
        let mut messages = context.get("messages").cloned().unwrap_or(json!([]));
        messages.as_array_mut().unwrap().push(msg_val.clone());
        context.set("messages", messages);

        Ok(ProcessResult::new(
            PiState::CheckSize,
            "check_size".to_string(),
        ))
    }
}
