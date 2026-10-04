use std::fmt;

#[derive(Debug)]
pub struct Failure {
    pub code: &'static str,
    pub message: String,
    pub exit: i32,
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Failure {}
pub fn fail(code: &'static str, message: impl Into<String>) -> anyhow::Error {
    Failure {
        code,
        message: message.into(),
        exit: 1,
    }
    .into()
}
pub fn cancelled() -> anyhow::Error {
    Failure {
        code: "cancelled",
        message: "Operation cancelled".into(),
        exit: 130,
    }
    .into()
}
