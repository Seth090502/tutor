//! Speech for the Languages world (contract C10): ElevenLabs text to speech with an on-disk
//! cache, the voices for a language, and Scribe speech to text. Every call uses the stored key,
//! which never reaches the webview; errors are readable and never contain it.
//!
//! API references (read 2026-09-30):
//! - Text to speech, https://elevenlabs.io/docs/api-reference/text-to-speech/convert:
//!   `POST /v1/text-to-speech/{voice_id}?output_format=mp3_44100_128` with `xi-api-key` and JSON
//!   `{"text", "model_id", "language_code" (ISO 639-1), "voice_settings": {"speed"}}`; the answer
//!   is the audio. `voice_settings` overrides the voice's stored settings for that request only,
//!   so it is sent only when a speed is given.
//! - Voices, https://elevenlabs.io/docs/api-reference/voices/search: `GET /v2/voices` with
//!   `language` ("based on the voice's 'language' label") and `page_size` (at most 100); the
//!   answer is `{"voices": [{"voice_id", "name", "labels", "preview_url", …}], "has_more", …}`.
//! - Speech to text, https://elevenlabs.io/docs/api-reference/speech-to-text/convert:
//!   `POST /v1/speech-to-text`, multipart form with `model_id`, `file`, `language_code` (ISO 639-1
//!   or 639-3) and `timestamps_granularity=word`; the answer is `{"text", "language_code",
//!   "words": [{"text", "start", "end", "type": "word"|"spacing"|"audio_event"}]}`.
//! - Model ids, https://elevenlabs.io/docs/models: `eleven_v4` (10,000 characters per request),
//!   `scribe_v2`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};

use crate::elevenlabs::API_BASE;
use crate::{secrets, AppState};

pub const DEFAULT_TTS_MODEL: &str = "eleven_v4";
pub const STT_MODEL: &str = "scribe_v2";
/// A premade voice (the one in the text-to-speech reference examples), used when the caller
/// names none. Premade voices speak every language the model supports.
pub const DEFAULT_VOICE: &str = "JBFqnCBsd6RMkjVDRZzb";
const OUTPUT_FORMAT: &str = "mp3_44100_128";
/// Clips are sentences and short passages. Below every model's per-request limit (v3: 5,000).
pub const MAX_TTS_CHARS: usize = 5_000;
/// A spoken answer, not a lecture: 25 MB is over half an hour of Opus.
pub const MAX_STT_BYTES: usize = 25 * 1024 * 1024;
const CACHE_DIR: &str = "tts-cache";

/// A validated text-to-speech request, with defaults filled in.
#[derive(Debug, Clone, PartialEq)]
pub struct Tts {
    pub text: String,
    pub lang: String,
    pub voice_id: String,
    pub model: String,
    pub speed: Option<f64>,
}

impl Tts {
    pub fn new(
        text: &str,
        lang: &str,
        voice_id: Option<&str>,
        model: Option<&str>,
        speed: Option<f64>,
    ) -> Result<Self, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("There is no text to speak.".into());
        }
        let chars = text.chars().count();
        if chars > MAX_TTS_CHARS {
            return Err(format!(
                "The text is too long to speak in one clip ({chars} characters; the limit is {MAX_TTS_CHARS}). Split it into sentences."
            ));
        }
        let voice_id = match voice_id.map(str::trim) {
            None | Some("") => DEFAULT_VOICE,
            Some(v) if v.len() <= 64 && v.bytes().all(|b| b.is_ascii_alphanumeric()) => v,
            Some(v) => return Err(format!("{v:?} is not an ElevenLabs voice id.")),
        };
        let model = match model.map(str::trim) {
            None | Some("") => DEFAULT_TTS_MODEL,
            Some(m)
                if m.len() <= 64
                    && m.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') =>
            {
                m
            }
            Some(m) => return Err(format!("{m:?} is not an ElevenLabs model id.")),
        };
        if let Some(s) = speed {
            if !(s.is_finite() && s > 0.0) {
                return Err(format!("Speed must be a positive number, not {s}."));
            }
        }
        Ok(Self {
            text: text.to_string(),
            lang: language_code(lang)?.to_string(),
            voice_id: voice_id.to_string(),
            model: model.to_string(),
            speed,
        })
    }

    /// sha256 over an unambiguous encoding of (model, voice, lang, speed, text), in hex.
    pub fn cache_key(&self) -> String {
        let encoded = json!([self.model, self.voice_id, self.lang, self.speed, self.text]);
        hex(&Sha256::digest(encoded.to_string().as_bytes()))
    }
}

