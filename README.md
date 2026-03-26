<div align="center">
  <img src="./static/pocketflow_rust_title.png" alt="Pocket Flow – 100-line minimalist LLM framework" width="400"/>
</div>

A Rust implementation of [PocketFlow](https://github.com/The-Pocket/PocketFlow), a minimalist flow-based programming framework.

🚀 [Get started quickly with our template →](#template)

## ✨ Features

- 🦀 **Type-safe:** State transitions using Rust enums
- 🏗️ **Macro-based:** Flow construction using `build_flow!` and `build_batch_flow!`
- ⚡ **Async first:** Non-blocking node execution and post-processing
- 📦 **Batch support:** High-performance processing of multiple contexts
- 🧩 **Extensible:** Custom state management and node systems
- 🛠️ **Utility-rich:** Optional integrations for OpenAI, Qdrant, and web search

## 🚀 Quick Start

### 0. Setup

```toml
[dependencies]
pocketflow_rs = "0.1.0"
strum = { version = "0.26", features = ["derive"] }
```

### 1. Define Custom States

```rust
use pocketflow_rs::ProcessState;
use strum::Display;

#[derive(Debug, Clone, PartialEq, Default, Display)]
#[strum(serialize_all = "snake_case")]
pub enum MyState {
    Success,
    Failure,
    #[default]
    Default,
}

impl ProcessState for MyState {
    fn is_default(&self) -> bool {
        matches!(self, MyState::Default)
    }
}
```

### 2. Implement Nodes

```rust
use pocketflow_rs::{Node, ProcessResult, Context};
use anyhow::Result;
use async_trait::async_trait;

struct MyNode;

#[async_trait]
impl Node for MyNode {
    type State = MyState;

    async fn execute(&self, context: &Context) -> Result<serde_json::Value> {
        // Your node logic here
        Ok(serde_json::json!({"data": 42}))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<serde_json::Value>,
    ) -> Result<ProcessResult<MyState>> {
        // Your post-processing logic here
        Ok(ProcessResult::new(MyState::Success, "success"))
    }
}
```

### 3. Build & Run Flows

```rust
use pocketflow_rs::{build_flow, Context};

let node1 = MyNode;
let node2 = MyNode;

let flow = build_flow!(
    start: ("start", node1),
    nodes: [("next", node2)],
    edges: [
        ("start", "next", MyState::Success)
    ]
);

let context = Context::new();
let result = flow.run(context).await?;
```

## 🏗️ Advanced Usage

### Batch Processing

Build high-throughput flows for parallel processing:

```rust
use pocketflow_rs::build_batch_flow;

let batch_flow = build_batch_flow!(
    start: ("start", node1),
    nodes: [("next", node2)],
    edges: [
        ("start", "next", MyState::Success)
    ],
    batch_size: 10
);

let contexts = vec![Context::new(); 10];
let results = batch_flow.run_batch(contexts).await?;
```

## 🛠️ Available Features

Customize `pocketflow_rs` by enabling the features you need in your `Cargo.toml`:

| Feature | Description |
|---------|-------------|
| `openai` (default) | OpenAI API integration for LLM capabilities |
| `websearch` | Google Custom Search API integration |
| `qdrant` | Vector database integration using Qdrant |
| `debug` | Enhanced logging and visualization tools |

Example:
```toml
pocketflow_rs = { version = "0.1.0", features = ["openai", "qdrant"] }
```

## 📂 Examples

Check out the `examples/` directory for detailed implementations:

- 🟢 [**basic.rs**](./examples/basic.rs): Basic flow with custom states
- 🗃️ [**text2sql**](./examples/text2sql/): Text-to-SQL workflow using OpenAI
- 🔍 [**pocketflow-rs-rag**](./examples/pocketflow-rs-rag/): Retrieval-Augmented Generation (RAG) system

## 📋 Template

Don't start from scratch! Use the [PocketFlow-Template-Rust](https://github.com/The-Pocket/PocketFlow-Template-Rust) to kickstart your project.

## License

MIT
