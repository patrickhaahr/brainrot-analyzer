use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
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
    #[serde(rename = "sourceUuid")]
    source_uuid: Option<String>,
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
    println!("[DEBUG] Entering main event loop, waiting for messages...");
    while let Ok(Some(line)) = reader.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        // Debug: Print raw JSON (truncated for readability)
        let preview = if line.len() > 300 {
            &line[..300]
        } else {
            &line
        };
        println!("[DEBUG] Raw JSON: {}...", preview);

        // Parse JSON-RPC wrapper
        let rpc_msg: RpcResponse = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                if line.trim().starts_with('{') {
                    println!("[DEBUG] JSON parse error: {}", e);
                }
                continue;
            }
        };

        // We only care about "receive" methods
        let method = rpc_msg.method.as_deref();
        if method != Some("receive") {
            println!("[DEBUG] Skipping method: {:?}", method);
            continue;
        }

        let Some(params) = rpc_msg.params else {
            println!("[DEBUG] No params in message");
            continue;
        };
        let Some(envelope) = params.envelope else {
            println!("[DEBUG] No envelope in params");
            continue;
        };

        // Get source identifier - prefer phone number, fallback to UUID
        let source = envelope
            .source_number
            .clone()
            .or_else(|| envelope.source_uuid.clone());

        let Some(source) = source else {
            println!("[DEBUG] No sourceNumber or sourceUuid in envelope");
            continue;
        };

        println!("[DEBUG] Message from source: {}", source);
        println!(
            "[DEBUG] Has dataMessage: {}",
            envelope.data_message.is_some()
        );
        println!(
            "[DEBUG] Has syncMessage: {}",
            envelope.sync_message.is_some()
        );

        let mut text_content = None;
        let recipient = source.clone();

        // Check standard message (from others)
        if let Some(ref data) = envelope.data_message {
            println!("[DEBUG] dataMessage.message: {:?}", data.message);
            text_content = data.message.clone();
        }
        // Check "Note to Self" (Sync)
        else if let Some(ref sync) = envelope.sync_message {
            if let Some(ref sent) = sync.sent_message {
                println!(
                    "[DEBUG] syncMessage.sentMessage.destination: {:?}",
                    sent.destination
                );
                println!(
                    "[DEBUG] syncMessage.sentMessage.message: {:?}",
                    sent.message
                );
                if sent.destination == Some(source.clone()) {
                    text_content = sent.message.clone();
                }
            } else {
                println!("[DEBUG] syncMessage has no sentMessage");
            }
        }

        let Some(text) = text_content else {
            println!("[DEBUG] No text content extracted, skipping");
            continue;
        };

        println!("[DEBUG] Extracted text: {}", &text[..text.len().min(100)]);

        if let Some(mat) = tiktok_regex.find(&text) {
            let url = mat.as_str().to_string();
            println!("🔗 TikTok detected from {}", recipient);
            println!("[DEBUG] Step 4k: Spawning analyze_task for TikTok...");

            let tx_clone = tx.clone();
            let reply_target = recipient.clone();

            tokio::spawn(async move {
                match analyze_video(&url).await {
                    Ok(result) => {
                        let _ = tx_clone.send((reply_target, result)).await;
                    }
                    Err(e) => {
                        eprintln!("❌ Error processing TikTok from {}: {}", reply_target, e);
                    }
                }
            });
        } else if let Some(mat) = instagram_regex.find(&text) {
            let url = mat.as_str().to_string();
            println!("📸 Instagram detected from {}", recipient);
            println!("[DEBUG] Step 4l: Spawning analyze_task for Instagram...");

            let tx_clone = tx.clone();
            let reply_target = recipient.clone();

            tokio::spawn(async move {
                match analyze_video(&url).await {
                    Ok(result) => {
                        let _ = tx_clone.send((reply_target, result)).await;
                    }
                    Err(e) => {
                        eprintln!("❌ Error processing Instagram from {}: {}", reply_target, e);
                    }
                }
            });
        } else {
            println!("[DEBUG] Step 4m: No matching URL patterns found");
        }
    }

    Ok(())
}

