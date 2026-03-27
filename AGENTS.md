---
layout: default
title: "Agentic Coding"
---

# Agentic Coding: Humans Design, Agents code!

> If you are an AI agent building apps with PocketFlow-Rust, read this guide carefully. Start small, design at a high level first (`docs/design.md`), then implement and verify.
{: .warning }

## Agentic Coding Steps

Agentic coding should be a collaboration between human system design and AI implementation.

| Steps | Human | AI | Comment |
|:--|:--:|:--:|:--|
| 1. Requirements | ★★★ High | ★☆☆ Low | Humans define the problem and success criteria. |
| 2. Flow | ★★☆ Medium | ★★☆ Medium | Humans define the orchestration; AI fills in details. |
| 3. Utilities | ★★☆ Medium | ★★☆ Medium | Humans provide external APIs; AI helps implement wrappers. |
| 4. Data | ★☆☆ Low | ★★★ High | AI proposes the schema; humans verify it matches the app. |
| 5. Node | ★☆☆ Low | ★★★ High | AI designs nodes around the flow and shared store. |
| 6. Implementation | ★☆☆ Low | ★★★ High | AI implements the flow and nodes from the design. |
| 7. Optimization | ★★☆ Medium | ★★☆ Medium | Iterate on prompts, data shape, and flow structure. |
| 8. Reliability | ★☆☆ Low | ★★★ High | Add validation, retries, logging, and tests. |

1. **Requirements**: Clarify the user problem, not just the feature list.
   - Good for: repetitive tasks, structured transformations, workflow automation, RAG, agent loops.
   - Not good for: vague goals without measurable outputs or unstable business decisions.
   - Keep it user-centric and small at first.

2. **Flow Design**: Define the graph at a high level.
   - Pick a pattern if it fits: [Agent](./design_pattern/agent.md), [Workflow](./design_pattern/workflow.md), [RAG](./design_pattern/rag.md), [Map Reduce](./design_pattern/mapreduce.md).
   - For each node, write a one-line purpose and its next action conditions.
   - Draw the flow in mermaid.
   - Use the Rust API as source of truth: `Node`, `Flow`, `BatchFlow`, `ProcessState`.
   - Example:
     ```mermaid
     flowchart LR
         start[Load Input] --> process[Process]
         process --> finish[Finish]
     ```
   - If you cannot describe the flow manually, do not automate it yet.

3. **Utilities**: Identify required external I/O helpers.
   - Think of utilities as the body of the agent: file I/O, web requests, LLM calls, DB access, embeddings.
   - Keep LLM tasks inside nodes or utilities, but do not confuse them with orchestration.
   - Put reusable wrappers in `src/utils/*.rs` and add a small test when practical.
   - Prefer returning `anyhow::Result<T>` and keep wrappers narrow and deterministic.
   - Example utility shape (real wrapper from `src/utils/llm_wrapper.rs`):
     ```rust
     use async_trait::async_trait;

     #[async_trait]
     pub trait LLMWrapper {
         async fn generate(&self, prompt: &str) -> anyhow::Result<LLMResponse>;
     }

     pub struct OpenAIClient {
         api_key: String,
         model: String,
         endpoint: String,
     }

     impl OpenAIClient {
         pub fn new(api_key: String, model: String, endpoint: String) -> Self {
             Self { api_key, model, endpoint }
         }
     }

     #[async_trait]
     impl LLMWrapper for OpenAIClient {
         async fn generate(&self, prompt: &str) -> anyhow::Result<LLMResponse> {
             // Use openai_api_rust or reqwest to call the API
             todo!()
         }
     }
     ```

4. **Data Design**: Design the shared store before coding the nodes.
   - The shared store is `Context`.
   - Use `Context::set()` / `Context::get()` for shared data.
   - Use `metadata` for auxiliary data that should not be treated as primary results.
   - Keep keys simple and avoid redundancy.
   - Example:
     ```rust
     use pocketflow_rs::Context;
     use serde_json::json;

     let mut context = Context::new();
     context.set("input", json!("hello"));
     context.set_metadata("source", json!("user"));
     ```

