use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

pub fn format_log_line(level: LogLevel, target: &str, message: &str) -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("[{}s] [{}] [{}] {}", secs, level.as_str(), target, message)
}

pub fn log(level: LogLevel, target: &str, message: &str) {
    eprintln!("{}", format_log_line(level, target, message));
}

pub fn info(target: &str, message: &str) {
    log(LogLevel::Info, target, message);
}

pub fn warn(target: &str, message: &str) {
    log(LogLevel::Warn, target, message);
}

pub fn error(target: &str, message: &str) {
    log(LogLevel::Error, target, message);
}

#[cfg(test)]
mod tests {
    use super::{LogLevel, format_log_line};

    #[test]
    fn test_format_log_line() {
        let info_line = format_log_line(LogLevel::Info, "config", "loaded settings");
        assert!(info_line.contains("[INFO] [config] loaded settings"));

        let warn_line = format_log_line(LogLevel::Warn, "terminal", "fallback shell");
        assert!(warn_line.contains("[WARN] [terminal] fallback shell"));

        let err_line = format_log_line(LogLevel::Error, "buffer", "save failed");
        assert!(err_line.contains("[ERROR] [buffer] save failed"));
    }
}
