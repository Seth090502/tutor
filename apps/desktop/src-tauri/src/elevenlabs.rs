//! Conversation tokens for the voice tutor, fetched with the stored key so the key never
//! reaches the webview. API: GET /v1/convai/conversation/token?agent_id=… with `xi-api-key`,
//! returning `{"token": "…", …}`.

use serde::Serialize;
use serde_json::Value;

pub const API_BASE: &str = "https://api.elevenlabs.io";

#[derive(Debug, Serialize)]
pub struct Token {
    pub token: String,
}

/// Errors are readable and never contain the key, even if the server echoes it.
pub async fn conversation_token(
    client: &reqwest::Client,
    base: &str,
    api_key: &str,
    agent_id: &str,
) -> Result<Token, String> {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        return Err("No ElevenLabs agent id. Set it in Settings.".into());
    }
    let url = reqwest::Url::parse_with_params(
        &format!("{base}/v1/convai/conversation/token"),
        [("agent_id", agent_id)],
    )
    .map_err(|e| format!("bad ElevenLabs URL: {e}"))?;
    let response = client
        .get(url)
        .header("xi-api-key", api_key)
        .send()
        .await
        .map_err(|e| redact(format!("Could not reach ElevenLabs: {e}"), api_key))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        let reason = match status.as_u16() {
            401 => "ElevenLabs rejected the API key (401). Check the key in Settings.".to_string(),
            403 => {
                "ElevenLabs refused the request (403): the key may lack Agents permission.".into()
            }
            404 | 422 => format!("ElevenLabs did not accept agent id {agent_id:?} ({status})."),
            _ => format!("ElevenLabs returned {status}."),
        };
        let reason = match error_message(&body) {
            Some(message) => format!("{reason} {message}"),
            None => reason,
        };
        return Err(redact(reason, api_key));
    }
    let token = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| v.get("token").and_then(Value::as_str).map(str::to_owned))
        .ok_or("ElevenLabs answered without a token.")?;
    Ok(Token { token })
}

/// ElevenLabs errors look like `{"detail": {"message": "…"}}` or `{"detail": "…"}`.
fn error_message(body: &str) -> Option<String> {
    let detail = serde_json::from_str::<Value>(body)
        .ok()?
        .get("detail")?
        .clone();
    let message = match &detail {
        Value::String(s) => s.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mock_http as mock;
    use std::net::TcpListener;

    fn run(base: &str, key: &str, agent: &str) -> Result<Token, String> {
        tauri::async_runtime::block_on(conversation_token(&crate::http_client(), base, key, agent))
    }

    #[test]
    fn sends_the_key_header_and_agent_id_and_returns_the_token() {
        let (base, head) = mock(
            "200 OK",
            r#"{"token": "lk-token-1", "conversation_id": "c1"}"#,
        );
        let token = run(&base, "sk_test_key", "agent_7101 x").unwrap();
        assert_eq!(token.token, "lk-token-1");
        let head = head.recv().unwrap().to_ascii_lowercase();
        assert!(
            head.starts_with(
                "get /v1/convai/conversation/token?agent_id=agent_7101+x http/1.1\r\n"
            ),
            "{head}"
        );
        assert!(head.contains("\r\nxi-api-key: sk_test_key\r\n"), "{head}");
    }

    #[test]
    fn bad_key_is_readable_and_never_echoes_the_key() {
        let body = r#"{"detail": {"status": "invalid_api_key", "message": "Invalid API key: sk_secret_123"}}"#;
        let (base, _head) = mock("401 Unauthorized", body);
        let err = run(&base, "sk_secret_123", "agent_1").unwrap_err();
        assert!(
            err.starts_with("ElevenLabs rejected the API key (401)."),
            "{err}"
        );
        assert!(err.contains("Invalid API key: [redacted]"), "{err}");
        assert!(!err.contains("sk_secret_123"), "{err}");
    }

    #[test]
    fn unknown_agent_and_missing_token_are_errors() {
        let (base, _head) = mock("404 Not Found", r#"{"detail": "Agent not found"}"#);
        let err = run(&base, "k", "agent_nope").unwrap_err();
        assert_eq!(
            err,
            r#"ElevenLabs did not accept agent id "agent_nope" (404 Not Found). Agent not found"#
        );

        let (base, _head) = mock("200 OK", r#"{"signed_url": "wss://x"}"#);
        assert_eq!(
            run(&base, "k", "a").unwrap_err(),
            "ElevenLabs answered without a token."
        );
    }

    #[test]
    fn unreachable_server_and_empty_agent_are_errors() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let err = run(&format!("http://127.0.0.1:{port}"), "sk_secret_123", "a").unwrap_err();
        assert!(err.starts_with("Could not reach ElevenLabs"), "{err}");
        assert!(!err.contains("sk_secret_123"));
        assert_eq!(
            run("http://127.0.0.1:9", "k", "  ").unwrap_err(),
            "No ElevenLabs agent id. Set it in Settings."
        );
    }
}
