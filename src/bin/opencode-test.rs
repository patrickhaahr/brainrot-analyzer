use std::process::Command;

fn main() {
    println!("Testing opencode with TikTok analysis prompt...");

    let prompt = "Analyze this TikTok video: https://example.com. 1. Summarize what happens. 2. Is it funny? 3. Rate the Brainrot Level from 1 to 10.";

    let child = Command::new("opencode")
        .args(["-m", "opencode/grok-code", "run", prompt])
        .output()
        .expect("Failed to execute opencode command");

    println!("Exit status: {}", child.status);

    if !child.stdout.is_empty() {
        println!("\n--- STDOUT ---");
        println!("{}", String::from_utf8_lossy(&child.stdout));
    }

    if !child.stderr.is_empty() {
        println!("\n--- STDERR ---");
        println!("{}", String::from_utf8_lossy(&child.stderr));
    }

    if child.status.success() {
        println!("\n✅ Opencode is working!");
    } else {
        println!("\n❌ Opencode failed with exit code: {:?}", child.status.code());
    }
}
