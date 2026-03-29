use anyhow::Result;
use clap::Parser;
use pi::{
    AppConfig, AppContext, CheckSizeNode, DoCompactNode, InputNode, LLMReasoningNode, PiLLM,
    PiState, SessionManager, ToolExecutionNode,
};
use pocketflow_rs::{Context, build_flow};
use serde_json::json;
use std::sync::Arc;

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup directory and SessionManager
    let cwd = std::env::current_dir()?;
    let session_manager = SessionManager::new(&cwd);

    // Load config
    let config = AppConfig::load(&cwd)?;

    // Load API Key
    let (api_key, mut endpoint): (String, String) =
        if let Some(model_conf) = config.models.get(&args.model) {
            if let Some(provider_conf) = config.providers.get(&model_conf.provider) {
                let key = std::env::var(&provider_conf.api_key_env)
                    .unwrap_or_else(|_| "dummy_key".to_string());
                (key, provider_conf.api_base.clone())
            } else {
                (
                    std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "dummy_key".to_string()),
                    "https://api.openai.com/v1".to_string(),
                )
            }
        } else {
            println!(
                "Warning: Model '{}' not found in config, falling back to openai env vars.",
                args.model
            );
            (
                std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "dummy_key".to_string()),
                std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            )
        };

    if !endpoint.ends_with("/chat/completions") {
        endpoint = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
    }

    let llm = PiLLM::new(api_key, args.model.clone(), endpoint);

    let app_context = Arc::new(AppContext {
        llm,
        session_manager,
        config,
        model_name: args.model.clone(),
    });

    let input_node = InputNode {
        app: app_context.clone(),
    };
    let check_size_node = CheckSizeNode {
        app: app_context.clone(),
    };
    let compact_node = DoCompactNode {
        app: app_context.clone(),
    };
    let llm_node = LLMReasoningNode {
        app: app_context.clone(),
    };
    let tool_node = ToolExecutionNode {
        app: app_context.clone(),
    };

    let flow = build_flow!(
        start: ("input", input_node),
        nodes: [
            ("check_size", check_size_node),
            ("do_compact", compact_node),
            ("llm", llm_node),
            ("tool", tool_node)
        ],
        edges: [
            ("input", "check_size", PiState::CheckSize),
            ("check_size", "do_compact", PiState::DoCompact),
            ("check_size", "llm", PiState::CallLLM),
            ("do_compact", "llm", PiState::CallLLM),
            ("llm", "tool", PiState::ExecuteTool),
            ("llm", "input", PiState::WaitForInput),
            ("tool", "check_size", PiState::CheckSize)
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
    // Export flow visualization
    std::fs::create_dir_all("test_dir")?;
    let mermaid = flow.to_mermaid();
    std::fs::write("test_dir/pi_flow.mmd", mermaid)?;
    println!("Saved flow visualization to test_dir/pi_flow.mmd");
    println!("pi agent started. Type 'exit' to quit.");

    match flow.run(context).await {
        Ok(_) => println!("Agent shutdown."),
        Err(e) => eprintln!("Error running flow: {}", e),
    }

    Ok(())
}
