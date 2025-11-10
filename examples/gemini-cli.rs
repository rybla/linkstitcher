use anyhow::Result;
use linkstitcher::utility;

#[tokio::main]
async fn main() -> Result<()> {
    let prompt = "What is your model name?";
    println!("prompt: {prompt}");
    let output = utility::ai::gemini_cli(prompt)?;
    println!("output: {output}");
    Ok(())
}
