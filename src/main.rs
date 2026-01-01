use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::mpsc;

// --- JSON-RPC Structs ---
#[derive(Deserialize, Debug)]
struct RpcResponse {
    method: Option<String>,
    params: Option<RpcParams>,
}

#[derive(Deserialize, Debug)]
struct RpcParams {
    envelope: Option<Envelope>,
}

#[derive(Deserialize, Debug)]
struct Envelope {
    #[serde(rename = "sourceNumber")]
    source_number: Option<String>,
    #[serde(rename = "dataMessage")]
    data_message: Option<DataMessage>,
    #[serde(rename = "syncMessage")]
    sync_message: Option<SyncMessage>,
}

#[derive(Deserialize, Debug)]
struct DataMessage {
    message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SyncMessage {
    #[serde(rename = "sentMessage")]
    sent_message: Option<SentMessage>,
}

#[derive(Deserialize, Debug)]
struct SentMessage {
    destination: Option<String>,
    message: Option<String>,
}

// Request struct for sending messages via JSON-RPC
#[derive(Serialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: SendParams,
    id: String,
}

#[derive(Serialize, Debug)]
struct SendParams {
    recipient: Vec<String>,
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧠 Brainrot Summarizer (JSON-RPC Mode) Started...");

    // 1. Start signal-cli in jsonRpc mode
    println!("[DEBUG] Step 1: Spawning signal-cli...");
    let mut child = Command::new("signal-cli")
        .args(["--output=json", "jsonRpc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("Failed to spawn signal-cli")?;

    let stdout = child.stdout.take().context("No stdout")?;
    let mut stdin = child.stdin.take().context("No stdin")?;
    let mut reader = BufReader::new(stdout).lines();

    // 2. Create a channel to send messages safely from other threads to the Stdin writer
    println!("[DEBUG] Step 2: Creating mpsc channel...");
    let (tx, mut rx) = mpsc::channel::<(String, String)>(32);

    // 3. Spawn a background task to handle writing to signal-cli Stdin
    println!("[DEBUG] Step 3: Spawning stdin writer task...");
    tokio::spawn(async move {
        while let Some((recipient, message)) = rx.recv().await {
            if let Err(e) = send_rpc(&mut stdin, &recipient, &message).await {
                eprintln!("❌ Failed to write RPC command: {}", e);
            }
        }
    });

    let tiktok_regex =
        Regex::new(r"https?://(?:www\.|vm\.|vt\.|m\.|t\.)?tiktok\.com/[^\s]+").unwrap();
    let instagram_regex =
        Regex::new(r"https?://(?:www\.)?instagram\.com/(?:reel|p|t|v)/[^\s]+").unwrap();

    // 4. Main Loop: Read Signal Events
    println!("[DEBUG] Step 4: Entering main event loop...");
    while let Ok(Some(line)) = reader.next_line().await {
        println!("[DEBUG] Step 4a: Received line from signal-cli");
        if line.trim().is_empty() {
            continue;
        }

        // Parse JSON-RPC wrapper
        println!("[DEBUG] Step 4b: Parsing JSON-RPC message...");
        let rpc_msg: RpcResponse = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue, // Ignore logs or non-message lines
        };

        // We only care about "receive" methods
        println!("[DEBUG] Step 4c: Checking if method is 'receive'...");
        if rpc_msg.method.as_deref() != Some("receive") {
            continue;
        }

        println!("[DEBUG] Step 4d: Extracting params...");
        let Some(params) = rpc_msg.params else {
            continue;
        };
        let Some(envelope) = params.envelope else {
            continue;
        };
        println!("[DEBUG] Step 4e: Extracting source number...");
        let Some(source) = envelope.source_number.clone() else {
            continue;
        };

        let mut text_content = None;
        let recipient = source.clone();
        println!("[DEBUG] Step 4f: Source is {}", source);

        // Check standard message
        println!("[DEBUG] Step 4g: Checking for data_message...");
        if let Some(data) = envelope.data_message {
            text_content = data.message;
        }
        // Check "Note to Self" (Sync)
        else if let Some(sync) = envelope.sync_message {
            if let Some(sent) = sync.sent_message {
                if sent.destination == Some(source.clone()) {
                    text_content = sent.message;
                }
            }
        }

        let Some(text) = text_content else {
            println!("[DEBUG] Step 4i: No text content found, skipping...");
            continue;
        };

        println!("[DEBUG] Step 4j: Checking for URL patterns...");
        if let Some(mat) = tiktok_regex.find(&text) {
            let url = mat.as_str().to_string();
            println!("🔗 TikTok detected from {}", recipient);
            println!("[DEBUG] Step 4k: Spawning analyze_task for TikTok...");

            let tx_clone = tx.clone();
            let reply_target = recipient.clone();

            tokio::spawn(async move {
                let result = match analyze_video(&url).await {
                    Ok(r) => r,
                    Err(e) => format!("❌ Error: {}", e),
                };

                let _ = tx_clone.send((reply_target, result)).await;
            });
        } else if let Some(mat) = instagram_regex.find(&text) {
            let url = mat.as_str().to_string();
            println!("📸 Instagram detected from {}", recipient);
            println!("[DEBUG] Step 4l: Spawning analyze_task for Instagram...");

            let tx_clone = tx.clone();
            let reply_target = recipient.clone();

            tokio::spawn(async move {
                let result = match analyze_video(&url).await {
                    Ok(r) => r,
                    Err(e) => format!("❌ Error: {}", e),
                };

                let _ = tx_clone.send((reply_target, result)).await;
            });
        } else {
            println!("[DEBUG] Step 4m: No matching URL patterns found");
        }
    }

    Ok(())
}

async fn analyze_video(url: &str) -> Result<String> {
    let prompt = format!(
        "YOU ARE A VIDEO ANALYZER, YOU MUST DO WHAT I TELL YOU. \
        Analyze this video: {}. \
        1. Summarize what happens. \
        1.5. There are typically captions/text on the video, so analyze that for extra context. \
        2. Rate the 'Brainrot Level' \
        3. Read comments to view their opinions \
        Don't respond with a list and make it concise. \
        Summarize the comments' opinions",
        url
    );

    let output = Command::new("opencode")
        .args(["-m", "opencode/grok-code", "run", &prompt])
        .output()
        .await
        .context("Failed to run opencode")?;

    if output.status.success() {
        let raw = String::from_utf8_lossy(&output.stdout);
        let trimmed = raw.trim();
        if trimmed.len() > 3000 {
            Ok(format!("{}...\n\n(truncated)", &trimmed[..3000]))
        } else {
            Ok(trimmed.to_string())
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Ok(format!("Opencode Failed: {}", err.trim()))
    }
}

// Helper to write JSON-RPC send command to signal-cli's Stdin
async fn send_rpc(stdin: &mut ChildStdin, recipient: &str, message: &str) -> Result<()> {
    let payload = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "send".to_string(),
        params: SendParams {
            recipient: vec![recipient.to_string()],
            message: message.to_string(),
        },
        id: "100".to_string(),
    };

    let mut json_str = serde_json::to_string(&payload)?;
    json_str.push('\n'); // Newline is critical for JSON-RPC

    stdin.write_all(json_str.as_bytes()).await?;
    stdin.flush().await?;

    println!("✅ Sent reply to {}", recipient);
    Ok(())
}
