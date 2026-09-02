# Rad Dictate Workbench — Phase 0

Local-only Windows dictation prototype. It captures only to a UUID session folder below `%TEMP%\rad-dictate-workbench`, sends a completed 16 kHz mono WAV to a Python `faster-whisper` CPU sidecar, and removes session audio on Clear or normal shutdown. It has no telemetry, database, cloud transcription, PACS/RIS/EHR connection, or clinical decision support.

## Developer launch

1. Ensure the verified model is at `%LOCALAPPDATA%\RadDictateWorkbench\models\large-v3`.
2. Double-click `dev.cmd`. It supplies the local Python, sidecar, and model paths, then launches `npm run tauri dev`.
3. Hold **Hold to dictate**, release it, then edit the inserted transcript or click a timestamped word to replay its surrounding audio.

## Offline check

Disconnect networking after the model is installed, start with `dev.cmd`, and dictate a short phrase. The sidecar requires a local directory path and never accepts a model name, preventing automatic model downloads. Verify no application process has an external connection with `netstat -ano`.

## Test checklist

- [ ] Native microphone exposes a 16 kHz mono configuration.
- [ ] Hold/release records and inserts a local transcript.
- [ ] Word replay works near the start and end of an utterance.
- [ ] Clear Session removes the session WAV files.
- [ ] Restart removes abandoned session folders older than 24 hours.
- [ ] Forced-close leaves no sidecar Python process.

## Known Phase 0 limits

The microphone selector currently uses Windows' default input device. The developer workflow invokes the Python source sidecar; packaging that sidecar as an executable is a Phase 1 packaging step.
