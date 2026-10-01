//! Bearer-token authentication for MCP (an axum middleware) and gRPC (a tonic interceptor).
//!
//! The token file holds 32 random bytes as hex. The token is compared in constant time and never appears in
//! logs or `Debug` output.

use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use rand::RngExt;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use subtle::ConstantTimeEq;

/// Shorter tokens are rejected at load time; a hex-encoded 32-byte token has 64 characters.
pub const MIN_TOKEN_CHARS: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("cannot read token file {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("the token must be at least {MIN_TOKEN_CHARS} printable ASCII characters without spaces")]
    Weak,
}

#[derive(Clone)]
pub struct Token(Arc<str>);

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

impl Token {
    pub fn new(token: &str) -> Result<Self, TokenError> {
        let token = token.trim();
        if token.len() < MIN_TOKEN_CHARS || !token.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(TokenError::Weak);
        }
        Ok(Self(token.into()))
    }

    /// Reads the token file. Surrounding whitespace such as a final newline is ignored.
    pub fn load(path: &Path) -> Result<Self, TokenError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| TokenError::Read { path: path.to_path_buf(), source })?;
        Self::new(&text)
    }

    /// 32 random bytes from the operating system's generator, as 64 hex characters.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill(&mut bytes);
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Self(hex.into())
    }

    /// The token text, for writing the token file or building a client header. Never log it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Constant-time comparison. Only the length can leak, and it is the same for every token we issue.
    pub fn verify(&self, presented: &str) -> bool {
        self.0.as_bytes().ct_eq(presented.as_bytes()).into()
    }

    /// Checks an `Authorization` header value of the form `Bearer <token>`.
    pub fn verify_header(&self, header_value: &str) -> bool {
        bearer(header_value).is_some_and(|presented| self.verify(presented))
    }
}

/// The credentials in an `Authorization: Bearer ...` value; the scheme is case-insensitive.
fn bearer(value: &str) -> Option<&str> {
    let (scheme, rest) = value.trim().split_once(char::is_whitespace)?;
    scheme.eq_ignore_ascii_case("bearer").then(|| rest.trim()).filter(|t| !t.is_empty())
}

/// Axum middleware: `401` unless the request carries the token.
pub async fn require_bearer(State(token): State<Token>, request: Request, next: Next) -> Response {
    let ok = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| token.verify_header(v));
    if ok {
        return next.run(request).await;
    }
    tracing::warn!(path = %request.uri().path(), "rejected request without a valid bearer token");
    let mut response = (StatusCode::UNAUTHORIZED, "{\"error\":\"unauthorized\"}").into_response();
    response.headers_mut().insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
}

/// Tonic interceptor: `unauthenticated` unless the call carries the token.
#[derive(Clone)]
pub struct BearerInterceptor(Token);

impl BearerInterceptor {
    pub fn new(token: Token) -> Self {
        Self(token)
    }
}

impl tonic::service::Interceptor for BearerInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let ok = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| self.0.verify_header(v));
        if ok {
            Ok(request)
        } else {
            tracing::warn!("rejected gRPC call without a valid bearer token");
            Err(tonic::Status::unauthenticated("missing or invalid bearer token"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::service::Interceptor;

    const T: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn generated_tokens_are_64_hex_and_distinct() {
        let (a, b) = (Token::generate(), Token::generate());
        assert_eq!(a.expose().len(), 64);
        assert!(a.expose().bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a.expose(), b.expose());
        assert!(Token::new(a.expose()).is_ok());
    }

    #[test]
    fn weak_or_malformed_tokens_are_refused() {
        for bad in
            ["", "short", "has a space in it 0123456789abcdef0123456789", "tab\tseparated0123456789abcdef0123456789"]
        {
            assert!(matches!(Token::new(bad), Err(TokenError::Weak)), "{bad:?}");
        }
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        let t = Token::new(&format!("{T}\r\n")).unwrap();
        assert!(t.verify(T));
    }

    #[test]
    fn verify_accepts_only_the_exact_token() {
        let t = Token::new(T).unwrap();
        assert!(t.verify(T));
        assert!(!t.verify(""));
        assert!(!t.verify(&T[..63]));
        assert!(!t.verify(&format!("{T}0")));
        assert!(!t.verify(&T.to_uppercase()));
    }

    #[test]
    fn bearer_parsing() {
        let t = Token::new(T).unwrap();
        assert!(t.verify_header(&format!("Bearer {T}")));
        assert!(t.verify_header(&format!("bearer   {T}")));
        assert!(!t.verify_header(T));
        assert!(!t.verify_header("Bearer "));
        assert!(!t.verify_header(&format!("Basic {T}")));
        assert!(!t.verify_header(&format!("Bearer {T} extra")));
    }

    #[test]
    fn debug_output_hides_the_token() {
        let t = Token::new(T).unwrap();
        assert!(!format!("{t:?}").contains("0123"));
    }

    #[test]
    fn missing_file_is_a_read_error() {
        assert!(matches!(Token::load(Path::new("definitely/not/here")), Err(TokenError::Read { .. })));
    }

    #[test]
    fn interceptor_checks_the_authorization_metadata() {
        let mut i = BearerInterceptor::new(Token::new(T).unwrap());
        let mut ok = tonic::Request::new(());
        ok.metadata_mut().insert("authorization", format!("Bearer {T}").parse().unwrap());
        assert!(i.call(ok).is_ok());
        let none = i.call(tonic::Request::new(())).unwrap_err();
        assert_eq!(none.code(), tonic::Code::Unauthenticated);
        let mut wrong = tonic::Request::new(());
        wrong.metadata_mut().insert("authorization", "Bearer nope".parse().unwrap());
        assert_eq!(i.call(wrong).unwrap_err().code(), tonic::Code::Unauthenticated);
    }
}
