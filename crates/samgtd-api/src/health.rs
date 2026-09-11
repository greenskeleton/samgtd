use serde::{Deserialize, Serialize};

/// Response body for `GET /health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

impl HealthResponse {
    pub fn ok() -> Self {
        Self { status: "ok" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_reports_status_ok() {
        assert_eq!(HealthResponse::ok().status, "ok");
    }

    #[test]
    fn serializes_to_expected_json_shape() {
        let json = serde_json::to_string(&HealthResponse::ok()).unwrap();
        assert_eq!(json, r#"{"status":"ok"}"#);
    }
}
