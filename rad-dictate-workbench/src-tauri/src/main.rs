use std::{fs, io::{BufRead, BufReader, BufWriter, Write}, path::{Path, PathBuf}, process::{Command, Stdio}, sync::Arc, time::{Duration, SystemTime}};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

const SAMPLE_RATE: u32 = 16_000;
const CONTEXT_SECONDS: f64 = 1.5;

struct Recording { stream: cpal::Stream, writer: Arc<Mutex<Option<hound::WavWriter<BufWriter<fs::File>>>>>, audio_path: PathBuf }
struct AppState { session_dir: PathBuf, recording: Mutex<Option<Recording>> }
#[derive(Deserialize)] #[serde(rename_all = "camelCase")]
struct WordRange { start_seconds: f64, end_seconds: f64 }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
struct Word { text: String, start_seconds: f64, end_seconds: f64 }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
struct TranscriptResult { text: String, words: Vec<Word> }
#[derive(Deserialize)] #[serde(rename_all = "camelCase")]
struct SidecarWord { text: String, start_seconds: f64, end_seconds: f64 }
#[derive(Deserialize)] #[serde(rename_all = "camelCase")]
struct SidecarResult { text: String, words: Vec<SidecarWord> }

fn app_temp_root() -> PathBuf { std::env::temp_dir().join("rad-dictate-workbench") }
fn session_dir() -> PathBuf { app_temp_root().join(Uuid::new_v4().to_string()) }
fn cleanup_orphans() {
    let root = app_temp_root();
    let cutoff = SystemTime::now().checked_sub(Duration::from_secs(24 * 60 * 60)).unwrap();
    if let Ok(entries) = fs::read_dir(root) { for entry in entries.flatten() { if entry.path().is_dir() && entry.metadata().and_then(|m| m.modified()).map(|t| t < cutoff).unwrap_or(false) { let _ = fs::remove_dir_all(entry.path()); } } }
}
fn write_i16(writer: &Arc<Mutex<Option<hound::WavWriter<BufWriter<fs::File>>>>>, samples: impl Iterator<Item = i16>) { if let Some(w) = writer.lock().as_mut() { for sample in samples { let _ = w.write_sample(sample); } } }
fn create_stream(writer: Arc<Mutex<Option<hound::WavWriter<BufWriter<fs::File>>>>>) -> Result<cpal::Stream, String> {
    let device = cpal::default_host().default_input_device().ok_or("No default microphone is available.")?;
    let config = device.supported_input_configs().map_err(|e| e.to_string())?.find_map(|range| {
        if range.channels() == 1 && range.min_sample_rate().0 <= SAMPLE_RATE && range.max_sample_rate().0 >= SAMPLE_RATE { Some(range.with_sample_rate(cpal::SampleRate(SAMPLE_RATE))) } else { None }
    }).ok_or("The selected microphone does not support native 16 kHz mono capture.")?;
    let stream_config: cpal::StreamConfig = config.clone().into();
    let err = |e| eprintln!("microphone error: {e}");
    match config.sample_format() {
        cpal::SampleFormat::I16 => device.build_input_stream(&stream_config, move |d: &[i16], _| write_i16(&writer, d.iter().copied()), err, None),
        cpal::SampleFormat::U16 => device.build_input_stream(&stream_config, move |d: &[u16], _| write_i16(&writer, d.iter().map(|v| (v as i32 - 32768) as i16)), err, None),
        cpal::SampleFormat::F32 => device.build_input_stream(&stream_config, move |d: &[f32], _| write_i16(&writer, d.iter().map(|v| (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)), err, None),
        _ => return Err("Unsupported microphone sample format.".into()),
    }.map_err(|e| e.to_string())
}
#[tauri::command]
fn start_recording(state: State<AppState>, _microphone: String) -> Result<(), String> {
    if state.recording.lock().is_some() { return Err("Recording is already active.".into()); }
    fs::create_dir_all(&state.session_dir).map_err(|e| e.to_string())?;
    let audio_path = state.session_dir.join(format!("{}.wav", Uuid::new_v4()));
    let file = fs::File::create(&audio_path).map_err(|e| e.to_string())?;
    let spec = hound::WavSpec { channels: 1, sample_rate: SAMPLE_RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let writer = Arc::new(Mutex::new(Some(hound::WavWriter::new(BufWriter::new(file), spec).map_err(|e| e.to_string())?)));
    let stream = create_stream(writer.clone())?; stream.play().map_err(|e| e.to_string())?;
    *state.recording.lock() = Some(Recording { stream, writer, audio_path }); Ok(())
}
fn run_sidecar(audio_path: &Path) -> Result<TranscriptResult, String> {
    let python = std::env::var("RAD_DICTATE_PYTHON").map_err(|_| "RAD_DICTATE_PYTHON is not configured. Run dev.cmd.".to_string())?;
    let script = std::env::var("RAD_DICTATE_SIDECAR").map_err(|_| "RAD_DICTATE_SIDECAR is not configured. Run dev.cmd.".to_string())?;
    let model = std::env::var("RAD_DICTATE_MODEL_PATH").map_err(|_| "RAD_DICTATE_MODEL_PATH is not configured. Run dev.cmd.".to_string())?;
    let mut child = Command::new(python).arg(script).env("RAD_DICTATE_MODEL_PATH", model).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
    let request = serde_json::json!({"type":"transcribe","requestId":Uuid::new_v4().to_string(),"audioPath":audio_path});
    child.stdin.as_mut().ok_or("Could not open sidecar input.")?.write_all(format!("{request}\n").as_bytes()).map_err(|e| e.to_string())?;
    let output = child.stdout.take().ok_or("Could not open sidecar output.")?; let reader = BufReader::new(output);
    for line in reader.lines().flatten() { let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| e.to_string())?; match value.get("type").and_then(|v| v.as_str()) { Some("result") => { let result: SidecarResult = serde_json::from_value(value).map_err(|e| e.to_string())?; let _ = child.kill(); return Ok(TranscriptResult { text: result.text, words: result.words.into_iter().map(|w| Word { text:w.text, start_seconds:w.start_seconds, end_seconds:w.end_seconds }).collect() }); }, Some("error") => return Err(value["message"].as_str().unwrap_or("Sidecar failed.").to_string()), _ => {} } }
    Err("Sidecar exited without a result.".into())
}
#[tauri::command]
fn stop_and_transcribe(state: State<AppState>) -> Result<TranscriptResult, String> {
    let recording = state.recording.lock().take().ok_or("No active recording.")?; drop(recording.stream);
    if let Some(writer) = recording.writer.lock().take() { writer.finalize().map_err(|e| e.to_string())?; }
    run_sidecar(&recording.audio_path)
}
#[tauri::command]
fn word_audio(state: State<AppState>, range: WordRange) -> Result<Vec<u8>, String> {
    let audio = fs::read_dir(&state.session_dir).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e == "wav")).ok_or("No session audio available.")?;
    let mut reader = hound::WavReader::open(audio).map_err(|e| e.to_string())?; let spec = reader.spec();
    let start = ((range.start_seconds - CONTEXT_SECONDS).max(0.0) * spec.sample_rate as f64) as usize;
    let end = ((range.end_seconds + CONTEXT_SECONDS).max(0.0) * spec.sample_rate as f64) as usize;
    let mut cursor = std::io::Cursor::new(Vec::new()); { let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|e| e.to_string())?; for (i, sample) in reader.samples::<i16>().enumerate() { if i >= end { break; } if i >= start { writer.write_sample(sample.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?; } } writer.finalize().map_err(|e| e.to_string())?; }
    Ok(cursor.into_inner())
}
#[tauri::command]
fn clear_session(state: State<AppState>) -> Result<(), String> { if state.recording.lock().is_some() { return Err("Stop recording before clearing the session.".into()); } if state.session_dir.exists() { fs::remove_dir_all(&state.session_dir).map_err(|e| e.to_string())?; } fs::create_dir_all(&state.session_dir).map_err(|e| e.to_string()) }
fn main() { cleanup_orphans(); let dir = session_dir(); let _ = fs::create_dir_all(&dir); tauri::Builder::default().manage(AppState { session_dir: dir, recording: Mutex::new(None) }).invoke_handler(tauri::generate_handler![start_recording, stop_and_transcribe, word_audio, clear_session]).run(tauri::generate_context!()).expect("tauri error"); }
