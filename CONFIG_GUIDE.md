# PocketFlow-RS 配置指南

本项目使用 `config.toml` 管理模型、API 提供商和通用设置。配置文件位于工作区下的 `./.pi/config.toml`。

## 文件位置
- **默认路径**：`./.pi/config.toml`
- **日志路径**：`./.pi/logs/log.jsonl`

> 提示：当你首次运行 `pi` 命令时，若配置不存在，会自动创建默认配置。

## 配置结构

```toml
[general]
auto_compact = true

[providers.openai]
api_base = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"

[providers.cerebras]
api_base = "http://127.0.0.1:4000/v1"
api_key_env = "OPENAI_API_KEY"

[models."gpt-4o"]
provider = "openai"
context_window = 128000
compact_threshold = 100000

[models."cerebras/qwen-3-235b-a22b-instruct-2507"]
provider = "cerebras"
context_window = 8192
compact_threshold = 6000
```

### 字段说明

| 区域 | 字段 | 说明 |
|------|------|------|
| `general` | `auto_compact` | 是否开启自动历史压缩（摘要旧消息以节省上下文） |
| `providers` | `api_base` | 模型提供商的 API 地址 |
| `providers` | `api_key_env` | 读取 API 密钥的环境变量名 |
| `models` | `provider` | 该模型使用的提供商（需匹配上方 `providers` 中的键） |
| `models` | `context_window` | 模型最大上下文长度（token 数） |
| `models` | `compact_threshold` | 超过此长度时触发自动压缩 |

## 自定义配置示例

你可以添加本地大模型支持，例如：

```toml
[providers.localhost]
api_base = "http://127.0.0.1:8080/v1"
api_key_env = "LOCAL_API_KEY"

[models."qwen:14b"]
provider = "localhost"
context_window = 32768
compact_threshold = 24000
```

然后运行：
```bash
pi --model qwen:14b --provider localhost
```

---

📌 提示：确保相应 `api_key_env` 环境变量已设置，例如：
```bash
export OPENAI_API_KEY=sk-...
```