5. **Node Design**: Plan each node’s role and state transitions.
   - `prepare(&mut context)`: optional, read from `Context` and prepare inputs.
   - `execute(&context)`: do compute or remote calls; keep it idempotent when possible.
   - `post_process(&mut context, &result)`: write outputs back to `Context` and return `ProcessResult<Self::State>`.
   - Define a custom `State` enum implementing `ProcessState` for branching; use `BaseState` when simple.
   - Example node shape (from `examples/basic.rs`):
     ```rust
     use anyhow::Result;
     use async_trait::async_trait;
     use pocketflow_rs::{Context, Node, ProcessResult, ProcessState};
     use serde_json::Value;
     use strum::Display;

     #[derive(Debug, Clone, PartialEq, Default, Display)]
     #[strum(serialize_all = "snake_case")]
     enum MyState {
         #[default]
         Default,
         Success,
     }

     impl ProcessState for MyState {
         fn is_default(&self) -> bool {
             matches!(self, MyState::Default)
         }
     }

     struct MyNode;

     #[async_trait]
     impl Node for MyNode {
         type State = MyState;

         async fn execute(&self, context: &Context) -> Result<Value> {
             let input = context.get("input").cloned().unwrap_or(Value::Null);
             Ok(input)
         }

         async fn post_process(
             &self,
             context: &mut Context,
             result: &Result<Value>,
         ) -> Result<ProcessResult<Self::State>> {
             match result {
                 Ok(value) => {
                     context.set("output", value.clone());
                     Ok(ProcessResult::new(MyState::Success, "done".to_string()))
                 }
                 Err(e) => {
                     context.set("error", Value::String(e.to_string()));
                     Ok(ProcessResult::new(MyState::Default, e.to_string()))
                 }
             }
         }
     }
     ```

6. **Implementation**: Build the initial nodes and flows.
   - Keep the first pass simple.
   - Use `build_flow!` and `build_batch_flow!` instead of hand-wiring infrastructure.
   - Example flow assembly:
     ```rust
     use pocketflow_rs::{build_flow, Flow, BaseState};

     pub fn create_flow() -> Flow<BaseState> {
         build_flow!(
             start: ("get_input", GetInputNode),
             nodes: [
                 ("process", ProcessNode),
                 ("output", OutputNode)
             ],
             edges: [
                 ("get_input", "process", BaseState::Default),
                 ("process", "output", BaseState::Default)
             ]
         )
     }
     ```
   - Add logging via `tracing` where it helps debugging.
   - Prefer small, composable nodes over large monoliths.

7. **Optimization**: Improve after the first working version.
   - Refine the flow when the bottleneck is logic or structure.
   - Refine prompts and context when the bottleneck is model behavior.
   - Refine utilities when the bottleneck is I/O or integration.

8. **Reliability**: Make failures visible and recoverable.
   - Validate results in `execute` or `post_process`.
   - Use the framework’s retry behavior where available.
   - Add tests for utility wrappers and critical node transitions.
   - Log failures and important decisions.

## Example Rust Project Layout

```
my_project/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── flow.rs
│   ├── nodes.rs
│   └── utils/
│       ├── mod.rs
│       ├── llm_wrapper.rs
│       └── web_search.rs
└── docs/
    └── design.md
```

- **`Cargo.toml`**: add `pocketflow_rs`, `serde_json`, `anyhow`, `async-trait`, `tokio`, and `strum` as dependencies.
- **`docs/design.md`**: keep it high-level and Rust-oriented; do not copy Python pseudocode.
- **`src/utils/`**: one file per reusable integration is a good default.
- **`src/nodes.rs`**: node definitions should stay focused and readable.
- **`src/flow.rs`**: assemble the flow graph and state transitions.
- **State enums**: use `strum::Display` with `#[strum(serialize_all = "snake_case")]` for automatic state-to-string conversion.

## Before Finishing

- Check names, types, and examples against `src/lib.rs`, `src/node.rs`, and `src/flow.rs`.
- Remove any Python-only syntax or nonexistent APIs.
- Keep examples runnable or close to runnable against the current Rust API.
- Ensure state enums implement `ProcessState` and use `strum::Display` for edge matching.
