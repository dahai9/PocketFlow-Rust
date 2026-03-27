use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

pub struct SessionManager {
    log_path: PathBuf,
}

impl SessionManager {
    pub fn new(workspace: &Path) -> Self {
        let mut log_path = workspace.to_path_buf();
        log_path.push(".pi");
        log_path.push("logs");
        if !log_path.exists() {
            std::fs::create_dir_all(&log_path).unwrap_or_default();
        }
        log_path.push("log.jsonl");
        Self { log_path }
    }

    pub fn append_message(&self, message: &AgentMessage) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        let json = serde_json::to_string(message)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn load_history(&self, head_id: Option<&str>) -> anyhow::Result<Vec<AgentMessage>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        
        let mut messages = Vec::new();
        for line in reader.lines() {
            if let Ok(l) = line {
                if let Ok(msg) = serde_json::from_str::<AgentMessage>(&l) {
                    messages.push(msg);
                }
            }
        }
        
        // Simple linear history for now.
        // In a full implementation, we would rebuild the tree using parent_id up to head_id.
        Ok(messages)
    }
}