async fn analyze_video(url: &str) -> Result<String> {
    let temp_dir = PathBuf::from("/tmp/brainrot_summarizer");

    // Clean up previous run if exists, then create fresh directories
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }
    fs::create_dir_all(&temp_dir).context("Failed to create temp dir")?;

    let subs_dir = temp_dir.join("subs");
    fs::create_dir_all(&subs_dir).context("Failed to create subs dir")?;

    println!("[DEBUG] Downloading video...");
    let video_path = download_video_and_subs(url, &temp_dir, &subs_dir).await?;

    println!("[DEBUG] Extracting frames...");
    extract_frames(&temp_dir, &video_path).await?;

    println!("[DEBUG] Running Opencode analysis...");
    let prompt = "You are a video analyzer. \
        The current directory contains a video processed into: \
        - 'frames/' directory containing extracted frames (frame_001.jpg, etc) \
        - 'subs/' directory containing subtitle files (if available) \
        \
        Analyze the content based on these files. \
        1. Summarize what happens. Include Text and captions for important context \
        2. Summarize the sentiment/opinions expressed. \
        3. Rate the 'Brainrot Level' (1-10). \
        Natural formatting, no '*', keep Headings. START output from 'Summary' \
        Keep your response CONCISE \
        No more than 3 sentances of Summary \
        No more than 2 sentances of sentiment and opinions \
        No more than 1 sentance of brainrot level";

    let output = Command::new("opencode")
        .current_dir(&temp_dir)
        .args(["-m", "opencode/gemini-3-flash", "run", prompt])
        .output()
        .await
        .context("Failed to run opencode")?;

    // Cleanup is optional here depending on if we want to debug,
    // but the next run cleans up at the start anyway.
    // fs::remove_dir_all(&temp_dir)?;

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

async fn extract_frames(work_dir: &PathBuf, video_path: &PathBuf) -> Result<()> {
    let frames_dir = work_dir.join("frames");
    fs::create_dir_all(&frames_dir).context("Failed to create frames directory")?;

    let output = Command::new("ffmpeg")
        .current_dir(work_dir)
        .args([
            "-i",
            video_path.to_str().unwrap(),
            "-vf",
            "fps=0.5",
            "frames/frame_%03d.jpg",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to run ffmpeg")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("ffmpeg failed: {}", stderr));
    }

    Ok(())
}

async fn download_video_and_subs(
    url: &str,
    work_dir: &PathBuf,
    subs_dir: &PathBuf,
) -> Result<PathBuf> {
    let output = Command::new("yt-dlp")
        .current_dir(work_dir)
        .args(&[
            "-o",
            "video.%(ext)s", // Explicitly name it video.ext
            "--write-subs",
            "--write-auto-subs",
            "--sub-lang",
            "en",
            "--sub-format",
            "vtt",
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to run yt-dlp")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("yt-dlp failed: {}", stderr));
    }

    let mut video_path = None;
    let mut found_subs = false;

    // Move any .vtt files to the subs directory and find the video file
    let read_dir = fs::read_dir(work_dir)?;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "vtt" {
                let file_name = path.file_name().unwrap();
                let dest = subs_dir.join(file_name);
                fs::rename(path, dest)?;
                found_subs = true;
            } else if let Some(stem) = path.file_stem() {
                // If the file is named "video" and it's not a subtitle file, assume it's the video
                if stem == "video" {
                    video_path = Some(path);
                }
            }
        }
    }

    let video_path =
        video_path.ok_or_else(|| anyhow::anyhow!("Could not find downloaded video file"))?;

    if !found_subs {
        println!("[DEBUG] No subtitles found by yt-dlp. Running Whisper fallback...");
        let output = Command::new("whisper")
            .arg(video_path.to_str().unwrap())
            .arg("--model")
            .arg("tiny")
            .arg("--output_format")
            .arg("vtt")
            .arg("--output_dir")
            .arg(subs_dir.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run whisper")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Whisper failed: {}", stderr);
            // We proceed without subtitles rather than failing the whole download
        } else {
            println!("[DEBUG] Whisper generated subtitles successfully.");
        }
    }

    Ok(video_path)
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
