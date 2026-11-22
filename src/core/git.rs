use std::path::Path;

/// 从给定目录向上查找 `.git`（支持目录与 gitfile），解析当前分支名或 detached HEAD 短哈希。
pub fn detect_git_branch(start_dir: &Path) -> Option<String> {
    let mut current = Some(start_dir);
    while let Some(dir) = current {
        let dot_git = dir.join(".git");
        if dot_git.is_dir() {
            return parse_head_file(&dot_git.join("HEAD"));
        }
        if dot_git.is_file()
            && let Ok(content) = std::fs::read_to_string(&dot_git)
            && let Some(gitdir_raw) = content
                .lines()
                .find_map(|line| line.trim().strip_prefix("gitdir:"))
        {
            let gitdir_trimmed = gitdir_raw.trim();
            let gitdir_path = Path::new(gitdir_trimmed);
            let resolved = if gitdir_path.is_absolute() {
                gitdir_path.to_path_buf()
            } else {
                dir.join(gitdir_path)
            };
            return parse_head_file(&resolved.join("HEAD"));
        }
        current = dir.parent();
    }
    None
}

fn parse_head_file(head_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(head_path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(ref_target) = trimmed.strip_prefix("ref:") {
        let ref_name = ref_target.trim();
        if let Some(branch) = ref_name.strip_prefix("refs/heads/") {
            return Some(branch.to_string());
        }
        return ref_name.rsplit('/').next().map(str::to_string);
    }
    if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(trimmed[..7].to_string());
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_SEQ: AtomicUsize = AtomicUsize::new(1);

    struct TempDirGuard(PathBuf);

    impl TempDirGuard {
        fn new(label: &str) -> Self {
            let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "bmvp_git_test_{}_{}_{}",
                label,
                std::process::id(),
                seq
            ));
            std::fs::create_dir_all(&dir).expect("create temp dir");
            Self(dir)
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_detect_branch_from_repo_and_subdir() {
        let root = TempDirGuard::new("branch");
        let git_dir = root.0.join(".git");
        let sub_dir = root.0.join("src").join("nested");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/feat/p1-opt\n").unwrap();

        assert_eq!(detect_git_branch(&root.0).as_deref(), Some("feat/p1-opt"));
        assert_eq!(detect_git_branch(&sub_dir).as_deref(), Some("feat/p1-opt"));
    }

    #[test]
    fn test_detect_detached_head() {
        let root = TempDirGuard::new("detached");
        let git_dir = root.0.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("HEAD"),
            "0123456789abcdef0123456789abcdef01234567\n",
        )
        .unwrap();

        assert_eq!(detect_git_branch(&root.0).as_deref(), Some("0123456"));
    }

    #[test]
    fn test_detect_gitfile_worktree() {
        let root = TempDirGuard::new("worktree");
        let real_git = root.0.join("actual-git");
        let worktree = root.0.join("wt");
        std::fs::create_dir_all(&real_git).unwrap();
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(real_git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(worktree.join(".git"), "gitdir: ../actual-git\n").unwrap();

        assert_eq!(detect_git_branch(&worktree).as_deref(), Some("main"));
    }

    #[test]
    fn test_detect_non_git_returns_none() {
        let root = TempDirGuard::new("nongit");
        assert_eq!(detect_git_branch(&root.0), None);
    }
}
