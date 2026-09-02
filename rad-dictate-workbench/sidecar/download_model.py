import os
from pathlib import Path
from huggingface_hub import snapshot_download

model_dir = Path(os.environ["LOCALAPPDATA"]) / "RadDictateWorkbench" / "models" / "large-v3"
print(snapshot_download(repo_id="Systran/faster-whisper-large-v3", local_dir=model_dir), flush=True)
