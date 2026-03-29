use anyhow::Result;
use pocketflow_rs::{Context, Node, ProcessResult};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::state::{AppContext, PiState};
use crate::utils::session_manager::AgentMessage;
use crate::utils::tools::{execute_bash, read_file, write_file};

pub struct ToolExecutionNode {
    pub app: Arc<AppContext>,
}

#[async_trait::async_trait]
impl Node for ToolExecutionNode {
    type State = PiState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let messages = context.get("messages").unwrap().as_array().unwrap();
        let last_msg = messages.last().unwrap();
        
        let mut tool_results = Vec::new();

        if let Some(tool_calls) = last_msg.get("tool_calls").and_then(|tc| tc.as_array()) {
            for call in tool_calls {
                let id = call["id"].as_str().unwrap().to_string();
                let func = &call["function"];
                let name = func["name"].as_str().unwrap();
                let args_str = func["arguments"].as_str().unwrap();
                let args: Value = serde_json::from_str(args_str)?;

                println!("Executing tool: {} with args: {}", name, args_str);
                
                let output = match name {
                    "read_file" => {
                        let path = args["path"].as_str().unwrap();
                        read_file(path)
                    }
                    "write_file" => {
                        let path = args["path"].as_str().unwrap();
                        let content = args["content"].as_str().unwrap();
                        write_file(path, content)
                    }
                    "bash" => {
                        let command = args["command"].as_str().unwrap();
                        execute_bash(command, ".")
                    }
                    _ => format!("Unknown tool: {}", name),
                };

                let agent_msg = AgentMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id: None,
                    role: "tool".to_string(),
                    content: output,
                    name: Some(name.to_string()),
                    tool_calls: None,
                    tool_call_id: Some(id),
                    clears_history: None,
                };

                tool_results.push(agent_msg);
            }
        }

        Ok(serde_json::to_value(tool_results)?)
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<PiState>> {
        let tool_results: Vec<AgentMessage> = serde_json::from_value(result.as_ref().unwrap().clone())?;
        
        let mut messages = context.get("messages").cloned().unwrap_or(json!([]));
        
        for msg in tool_results {
            self.app.session_manager.append_message(&msg)?;
            messages.as_array_mut().unwrap().push(serde_json::to_value(&msg)?);
        }
        
        context.set("messages", messages);

        Ok(ProcessResult::new(PiState::CheckSize, "check_size".to_string()))
    }
}
