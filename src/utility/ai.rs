use anyhow::{Result, anyhow};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Gemini CLI error")]
struct GeminiCliError {
    stderr: String,
}

pub fn gemini_cli(prompt: &str) -> Result<String> {
    let escaped_prompt = prompt
        .chars()
        .flat_map(|c| c.escape_default())
        .collect::<String>();
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("gemini -p \"{escaped_prompt}\""))
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if stderr.contains("Error") || stderr.contains("error") {
        return Err(anyhow!(GeminiCliError { stderr }));
    }

    Ok(stdout)
}
