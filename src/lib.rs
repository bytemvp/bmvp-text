pub mod config;
pub mod core;
pub mod platform;
pub mod terminal;
pub mod ui;

use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartupArgs {
    pub root_dir: Option<PathBuf>,
    pub files: Vec<PathBuf>,
}

pub fn parse_startup_args<I>(args: I) -> StartupArgs
where
    I: IntoIterator<Item = OsString>,
{
    let mut result = StartupArgs::default();
    for arg in args.into_iter().skip(1) {
        let s = arg.to_string_lossy();
        if s.is_empty() || s.starts_with('-') {
            continue;
        }
        let path = PathBuf::from(arg);
        if path.is_dir() {
            result.root_dir = Some(path);
        } else {
            if result.root_dir.is_none()
                && let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
                && parent.is_dir()
            {
                result.root_dir = Some(parent.to_path_buf());
            }
            result.files.push(path);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{StartupArgs, parse_startup_args};
    use std::ffi::OsString;

    #[test]
    fn test_parse_startup_args_empty_and_dir_and_files() {
        let empty = parse_startup_args([OsString::from("bmvp-text")]);
        assert_eq!(empty, StartupArgs::default());

        let base = std::env::temp_dir().join(format!("bmvp_cli_test_{}", std::process::id()));
        let sub = base.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let file = sub.join("demo.rs");
        std::fs::write(&file, "fn main() {}").unwrap();

        let dir_only =
            parse_startup_args([OsString::from("bmvp-text"), base.clone().into_os_string()]);
        assert_eq!(dir_only.root_dir, Some(base.clone()));
        assert!(dir_only.files.is_empty());

        let file_only =
            parse_startup_args([OsString::from("bmvp-text"), file.clone().into_os_string()]);
        assert_eq!(file_only.root_dir, Some(sub.clone()));
        assert_eq!(file_only.files, vec![file]);

        let _ = std::fs::remove_dir_all(&base);
    }
}
