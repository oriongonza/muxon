use std::path::{Path, PathBuf};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/resurreccion.sock";
pub const SERVICE_NAME: &str = "resurreccion-daemon";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Health,
}

impl Request {
    pub fn parse(input: &str) -> Result<Self, String> {
        match input.trim() {
            "health" => Ok(Self::Health),
            other => Err(format!("unknown request: {other}")),
        }
    }

    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Health => "health\n",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthResponse {
    pub service: &'static str,
    pub status: &'static str,
    pub socket_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Health(HealthResponse),
    Error(ErrorResponse),
}

impl Response {
    pub fn health(socket_path: &Path) -> Self {
        Self::Health(HealthResponse {
            service: SERVICE_NAME,
            status: "ok",
            socket_path: socket_path.display().to_string(),
        })
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error(ErrorResponse {
            code: code.into(),
            message: message.into(),
        })
    }

    pub fn to_json(&self) -> String {
        match self {
            Self::Health(health) => format!(
                "{{\"ok\":true,\"data\":{{\"service\":\"{}\",\"status\":\"{}\",\"socket_path\":\"{}\"}}}}",
                escape_json(health.service),
                escape_json(health.status),
                escape_json(&health.socket_path),
            ),
            Self::Error(error) => format!(
                "{{\"ok\":false,\"error\":{{\"code\":\"{}\",\"message\":\"{}\"}}}}",
                escape_json(&error.code),
                escape_json(&error.message),
            ),
        }
    }
}

pub fn default_socket_path() -> PathBuf {
    PathBuf::from(DEFAULT_SOCKET_PATH)
}

fn escape_json(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{default_socket_path, Request, Response};
    use std::path::Path;

    #[test]
    fn parses_health_request() {
        assert_eq!(Request::parse("health").unwrap(), Request::Health);
        assert_eq!(Request::parse("health\n").unwrap(), Request::Health);
    }

    #[test]
    fn serializes_health_response_to_json() {
        let json = Response::health(Path::new("/tmp/example.sock")).to_json();
        assert_eq!(
            json,
            "{\"ok\":true,\"data\":{\"service\":\"resurreccion-daemon\",\"status\":\"ok\",\"socket_path\":\"/tmp/example.sock\"}}"
        );
    }

    #[test]
    fn default_socket_path_is_stable() {
        assert_eq!(default_socket_path().display().to_string(), "/tmp/resurreccion.sock");
    }
}
