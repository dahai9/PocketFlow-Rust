use anyhow::Result;
use clap::Parser;
use pocketflow_rs::{build_flow, Context, Flow, Node, ProcessResult, ProcessState};
use serde_json::{json, Value};
use std::io::{self, Write};
use std::sync::Arc;
use strum::Display;
use uuid::Uuid;
use pocketflow_rs::utils::pi_llm::PiLLM;
use pocketflow_rs::utils::session_manager::{AgentMessage, SessionManager};
use pocketflow_rs::utils::tools::{execute_bash, read_file, write_file};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    interactive: bool,

    #[arg(short, long, default_value = "openai")]
    provider: String,

    #[arg(short, long, default_value = "gpt-4o")]
    model: String,
}

#[derive(Debug, Clone, PartialEq, Default, Display)]
#[strum(serialize_all = "snake_case")]
enum PiState {
    #[default]
    Default,
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
struct AppContext {
    llm: PiLLM,
    session_manager: SessionManager,
}

struct InputNode {
    app: Arc<AppContext>,
}

#[async_trait::async_trait]
impl Node for InputNode {
    type State = PiState;

    async fn execute(&self, context: &Context) -> Result<Value> {
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
            return Ok(ProcessResult::new(PiState::Finished, "finished".to_string()));
        }

        let msg_val = res.get("message").unwrap();
        let mut messages = context.get("messages").cloned().unwrap_or(json!([]));
        messages.as_array_mut().unwrap().push(msg_val.clone());
        context.set("messages", messages);

        Ok(ProcessResult::new(PiState::CallLLM, "call_llm".to_string()))
    }
}

struct LLMReasoningNode {
    app: Arc<AppContext>,
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
                        mapped.as_object_mut().unwrap().insert("tool_calls".to_string(), calls.clone());
                    }
                }
                if let Some(tid) = m.get("tool_call_id") {
                    if !tid.is_null() {
                        mapped.as_object_mut().unwrap().insert("tool_call_id".to_string(), tid.clone());
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
                return Ok(ProcessResult::new(PiState::WaitForInput, "error".to_string()));
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
                };

                // Persist
                self.app.session_manager.append_message(&agent_msg)?;

                // Print
                if !content.is_empty() {
                    println!("\nAssistant: {}\n", content);
                }

                // Update context
                let mut messages = context.get("messages").cloned().unwrap_or(json!([]));
                messages.as_array_mut().unwrap().push(serde_json::to_value(&agent_msg)?);
                context.set("messages", messages);

                if let Some(tc) = tool_calls {
                    if !tc.is_null() && tc.as_array().map_or(false, |a| !a.is_empty()) {
                        return Ok(ProcessResult::new(PiState::ExecuteTool, "execute_tool".to_string()));
                    }
                }
            }
        }

        Ok(ProcessResult::new(PiState::WaitForInput, "wait_for_input".to_string()))
    }
}

struct ToolExecutionNode {
    app: Arc<AppContext>,
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
                    id: Uuid::new_v4().to_string(),
                    parent_id: None,
                    role: "tool".to_string(),
                    content: output,
                    name: Some(name.to_string()),
                    tool_calls: None,
                    tool_call_id: Some(id),
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

        Ok(ProcessResult::new(PiState::CallLLM, "call_llm".to_string()))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Setup directory and SessionManager
    let cwd = std::env::current_dir()?;
    let session_manager = SessionManager::new(&cwd);

    // Load API Key
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "dummy_key".to_string());
    let mut endpoint = std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    if !endpoint.ends_with("/chat/completions") {
        endpoint = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
    }

    let llm = PiLLM::new(api_key, args.model, endpoint);

    let app_context = Arc::new(AppContext {
        llm,
        session_manager,
    });

    let input_node = InputNode { app: app_context.clone() };
    let llm_node = LLMReasoningNode { app: app_context.clone() };
    let tool_node = ToolExecutionNode { app: app_context.clone() };

    let flow = build_flow!(
        start: ("input", input_node),
        nodes: [
            ("llm", llm_node),
            ("tool", tool_node)
        ],
        edges: [
            ("input", "llm", PiState::CallLLM),
            ("llm", "tool", PiState::ExecuteTool),
            ("llm", "input", PiState::WaitForInput),
            ("tool", "llm", PiState::CallLLM)
            // Implicit default stop for PiState::Finished
        ]
    );

    let mut context = Context::new();
    
    // Load history
    let history = app_context.session_manager.load_history(None)?;
    if !history.is_empty() {
        println!("Loaded {} messages from history.", history.len());
        let val = serde_json::to_value(history)?;
        context.set("messages", val);
    } else {
        context.set("messages", json!([]));
    }

    println!("pi agent started. Type 'exit' to quit.");
    
    match flow.run(context).await {
        Ok(_) => println!("Agent shutdown."),
        Err(e) => eprintln!("Error running flow: {}", e),
    }

    Ok(())
}
