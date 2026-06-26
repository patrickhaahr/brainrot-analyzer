# Brainrot Summarizer Hermes Plugin

This is a Hermes general plugin that detects TikTok and Instagram links in gateway messages and rewrites them into a tool-backed video analysis request.

The `brainrot_analyze_video` tool:

- downloads the video and available subtitles with `yt-dlp`
- extracts representative frames with `ffmpeg`
- falls back to `whisper` when subtitles are unavailable
- uses Hermes plugin LLM access to summarize the video, sentiment, and Brainrot Level

## Install

Copy or mount this directory into Hermes' user plugin directory:

```text
~/.hermes/plugins/brainrot-summarizer
```

Then enable it in `~/.hermes/config.yaml`:

```yaml
plugins:
  enabled:
    - brainrot-summarizer
```

The Hermes runtime must have `yt-dlp`, `ffmpeg`, and `whisper` available on `PATH`.

## Configuration

Optional environment variables:

- `BRAINROT_GATEWAY_PLATFORMS`: comma-separated gateway platform names to intercept, default `signal`
- `BRAINROT_SUB_LANGS`: subtitle languages passed to `yt-dlp`, default `en.*,en`
- `BRAINROT_FRAME_COUNT`: maximum frames sent to the LLM, default `8`
- `BRAINROT_FRAME_INTERVAL_SECONDS`: frame sampling interval, default `5`
- `BRAINROT_FRAME_WIDTH`: extracted frame width, default `640`
- `BRAINROT_WHISPER_COMMAND`: Whisper command name, default `whisper`
- `BRAINROT_WHISPER_MODEL`: Whisper model, default `tiny`
- `BRAINROT_MAX_TOKENS`: LLM summary token cap, default `700`
- `BRAINROT_LLM_TIMEOUT_SECONDS`: LLM timeout, default `120`
