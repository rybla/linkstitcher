use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg("gemini -p 'What is your model name?'")
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stdout: {stdout}");
    println!("stderr: {stderr}");
    Ok(())
}
