"""Line-delimited JSON transcription sidecar. It never downloads a model."""
import json, os, sys
from pathlib import Path
from faster_whisper import WhisperModel

model = None
def respond(value):
    print(json.dumps(value, ensure_ascii=False), flush=True)
def get_model():
    global model
    if model is None:
        model_path = Path(os.environ.get("RAD_DICTATE_MODEL_PATH", ""))
        if not model_path.is_dir():
            raise RuntimeError("RAD_DICTATE_MODEL_PATH must name a preinstalled local model directory")
        model = WhisperModel(str(model_path), device="cpu", compute_type="int8", cpu_threads=4)
    return model
for raw in sys.stdin:
    try:
        request = json.loads(raw)
        if request.get("type") == "shutdown": break
        if request.get("type") != "transcribe": raise ValueError("unsupported request")
        request_id, audio = request["requestId"], Path(request["audioPath"])
        if not audio.is_file() or audio.suffix.lower() != ".wav": raise ValueError("audioPath must be a readable WAV")
        respond({"type":"status", "requestId":request_id, "state":"processing"})
        segments, info = get_model().transcribe(str(audio), language="en", word_timestamps=True, condition_on_previous_text=False)
        words, text = [], []
        for segment in segments:
            text.append(segment.text)
            words.extend({"text":word.word, "startSeconds":word.start, "endSeconds":word.end} for word in (segment.words or []))
        respond({"type":"result", "requestId":request_id, "text":"".join(text).strip(), "language":info.language, "words":words})
    except Exception as error:
        respond({"type":"error", "requestId":locals().get("request_id", "unknown"), "code":"TRANSCRIPTION_FAILED", "message":str(error)})
