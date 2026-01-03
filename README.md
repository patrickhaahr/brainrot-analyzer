# Brainrot Summarizer 🧠

Get an AI summary of the brainrot your friends send you on Signal.

This bot runs as a background service that listens for TikTok and Instagram links in Signal messages. When it detects one, it:

- Downloads the video and extracts subtitles
- Frames the video for visual analysis  
- Uses whisper to get the transcribe of the audio
- Uses AI to summarize:
  - What happens in the video
  - Sentiment and opinions expressed
  - "Brainrot Level" rating (1-10)

- Sends a concise summary back to the sender

**Built with**: Rust + signal-cli + yt-dlp + ffmpeg + whisper + opencode

Perfect for when your friends send you brainrot you're too lazy to watch or in my case have both tiktok and instagram blocked from at network level (adguard). Created this project since they keep sending me brainrot and thought of this idea.

---

## todo:
dockerfile, docker compose
easy model switch - .env 
prompt switch? - .env
proper readme guide:
  * prereqs: rust, signal-lib, opencode (auth login), signal-cli (register or link), java-jre21, openai-whisper, yt-dlp, ffmpeg
  * installation guide

remove debug logs

make feature to send signal message always to note to self rather than the recipent - toggle?

github action - automate release binary file 

