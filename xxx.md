# 计划: 实现 pi-mono 会话压缩 (Compaction) 与 TOML 配置支持

## Context (背景与目标)
目前系统已打通核心交互循环和会话持久化。随着对话的增加，每次请求附带的历史上下文会不断变长，最终超出 LLM 模型的上下文窗口（Context Window）。
我们需要引入会话压缩（Compaction）机制，并在 `pi` 启动时加载一个 `config.toml` 文件，用于分类配置是否启用自动压缩、每种模型的窗口大小约束、以及对应的大模型服务提供商 (Provider) 设定。

由于安全设定约束，**在计划模式下，系统严格禁止直接执行代码提交（`git commit`）等修改系统状态的操作。** 我将在获得您对本计划的批准进入执行模式后，**第一步便为您执行代码提交**，然后再进行后续的代码修改。

## Proposed Changes (架构与组件设计)

### 1. 配置管理设计 (`src/config.rs`)
通过引入 `toml` 和 `serde` 读取全局或局部配置文件（如 `config.toml`）。设计配置类结构如下：

```rust
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub providers: HashMap<String, ProviderConfig>,
    pub models: HashMap<String, ModelConfig>,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    pub auto_compact: bool,       // 是否开启自动压缩
}

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    pub api_base: String,
    pub api_key_env: String,      // 指定从哪个环境变量读取 Key，提高安全性
}

#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    pub provider: String,         // 关联的 Provider 名称
    pub context_window: usize,    // 模型的绝对最大窗口 (按 Token 数，粗略可用字数/4近似)
    pub compact_threshold: usize, // 触发压缩的阈值 (如超过 80% 则压缩)
}
```

### 2. Append-Only 会话日志的压缩实现 (`src/utils/session_manager.rs`)
保留 `pi-mono` 的 Append-Only JSONL 特性，不直接修改旧日志文件。我们在 `AgentMessage` 中新增一个特殊字段：
- `clears_history: Option<bool>`

当发生压缩时，系统将先前的对话交由 LLM 生成摘要，并写入一条 `role: "system"`, `content: "Previous conversation summary: ..."` 且附带 `clears_history: true` 的新记录。
在重启应用并调用 `load_history` 恢复列表时，如果读到 `clears_history == Some(true)` 的消息，就将内存中的 `messages` 清空（或保留最初的系统设定），只保留这条 Summary 继续往下构建，完成完美的持久化无损截断。

### 3. PiLLM 动态路由支持 (`src/utils/pi_llm.rs`)
修改 `PiLLM` 内部逻辑：
不再硬编码读取 `OPENAI_API_KEY`，而是根据当前的选项，从 `AppConfig` 查找到对应的 `ModelConfig`，接着查找到 `ProviderConfig`，使用对应的 `api_base` 和对应的环境变量（`api_key_env`）进行鉴权与请求分发。

### 4. 压缩节点注入 (`src/bin/pi.rs`)
- 在 `LLMReasoningNode` 的 `execute` 开始前加入 Token 容量估算逻辑（如按字符串长度评估或引入 tiktoken 库）。
- 若超过 `model.compact_threshold` 且 `auto_compact` 为 true，即刻在此流程内部或单独生成一个阻塞调用，请求模型生成摘要（"Please summarize the history conversation concisely..."）。
- 成功获得摘要后，实例化一段 `clears_history=true` 的 `AgentMessage` 调用 `append_message` 持久化，重置 Context 中的 `messages`，接着再处理用户当下真实的发问。

### 5. `Cargo.toml` 依赖更新
添加 `toml` crates 支持。

## Verification Plan (验证计划)
### Automated Tests
1. 编写对 `AppConfig` TOML 解析的单元测试。
2. 针对包含 `clears_history: true` 的模拟 `.jsonl` 文件编写 `SessionManager::load_history` 测试，断言数组应当短路清空，仅保留之后的有效长度。

### Manual Verification
1. **代码提交流程验证**：执行模式开启后的第一件事就是运行 `git commit -am "chore: initial working implementation before compaction"`，证明我们遵守了指令。
2. 启动代理程序加载携带多 `models` 和 `providers` 的 `config.toml`。
3. 把某配置大模型的 `compact_threshold` 改为非常小的值（如 `20`）。
4. 对话几次，让长度超过该极小阈值，观察控制台是否输出了 "[Auto Compacting History...]" 提示。
5. 通过查看底层 `log.jsonl`，证实末尾产生了一条标志性的含有 `clears_history: true` 的 Json 行。使用 `cargo run` 重启程序，核实历史条数（`Loaded X messages`）是否明显变少（因为老信息已被抛弃截断）。
