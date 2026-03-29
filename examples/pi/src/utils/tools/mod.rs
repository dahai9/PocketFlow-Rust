// use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn read_file(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => format!("Error reading file: {}", e),
    }
}

pub fn write_file(path: &str, content: &str) -> String {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::write(p, content) {
        Ok(_) => format!("Successfully wrote to {}", path),
        Err(e) => format!("Error writing file: {}", e),
    }
}

pub fn execute_bash(command: &str, cwd: &str) -> String {
    let output = Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .output();

    match output {
        Ok(out) => {
            let mut result = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if !stderr.is_empty() {
                result.push_str("\n--- STDERR ---\n");
                result.push_str(&stderr);
            }
            if result.is_empty() {
                "Command executed successfully with no output.".to_string()
            } else {
                result
            }
        }
        Err(e) => format!("Error executing command: {}", e),
    }
}