/// An ISO 639-1 or 639-3 code: two or three lowercase ASCII letters.
fn language_code(lang: &str) -> Result<&str, String> {
    let lang = lang.trim();
    if (2..=3).contains(&lang.len()) && lang.bytes().all(|b| b.is_ascii_lowercase()) {
        Ok(lang)
    } else {
        Err(format!(
            "{lang:?} is not a language code (expected two or three lowercase letters, like \"es\")."
        ))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// MP3 bytes for `request`, from `<cache_dir>/<key>.mp3` when present. A cache hit makes no
/// network call and does not read the key. A failed cache write is logged; the audio is still
/// returned.
pub async fn synthesize(
    client: &reqwest::Client,
    base: &str,
    api_key: impl FnOnce() -> Result<String, String>,
    cache_dir: &Path,
    request: &Tts,
) -> Result<Vec<u8>, String> {
    let path = cache_dir.join(format!("{}.mp3", request.cache_key()));
    match std::fs::read(&path) {
        Ok(bytes) if !bytes.is_empty() => return Ok(bytes),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(format!(
                "Could not read cached audio {}: {e}",
                path.display()
            ))
        }
    }

    let key = api_key()?;
    let url = reqwest::Url::parse_with_params(
        &format!("{base}/v1/text-to-speech/{}", request.voice_id),
        [("output_format", OUTPUT_FORMAT)],
    )
    .map_err(|e| format!("bad ElevenLabs URL: {e}"))?;
    let mut body = json!({
        "text": request.text,
        "model_id": request.model,
        "language_code": request.lang,
    });
    if let Some(speed) = request.speed {
        body["voice_settings"] = json!({ "speed": speed });
    }
    let response = send(
        client
            .post(url)
            .header("xi-api-key", &key)
            .header("content-type", "application/json")
            .header("accept", "audio/mpeg")
            .body(body.to_string()),
        &key,
        Api::TextToSpeech {
            voice_id: &request.voice_id,
        },
    )
    .await?;
    let is_audio = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("audio/"));
    let bytes = response
        .bytes()
        .await
        .map_err(|e| redact(format!("ElevenLabs audio was cut off: {e}"), &key))?;
    if !is_audio || bytes.is_empty() {
        return Err("ElevenLabs answered without audio.".into());
    }
    if let Err(e) = write_atomically(&path, &bytes) {
        eprintln!("[tutor-shell] could not cache {}: {e}", path.display());
    }
    Ok(bytes.to_vec())
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // A unique temporary name, so two requests for one clip never write the same file.
    let tmp = path.with_extension(format!(
        "{}.{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Voice {
    pub voice_id: String,
    pub name: String,
    #[serde(default)]
    pub labels: Map<String, Value>,
    #[serde(default)]
    pub preview_url: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Voices {
    pub voices: Vec<Voice>,
}

/// The first page (up to 100) of the account's voices labelled with `lang`.
pub async fn list_voices(
    client: &reqwest::Client,
    base: &str,
    api_key: &str,
    lang: &str,
) -> Result<Voices, String> {
    let lang = language_code(lang)?;
    let url = reqwest::Url::parse_with_params(
        &format!("{base}/v2/voices"),
        [("language", lang), ("page_size", "100")],
    )
    .map_err(|e| format!("bad ElevenLabs URL: {e}"))?;
    let response = send(
        client.get(url).header("xi-api-key", api_key),
        api_key,
        Api::Voices,
    )
    .await?;
    let body = response
        .text()
        .await
        .map_err(|e| redact(format!("ElevenLabs voice list was cut off: {e}"), api_key))?;
    serde_json::from_str(&body).map_err(|_| "ElevenLabs answered without a voice list.".into())
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Transcript {
    pub text: String,
    pub words: Vec<Word>,
}

/// The file name and media type of a recording, from its leading bytes.
fn audio_kind(audio: &[u8]) -> Option<(&'static str, &'static str)> {
    if audio.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        Some(("audio.webm", "audio/webm"))
    } else if audio.len() >= 12 && &audio[..4] == b"RIFF" && &audio[8..12] == b"WAVE" {
        Some(("audio.wav", "audio/wav"))
    } else {
        None
    }
}

/// A multipart/form-data body. The boundary includes a hash of the audio, so it cannot occur
/// inside it.
fn multipart(
    fields: &[(&str, &str)],
    file: (&str, &str, &[u8]),
    audio_hash: &str,
) -> (String, Vec<u8>) {
    let boundary = format!("tutor-{}", &audio_hash[..32]);
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    let (file_name, media_type, bytes) = file;
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: {media_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

/// Scribe transcript of `audio` (WebM/Opus or WAV) with the language pinned. Only spoken words
/// are returned in `words`; spacing and audio events are dropped (audio-event tagging is off).
pub async fn transcribe(
    client: &reqwest::Client,
    base: &str,
    api_key: &str,
    audio: &[u8],
    lang: &str,
) -> Result<Transcript, String> {
    let lang = language_code(lang)?;
    if audio.is_empty() {
        return Err("The recording is empty.".into());
    }
    if audio.len() > MAX_STT_BYTES {
        return Err(format!(
            "The recording is too long to transcribe ({} MB; the limit is {} MB).",
            audio.len() / (1024 * 1024),
            MAX_STT_BYTES / (1024 * 1024)
        ));
    }
    let (file_name, media_type) =
        audio_kind(audio).ok_or("The recording is not WebM/Opus or WAV audio.")?;
    let (content_type, body) = multipart(
        &[
            ("model_id", STT_MODEL),
            ("language_code", lang),
            ("timestamps_granularity", "word"),
            ("tag_audio_events", "false"),
        ],
        (file_name, media_type, audio),
        &hex(&Sha256::digest(audio)),
    );
    let response = send(
        client
            .post(format!("{base}/v1/speech-to-text"))
            .header("xi-api-key", api_key)
            .header("content-type", content_type)
            .body(body),
        api_key,
        Api::SpeechToText,
    )
    .await?;
    let body = response
        .text()
        .await
        .map_err(|e| redact(format!("ElevenLabs transcript was cut off: {e}"), api_key))?;
    parse_transcript(&body).ok_or_else(|| "ElevenLabs answered without a transcript.".into())
}

fn parse_transcript(body: &str) -> Option<Transcript> {
    let value: Value = serde_json::from_str(body).ok()?;
    let text = value.get("text")?.as_str()?.to_string();
    let words = match value.get("words") {
        None | Some(Value::Null) => Vec::new(),
        Some(words) => words
            .as_array()?
            .iter()
            .filter(|w| w.get("type").and_then(Value::as_str).unwrap_or("word") == "word")
            .map(|w| {
                Some(Word {
                    text: w.get("text")?.as_str()?.to_string(),
                    start: w.get("start")?.as_f64()?,
                    end: w.get("end")?.as_f64()?,
                })
            })
            .collect::<Option<_>>()?,
    };
    Some(Transcript { text, words })
}

#[derive(Clone, Copy)]
enum Api<'a> {
    TextToSpeech { voice_id: &'a str },
    Voices,
    SpeechToText,
}

impl Api<'_> {
    fn permission(self) -> &'static str {
        match self {
            Api::TextToSpeech { .. } => "Text to Speech",
            Api::Voices => "Voices",
            Api::SpeechToText => "Speech to Text",
        }
    }
}

/// Sends a request and maps a failure to a readable sentence without the key.
async fn send(
    request: reqwest::RequestBuilder,
    api_key: &str,
    api: Api<'_>,
) -> Result<reqwest::Response, String> {
    let response = request
        .send()
        .await
        .map_err(|e| redact(format!("Could not reach ElevenLabs: {e}"), api_key))?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    let reason = match (status.as_u16(), api) {
        (401, _) => "ElevenLabs rejected the API key (401). Check the key in Settings.".to_string(),
        (403, _) => format!(
            "ElevenLabs refused the request (403): the key may lack {} permission.",
            api.permission()
        ),
        (404, Api::TextToSpeech { voice_id }) => {
            format!("ElevenLabs does not know voice {voice_id:?} (404).")
        }
        (422, _) => format!("ElevenLabs did not accept the request ({status})."),
        (429, _) => {
            "ElevenLabs is busy or the quota is used up (429). Try again later.".to_string()
        }
        _ => format!("ElevenLabs returned {status}."),
    };
    let reason = match error_message(&body) {
        Some(message) => format!("{reason} {message}"),
        None => reason,
    };
    Err(redact(reason, api_key))
}

/// ElevenLabs errors look like `{"detail": {"message": "…"}}`, `{"detail": "…"}`, or, for
/// validation errors, `{"detail": [{"msg": "…"}, …]}`.
fn error_message(body: &str) -> Option<String> {
    let detail = serde_json::from_str::<Value>(body)
        .ok()?
        .get("detail")?
        .clone();
    let message = match &detail {
        Value::String(s) => s.clone(),
        Value::Array(items) => items.first()?.get("msg")?.as_str()?.to_string(),
        _ => detail.get("message")?.as_str()?.to_string(),
    };
    Some(message.chars().take(300).collect())
}

fn redact(text: String, api_key: &str) -> String {
    if api_key.is_empty() {
        text
    } else {
        text.replace(api_key, "[redacted]")
    }
}

fn stored_key() -> Result<String, String> {
    secrets::get("elevenlabs")?
        .ok_or_else(|| "No ElevenLabs API key saved. Add it in Settings.".into())
}

fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join(CACHE_DIR))
}

#[tauri::command]
pub(crate) async fn tts_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    lang: String,
    voice_id: Option<String>,
    model: Option<String>,
    speed: Option<f64>,
) -> Result<Response, String> {
    let request = Tts::new(&text, &lang, voice_id.as_deref(), model.as_deref(), speed)?;
    let bytes = synthesize(
        &state.http,
        API_BASE,
        stored_key,
        &cache_dir(&app)?,
        &request,
    )
    .await?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub(crate) async fn tts_voices(state: State<'_, AppState>, lang: String) -> Result<Voices, String> {
    list_voices(&state.http, API_BASE, &stored_key()?, &lang).await
}

#[tauri::command]
pub(crate) async fn stt_transcribe(
    state: State<'_, AppState>,
    audio: Vec<u8>,
    lang: String,
) -> Result<Transcript, String> {
    transcribe(&state.http, API_BASE, &stored_key()?, &audio, &lang).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mock_http;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// Like `test_support::mock_http`, but also returns the request body and answers with any
    /// media type and bytes.
    fn mock(
        status: &str,
        content_type: &str,
        body: &[u8],
    ) -> (String, mpsc::Receiver<(String, Vec<u8>)>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let mut response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(body);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
                head.push_str(&line);
            }
            let length = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut request_body = vec![0; length];
            reader.read_exact(&mut request_body).unwrap();
            (&stream).write_all(&response).unwrap();
            let _ = tx.send((head, request_body));
        });
        (base, rx)
    }

    /// A base URL where nothing listens.
    fn dead_base() -> String {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        format!("http://127.0.0.1:{port}")
    }

    fn block<T>(f: impl std::future::Future<Output = T>) -> T {
        tauri::async_runtime::block_on(f)
    }

    fn key(k: &str) -> impl FnOnce() -> Result<String, String> + '_ {
        move || Ok(k.to_string())
    }

    const MP3: &[u8] = b"ID3\x04\x00fake-mp3-frames";

    #[test]
    fn validates_and_defaults_the_request() {
        let r = Tts::new("  Hola.  ", "es", None, None, None).unwrap();
        assert_eq!(
            r,
            Tts {
                text: "Hola.".into(),
                lang: "es".into(),
                voice_id: DEFAULT_VOICE.into(),
                model: DEFAULT_TTS_MODEL.into(),
                speed: None,
            }
        );
        assert_eq!(
            Tts::new("x", "es", Some(" "), Some(""), None)
                .unwrap()
                .voice_id,
            DEFAULT_VOICE
        );
        for (text, lang, voice, model, speed, expected) in [
            ("  ", "es", None, None, None, "There is no text to speak."),
            ("x", "ES", None, None, None, "is not a language code"),
            ("x", "spanish", None, None, None, "is not a language code"),
            (
                "x",
                "es",
                Some("../v1/user"),
                None,
                None,
                "is not an ElevenLabs voice id",
            ),
            (
                "x",
                "es",
                None,
                Some("eleven v4"),
                None,
                "is not an ElevenLabs model id",
            ),
            (
                "x",
                "es",
                None,
                None,
                Some(0.0),
                "Speed must be a positive number",
            ),
            (
                "x",
                "es",
                None,
                None,
                Some(f64::NAN),
                "Speed must be a positive number",
            ),
        ] {
            let err = Tts::new(text, lang, voice, model, speed).unwrap_err();
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn oversize_text_is_rejected_before_any_call() {
        let long = "é".repeat(MAX_TTS_CHARS + 1);
        let err = Tts::new(&long, "fr", None, None, None).unwrap_err();
        assert_eq!(
            err,
            format!("The text is too long to speak in one clip ({} characters; the limit is {MAX_TTS_CHARS}). Split it into sentences.", MAX_TTS_CHARS + 1)
        );
        // Counted in characters, not bytes: the limit itself is accepted.
        assert!(Tts::new(&"é".repeat(MAX_TTS_CHARS), "fr", None, None, None).is_ok());
    }

    #[test]
    fn cache_key_covers_every_input() {
        let base = Tts::new("Hola", "es", Some("v1"), Some("eleven_v4"), Some(1.0)).unwrap();
        let key = base.cache_key();
        assert_eq!(key.len(), 64);
        assert!(key.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(key, base.clone().cache_key());
        let variants = [
            Tts {
                text: "Hola!".into(),
                ..base.clone()
            },
            Tts {
                lang: "ca".into(),
                ..base.clone()
            },
            Tts {
                voice_id: "v2".into(),
                ..base.clone()
            },
            Tts {
                model: "eleven_multilingual_v2".into(),
                ..base.clone()
            },
            Tts {
                speed: Some(0.9),
                ..base.clone()
            },
            Tts {
                speed: None,
                ..base.clone()
            },
        ];
        for v in variants {
            assert_ne!(v.cache_key(), key, "{v:?}");
        }
    }

    #[test]
    fn tts_sends_the_documented_request_and_caches_the_audio() {
        let dir = tempfile::tempdir().unwrap();
        let (base, request) = mock("200 OK", "audio/mpeg", MP3);
        let tts = Tts::new("¿Qué tal?", "es", Some("Voice123"), None, Some(0.9)).unwrap();
        let audio = block(synthesize(
            &crate::http_client(),
            &base,
            key("sk_k"),
            dir.path(),
            &tts,
        ))
        .unwrap();
        assert_eq!(audio, MP3);

        let (head, body) = request.recv().unwrap();
        let head = head.to_ascii_lowercase();
        assert!(
            head.starts_with(
                "post /v1/text-to-speech/voice123?output_format=mp3_44100_128 http/1.1\r\n"
            ),
            "{head}"
        );
        assert!(head.contains("\r\nxi-api-key: sk_k\r\n"), "{head}");
        assert!(
            head.contains("\r\ncontent-type: application/json\r\n"),
            "{head}"
        );
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            json!({"text": "¿Qué tal?", "model_id": "eleven_v4", "language_code": "es",
                   "voice_settings": {"speed": 0.9}})
        );

        let cached = dir.path().join(format!("{}.mp3", tts.cache_key()));
        assert_eq!(std::fs::read(&cached).unwrap(), MP3);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "no temporary files remain");
    }

    #[test]
    fn tts_without_speed_leaves_the_voice_settings_alone() {
        let dir = tempfile::tempdir().unwrap();
        let (base, request) = mock("200 OK", "audio/mpeg", MP3);
        let tts = Tts::new("Bonjour", "fr", None, Some("eleven_multilingual_v2"), None).unwrap();
        block(synthesize(
            &crate::http_client(),
            &base,
            key("k"),
            dir.path(),
            &tts,
        ))
        .unwrap();
        let (head, body) = request.recv().unwrap();
        assert!(
            head.starts_with(&format!("POST /v1/text-to-speech/{DEFAULT_VOICE}?")),
            "{head}"
        );
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            json!({"text": "Bonjour", "model_id": "eleven_multilingual_v2", "language_code": "fr"})
        );
    }

    #[test]
    fn cache_hit_makes_no_call_and_needs_no_key() {
        let dir = tempfile::tempdir().unwrap();
        let tts = Tts::new("Guten Tag", "de", None, None, None).unwrap();
        std::fs::write(dir.path().join(format!("{}.mp3", tts.cache_key())), MP3).unwrap();
        let no_key = || -> Result<String, String> { panic!("the key was read on a cache hit") };
        let audio = block(synthesize(
            &crate::http_client(),
            &dead_base(),
            no_key,
            dir.path(),
            &tts,
        ))
        .unwrap();
        assert_eq!(audio, MP3);
    }

    #[test]
    fn cache_miss_for_a_different_request_calls_out() {
        let dir = tempfile::tempdir().unwrap();
        let cached = Tts::new("Guten Tag", "de", None, None, None).unwrap();
        std::fs::write(dir.path().join(format!("{}.mp3", cached.cache_key())), MP3).unwrap();
        let other = Tts::new("Guten Tag", "de", None, None, Some(0.8)).unwrap();
        let err = block(synthesize(
            &crate::http_client(),
            &dead_base(),
            key("k"),
            dir.path(),
            &other,
        ))
        .unwrap_err();
        assert!(err.starts_with("Could not reach ElevenLabs"), "{err}");
    }

    #[test]
    fn tts_errors_are_readable_and_never_contain_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let tts = Tts::new("Hola", "es", Some("nope"), None, None).unwrap();
        let run = |base: &str| {
            block(synthesize(
                &crate::http_client(),
                base,
                key("sk_secret_9"),
                dir.path(),
                &tts,
            ))
        };

        let (base, _) = mock(
            "401 Unauthorized",
            "application/json",
            br#"{"detail": {"status": "invalid_api_key", "message": "Invalid API key: sk_secret_9"}}"#,
        );
        let err = run(&base).unwrap_err();
        assert_eq!(
            err,
            "ElevenLabs rejected the API key (401). Check the key in Settings. Invalid API key: [redacted]"
        );

        let (base, _) = mock(
            "404 Not Found",
            "application/json",
            br#"{"detail": {"message": "voice_not_found"}}"#,
        );
        assert_eq!(
            run(&base).unwrap_err(),
            r#"ElevenLabs does not know voice "nope" (404). voice_not_found"#
        );

        let (base, _) = mock(
            "422 Unprocessable Entity",
            "application/json",
            br#"{"detail": [{"loc": ["body", "voice_settings", "speed"], "msg": "value too large", "type": "x"}]}"#,
        );
        assert_eq!(
            run(&base).unwrap_err(),
            "ElevenLabs did not accept the request (422 Unprocessable Entity). value too large"
        );

        let (base, _) = mock("429 Too Many Requests", "application/json", b"{}");
        assert_eq!(
            run(&base).unwrap_err(),
            "ElevenLabs is busy or the quota is used up (429). Try again later."
        );

        let (base, _) = mock("403 Forbidden", "text/plain", b"forbidden");
        assert_eq!(
            run(&base).unwrap_err(),
            "ElevenLabs refused the request (403): the key may lack Text to Speech permission."
        );

        let (base, _) = mock("200 OK", "application/json", br#"{"ok": true}"#);
        assert_eq!(
            run(&base).unwrap_err(),
            "ElevenLabs answered without audio."
        );

        assert_eq!(
            std::fs::read_dir(dir.path())
                .map(|d| d.count())
                .unwrap_or(0),
            0,
            "errors are never cached"
        );

        let err = run(&dead_base()).unwrap_err();
        assert!(err.starts_with("Could not reach ElevenLabs"), "{err}");
        assert!(!err.contains("sk_secret_9"), "{err}");
    }

    #[test]
    fn missing_key_is_reported_on_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let tts = Tts::new("Hola", "es", None, None, None).unwrap();
        let err = block(synthesize(
            &crate::http_client(),
            &dead_base(),
            || Err("No ElevenLabs API key saved. Add it in Settings.".into()),
            dir.path(),
            &tts,
        ))
        .unwrap_err();
        assert_eq!(err, "No ElevenLabs API key saved. Add it in Settings.");
    }

    #[test]
    fn voices_are_filtered_by_language_and_trimmed_to_the_contract() {
        let (base, head) = mock_http(
            "200 OK",
            r#"{"voices": [
                {"voice_id": "v1", "name": "Lucía", "labels": {"language": "es", "accent": "peninsular"},
                 "preview_url": "https://example.test/v1.mp3", "category": "premade",
                 "verified_languages": [{"language": "es", "model_id": "eleven_v4"}]},
                {"voice_id": "v2", "name": "Mateo", "preview_url": null}
             ], "has_more": false, "next_page_token": null}"#,
        );
        let voices = block(list_voices(&crate::http_client(), &base, "sk_k", "es")).unwrap();
        let head = head.recv().unwrap().to_ascii_lowercase();
        assert!(
            head.starts_with("get /v2/voices?language=es&page_size=100 http/1.1\r\n"),
            "{head}"
        );
        assert!(head.contains("\r\nxi-api-key: sk_k\r\n"), "{head}");
        assert_eq!(
            serde_json::to_value(&voices).unwrap(),
            json!({"voices": [
                {"voice_id": "v1", "name": "Lucía", "labels": {"language": "es", "accent": "peninsular"},
                 "preview_url": "https://example.test/v1.mp3"},
                {"voice_id": "v2", "name": "Mateo", "labels": {}, "preview_url": null}
            ]})
        );
    }

    #[test]
    fn voices_errors() {
        assert!(block(list_voices(
            &crate::http_client(),
            &dead_base(),
            "k",
            "es-ES"
        ))
        .unwrap_err()
        .contains("is not a language code"));
        let (base, _) = mock_http("200 OK", r#"{"items": []}"#);
        assert_eq!(
            block(list_voices(&crate::http_client(), &base, "k", "es")).unwrap_err(),
            "ElevenLabs answered without a voice list."
        );
        let (base, _) = mock_http(
            "403 Forbidden",
            r#"{"detail": "missing_permissions sk_x1"}"#,
        );
        assert_eq!(
            block(list_voices(&crate::http_client(), &base, "sk_x1", "es")).unwrap_err(),
            "ElevenLabs refused the request (403): the key may lack Voices permission. missing_permissions [redacted]"
        );
    }

    const WEBM: &[u8] = b"\x1a\x45\xdf\xa3\x01\x00opus-frames";

    #[test]
    fn stt_sends_a_scribe_form_and_keeps_only_words() {
        let (base, request) = mock(
            "200 OK",
            "application/json",
            r#"{"language_code": "spa", "language_probability": 0.98, "text": "Hola, amigo.",
                "words": [
                  {"text": "Hola,", "start": 0.1, "end": 0.5, "type": "word"},
                  {"text": " ", "start": 0.5, "end": 0.6, "type": "spacing"},
                  {"text": "(cough)", "start": 0.6, "end": 0.7, "type": "audio_event"},
                  {"text": "amigo.", "start": 0.7, "end": 1.2, "type": "word"}]}"#
                .as_bytes(),
        );
        let transcript = block(transcribe(
            &crate::http_client(),
            &base,
            "sk_k",
            WEBM,
            "spa",
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(&transcript).unwrap(),
            json!({"text": "Hola, amigo.", "words": [
                {"text": "Hola,", "start": 0.1, "end": 0.5},
                {"text": "amigo.", "start": 0.7, "end": 1.2}]})
        );

        let (head, body) = request.recv().unwrap();
        let lower = head.to_ascii_lowercase();
        assert!(
            lower.starts_with("post /v1/speech-to-text http/1.1\r\n"),
            "{head}"
        );
        assert!(lower.contains("\r\nxi-api-key: sk_k\r\n"), "{head}");
        let boundary = format!("tutor-{}", &hex(&Sha256::digest(WEBM))[..32]);
        assert!(
            lower.contains(&format!(
                "\r\ncontent-type: multipart/form-data; boundary={boundary}\r\n"
            )),
            "{head}"
        );

        let mut expected = Vec::new();
        for (name, value) in [
            ("model_id", "scribe_v2"),
            ("language_code", "spa"),
            ("timestamps_granularity", "word"),
            ("tag_audio_events", "false"),
        ] {
            expected.extend_from_slice(
                format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes(),
            );
        }
        expected.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.webm\"\r\nContent-Type: audio/webm\r\n\r\n").as_bytes(),
        );
        expected.extend_from_slice(WEBM);
        expected.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        assert_eq!(body, expected);
    }

    #[test]
    fn stt_accepts_wav_and_a_transcript_without_words() {
        let wav = b"RIFF\x24\x00\x00\x00WAVEfmt ";
        let (base, request) = mock("200 OK", "application/json", br#"{"text": ""}"#);
        let transcript = block(transcribe(&crate::http_client(), &base, "k", wav, "de")).unwrap();
        assert_eq!(
            transcript,
            Transcript {
                text: String::new(),
                words: vec![]
            }
        );
        let (_, body) = request.recv().unwrap();
        let body = String::from_utf8_lossy(&body);
        assert!(
            body.contains("filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n"),
            "{body}"
        );
        assert!(
            body.contains("name=\"language_code\"\r\n\r\nde\r\n"),
            "{body}"
        );
    }

    #[test]
    fn stt_rejects_bad_input_before_any_call() {
        let run = |audio: &[u8], lang: &str| {
            block(transcribe(
                &crate::http_client(),
                &dead_base(),
                "k",
                audio,
                lang,
            ))
            .unwrap_err()
        };
        assert_eq!(run(b"", "es"), "The recording is empty.");
        assert_eq!(
            run(b"ID3 not a recording", "es"),
            "The recording is not WebM/Opus or WAV audio."
        );
        assert!(run(WEBM, "zh-Hans").contains("is not a language code"));
        let mut huge = WEBM.to_vec();
        huge.resize(MAX_STT_BYTES + 1, 0);
        assert_eq!(
            run(&huge, "es"),
            "The recording is too long to transcribe (25 MB; the limit is 25 MB)."
        );
    }

    #[test]
    fn stt_errors_are_readable_and_never_contain_the_key() {
        let (base, _) = mock(
            "401 Unauthorized",
            "application/json",
            br#"{"detail": {"message": "Invalid API key: sk_secret_7"}}"#,
        );
        let err = block(transcribe(
            &crate::http_client(),
            &base,
            "sk_secret_7",
            WEBM,
            "fra",
        ))
        .unwrap_err();
        assert_eq!(err, "ElevenLabs rejected the API key (401). Check the key in Settings. Invalid API key: [redacted]");

        let (base, _) = mock(
            "500 Internal Server Error",
            "text/html",
            b"<html>oops</html>",
        );
        assert_eq!(
            block(transcribe(&crate::http_client(), &base, "k", WEBM, "fra")).unwrap_err(),
            "ElevenLabs returned 500 Internal Server Error."
        );

        let (base, _) = mock("200 OK", "application/json", br#"{"words": []}"#);
        assert_eq!(
            block(transcribe(&crate::http_client(), &base, "k", WEBM, "fra")).unwrap_err(),
            "ElevenLabs answered without a transcript."
        );
    }
}
