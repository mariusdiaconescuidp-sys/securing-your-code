import { useRef, useState } from "react";
import "./App.css";
type Word = { text: string; startSeconds: number; endSeconds: number };
type Result = { text: string; words: Word[] };
const invoke = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
  const tauri = (window as Window & { __TAURI__?: { core: { invoke: <R>(c: string, a?: Record<string, unknown>) => Promise<R> } } }).__TAURI__;
  if (!tauri) throw new Error("Run this app with npm run tauri dev to use microphone capture.");
  return tauri.core.invoke<T>(command, args);
};
export default function App() {
  const [recording, setRecording] = useState(false), [transcript, setTranscript] = useState(""), [words, setWords] = useState<Word[]>([]), [status, setStatus] = useState("Ready. Audio and transcripts stay local to this session."), [microphone, setMicrophone] = useState("Default microphone");
  const textRef = useRef<HTMLTextAreaElement>(null);
  const insert = (text: string) => { const f = textRef.current; if (!f) return setTranscript((v) => v + text); setTranscript((v) => v.slice(0, f.selectionStart) + text + v.slice(f.selectionEnd)); };
  const begin = async () => { try { await invoke("start_recording", { microphone }); setRecording(true); setStatus("Recording — release to transcribe."); } catch (e) { setStatus(e instanceof Error ? e.message : "Could not start recording."); } };
  const end = async () => { if (!recording) return; setRecording(false); setStatus("Transcribing locally…"); try { const r = await invoke<Result>("stop_and_transcribe"); insert(r.text); setWords(r.words); setStatus("Transcript inserted. Select a timestamped word to replay context."); } catch (e) { setStatus(e instanceof Error ? e.message : "Transcription failed."); } };
  const clear = async () => { await invoke("clear_session").catch(() => undefined); setTranscript(""); setWords([]); setStatus("Session cleared. Temporary audio was removed."); };
  const replay = async (w: Word) => { try { const bytes = await invoke<number[]>("word_audio", { startSeconds: w.startSeconds, endSeconds: w.endSeconds }); await new Audio(URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: "audio/wav" }))).play(); } catch { setStatus("No replay audio is available for this word."); } };
  return <main><header><p className="eyebrow">LOCAL-ONLY • PHASE 0</p><h1>Rad Dictate Workbench</h1><p>{status}</p></header><section className="controls"><label>Microphone<select value={microphone} onChange={(e) => setMicrophone(e.target.value)}><option>Default microphone</option></select></label><button className={recording ? "recording" : "record"} onPointerDown={begin} onPointerUp={end} onPointerLeave={end}>{recording ? "● Recording" : "Hold to dictate"}</button><button className="clear" onClick={clear}>Clear Session</button></section><section><label htmlFor="transcript">Editable transcript</label><textarea id="transcript" ref={textRef} value={transcript} onChange={(e) => setTranscript(e.target.value)} placeholder="Dictated text appears here…" /></section><section><h2>Replay a word</h2><div className="words">{words.length ? words.map((w, i) => <button key={`${w.startSeconds}-${i}`} onClick={() => replay(w)}>{w.text}</button>) : <span>No timestamps yet.</span>}</div></section></main>;
}
