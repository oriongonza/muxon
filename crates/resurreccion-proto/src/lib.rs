//! Protocol types for the Resurreccion daemon and clients.

use std::path::{Path, PathBuf};

/// Default socket path for the Resurreccion daemon.
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/resurreccion.sock";
/// Service name of the Resurreccion daemon.
pub const SERVICE_NAME: &str = "resurreccion-daemon";

/// A request that can be sent to the Resurreccion daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Health check request.
    Health,
}

impl Request {
    /// Parse a request from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is not a valid request.
    pub fn parse(input: &str) -> Result<Self, String> {
        match input.trim() {
            "health" => Ok(Self::Health),
            other => Err(format!("unknown request: {other}")),
        }
    }

    /// Get the wire format representation of this request.
    pub const fn as_wire(&self) -> &'static str {
        match self {
            Self::Health => "health\n",
        }
    }
}

/// An error response from the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorResponse {
    /// Error code.
    pub code: String,
    /// Error message.
    pub message: String,
}

/// A health check response from the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthResponse {
    /// Name of the service.
    pub service: &'static str,
    /// Status of the service.
    pub status: &'static str,
    /// Path to the socket.
    pub socket_path: String,
}

/// A response from the Resurreccion daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// Health check response.
    Health(HealthResponse),
    /// Error response.
    Error(ErrorResponse),
}

impl Response {
    /// Create a successful health response.
    pub fn health(socket_path: &Path) -> Self {
        Self::Health(HealthResponse {
            service: SERVICE_NAME,
            status: "ok",
            socket_path: socket_path.display().to_string(),
        })
    }

    /// Create an error response.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error(ErrorResponse {
            code: code.into(),
            message: message.into(),
        })
    }

    /// Serialize this response to JSON.
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

/// Get the default socket path for the Resurreccion daemon.
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
        assert_eq!(
            default_socket_path().display().to_string(),
            "/tmp/resurreccion.sock"
        );
    }
}
