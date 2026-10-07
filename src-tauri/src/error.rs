use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    pub fn storage(message: &str) -> Self {
        Self {
            code: "storage_error",
            message: message.into(),
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: "not_found",
            message: "History item no longer exists".into(),
        }
    }

    pub fn duplicate() -> Self {
        Self {
            code: "duplicate",
            message: "That text already exists in history".into(),
        }
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self {
            code: "invalid_input",
            message: message.into(),
        }
    }
}
