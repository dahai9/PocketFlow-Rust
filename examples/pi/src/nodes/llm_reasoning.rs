use crate::state::{AppContext, PiState};
use crate::utils::session_manager::AgentMessage;
use anyhow::Result;
use pocketflow_rs::{Context, Node, ProcessResult};
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

pub struct LLMReasoningNode {
    pub app: Arc<AppContext>,
}

#[async_trait::async_trait]
impl Node for LLMReasoningNode {
    type State = PiState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let messages = context.get("messages").unwrap_or(&json!([])).clone();

        let tools = json!([
            {
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read the contents of a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" }
                        },
                        "required": ["path"]
                    }
                }
            },
            {
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "Write contents to a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "content": { "type": "string" }
                        },
                        "required": ["path", "content"]
                    }
                }
            },
            {
                "type": "function",
                "function": {
                    "name": "bash",
                    "description": "Execute a bash command",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string" }
                        },
                        "required": ["command"]
                    }
                }
            }
        ]);

        let mut openai_messages = Vec::new();
        // Convert AgentMessage to format expected by OpenAI
        if let Some(arr) = messages.as_array() {
            for m in arr {
                let mut mapped = json!({
                    "role": m["role"].as_str().unwrap(),
                    "content": m["content"].as_str().unwrap()
                });
                if let Some(calls) = m.get("tool_calls") {
                    if !calls.is_null() {
                        mapped
                            .as_object_mut()
                            .unwrap()
                            .insert("tool_calls".to_string(), calls.clone());
                    }
                }
                if let Some(tid) = m.get("tool_call_id") {
                    if !tid.is_null() {
                        mapped
                            .as_object_mut()
                            .unwrap()
                            .insert("tool_call_id".to_string(), tid.clone());
                    }
                }
                openai_messages.push(mapped);
            }
        }

        let response = self.app.llm.chat_completion(openai_messages, tools).await?;
        Ok(response)
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<PiState>> {
        let res = match result {
            Ok(v) => v,
            Err(e) => {
                println!("\n[LLM Error]: {}\n", e);
                return Ok(ProcessResult::new(
                    PiState::WaitForInput,
                    "error".to_string(),
                ));
            }
        };

        // Ensure choice 0 exists
        if let Some(choices) = res.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                let msg = choice.get("message").unwrap();
                let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                let tool_calls = msg.get("tool_calls");

                let agent_msg = AgentMessage {
                    id: Uuid::new_v4().to_string(),
                    parent_id: None,
                    role: "assistant".to_string(),
                    content: content.to_string(),
                    name: None,
                    tool_calls: tool_calls.cloned(),
                    tool_call_id: None,
                    clears_history: None,
                };

                // Persist
                self.app.session_manager.append_message(&agent_msg)?;

                // Print
                if !content.is_empty() {
                    println!("\nAssistant: {}\n", content);
                }

                // Update context
                let mut messages = context.get("messages").cloned().unwrap_or(json!([]));
                messages
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::to_value(&agent_msg)?);
                context.set("messages", messages);

                if let Some(tc) = tool_calls {
                    if !tc.is_null() && tc.as_array().map_or(false, |a| !a.is_empty()) {
                        return Ok(ProcessResult::new(
                            PiState::ExecuteTool,
                            "execute_tool".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(ProcessResult::new(
            PiState::WaitForInput,
            "wait_for_input".to_string(),
        ))
    }
}
