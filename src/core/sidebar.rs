use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
}

pub struct Sidebar {
    pub root_path: Option<PathBuf>,
    pub items: Vec<FileNode>,
    pub scroll_offset: usize,
    pub gitignore_names: Vec<String>,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().ok();
        let mut sidebar = Self {
            root_path: current_dir.clone(),
            items: Vec::new(),
            scroll_offset: 0,
            gitignore_names: Vec::new(),
        };
        if let Some(ref path) = current_dir {
            sidebar.rebuild_tree(path);
        }
        sidebar
    }

    pub fn load_gitignore_names(root: &Path) -> Vec<String> {
        let mut names = Vec::new();
        let Ok(content) = std::fs::read_to_string(root.join(".gitignore")) else {
            return names;
        };
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                continue;
            }
            let clean = trimmed.trim_start_matches('/').trim_end_matches('/');
            if !clean.is_empty()
                && !clean.contains('/')
                && !clean.contains('*')
                && !clean.contains('?')
            {
                names.push(clean.to_string());
            }
        }
        names
    }

    pub fn is_ignored_entry(name: &str, gitignore_names: &[String]) -> bool {
        if matches!(name, ".git" | "target" | "node_modules") {
            return true;
        }
        gitignore_names.iter().any(|pattern| pattern == name)
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize, max_visible: usize) {
        let max_scroll = self.items.len().saturating_sub(max_visible.max(1));
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    pub fn set_root(&mut self, path: PathBuf) {
        self.root_path = Some(path.clone());
        self.scroll_offset = 0;
        self.rebuild_tree(&path);
    }

    pub fn refresh(&mut self) {
        if let Some(ref root) = self.root_path.clone() {
            self.rebuild_tree(root);
        }
    }

    #[allow(dead_code)]
    pub fn open_folder_dialog(&mut self) -> bool {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.set_root(folder);
            true
        } else {
            false
        }
    }

    pub fn rebuild_tree(&mut self, root: &Path) {
        let expanded_paths: HashSet<PathBuf> = self
            .items
            .iter()
            .filter(|item| item.is_dir && item.is_expanded)
            .map(|item| item.path.clone())
            .collect();

        self.gitignore_names = Self::load_gitignore_names(root);
        self.items.clear();
        Self::collect_children(
            root,
            0,
            &expanded_paths,
            &self.gitignore_names,
            &mut self.items,
        );
        self.scroll_offset = self.scroll_offset.min(self.items.len().saturating_sub(1));
    }

    fn read_sorted_children(
        dir: &Path,
        depth: usize,
        gitignore_names: &[String],
    ) -> Vec<(PathBuf, String, bool, usize)> {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if Self::is_ignored_entry(&name, gitignore_names) {
                    continue;
                }
                let is_dir = path.is_dir();
                entries.push((path, name, is_dir, depth));
            }
        }

        // 文件夹在前，其次按名称不区分大小写排序
        entries.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
        });
        entries
    }

    fn collect_children(
        dir: &Path,
        depth: usize,
        expanded_paths: &HashSet<PathBuf>,
        gitignore_names: &[String],
        out: &mut Vec<FileNode>,
    ) {
        for (path, name, is_dir, d) in Self::read_sorted_children(dir, depth, gitignore_names) {
            let is_expanded = is_dir && expanded_paths.contains(&path);
            out.push(FileNode {
                path: path.clone(),
                name,
                is_dir,
                depth: d,
                is_expanded,
            });

            if is_dir && is_expanded {
                Self::collect_children(&path, depth + 1, expanded_paths, gitignore_names, out);
            }
        }
    }

    /// 增量展开或折叠单个目录节点，避免每次点击全量重建整棵目录树
    pub fn toggle_expand(&mut self, idx: usize) {
        let Some(item) = self.items.get(idx) else {
            return;
        };
        if !item.is_dir {
            return;
        }
        let path = item.path.clone();
        let parent_depth = item.depth;
        let currently_expanded = item.is_expanded;

        if currently_expanded {
            self.items[idx].is_expanded = false;
            let mut end = idx + 1;
            while end < self.items.len() && self.items[end].depth > parent_depth {
                end += 1;
            }
            if end > idx + 1 {
                self.items.drain((idx + 1)..end);
            }
            self.scroll_offset = self.scroll_offset.min(self.items.len().saturating_sub(1));
        } else {
            self.items[idx].is_expanded = true;
            let children: Vec<FileNode> =
                Self::read_sorted_children(&path, parent_depth + 1, &self.gitignore_names)
                    .into_iter()
                    .map(|(p, name, is_dir, depth)| FileNode {
                        path: p,
                        name,
                        is_dir,
                        depth,
                        is_expanded: false,
                    })
                    .collect();
            self.items.splice((idx + 1)..(idx + 1), children);
        }
    }

    pub fn file_icon(is_dir: bool, is_expanded: bool, name: &str) -> &'static str {
        if is_dir {
            return if is_expanded { "⌄" } else { "›" };
        }
        let lower = name.to_lowercase();
        if lower.ends_with(".rs") {
            "rs"
        } else if lower.ends_with(".toml") {
            "toml"
        } else if lower.ends_with(".md") {
            "md"
        } else if lower.ends_with(".json") {
            "{}"
        } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
            "yml"
        } else if lower.ends_with(".lock") {
            "·"
        } else if lower.ends_with(".gitignore") {
            "git"
        } else if lower.ends_with(".py") {
            "py"
        } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
            "js"
        } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
            "ts"
        } else if lower.ends_with(".html") || lower.ends_with(".htm") {
            "<>"
        } else if lower.ends_with(".css") || lower.ends_with(".scss") || lower.ends_with(".sass") {
            "css"
        } else if lower.ends_with(".sh")
            || lower.ends_with(".bash")
            || lower.ends_with(".bat")
            || lower.ends_with(".cmd")
            || lower.ends_with(".ps1")
        {
            "sh"
        } else if lower.ends_with(".c")
            || lower.ends_with(".cpp")
            || lower.ends_with(".cc")
            || lower.ends_with(".h")
            || lower.ends_with(".hpp")
        {
            "c"
        } else if lower.ends_with(".svg")
            || lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".ico")
        {
            "img"
        } else if lower.ends_with(".txt") {
            "txt"
        } else {
            "·"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_dotfiles_and_gitignore_filtering() {
        let ignored = vec!["dist".to_string(), "package-output".to_string()];
        assert!(Sidebar::is_ignored_entry(".git", &ignored));
        assert!(Sidebar::is_ignored_entry("target", &ignored));
        assert!(Sidebar::is_ignored_entry("dist", &ignored));
        assert!(!Sidebar::is_ignored_entry(".gitignore", &ignored));
        assert!(!Sidebar::is_ignored_entry(".github", &ignored));
        assert!(!Sidebar::is_ignored_entry(".env", &ignored));
    }

    #[test]
    fn test_sidebar_scroll_clamping() {
        let mut sb = Sidebar {
            root_path: None,
            items: (0..20)
                .map(|i| FileNode {
                    path: PathBuf::from(format!("file_{}", i)),
                    name: format!("file_{}", i),
                    is_dir: false,
                    depth: 0,
                    is_expanded: false,
                })
                .collect(),
            scroll_offset: 0,
            gitignore_names: Vec::new(),
        };
        sb.scroll_down(100, 8);
        assert_eq!(sb.scroll_offset, 12);
        sb.scroll_up(5);
        assert_eq!(sb.scroll_offset, 7);
        sb.scroll_up(99);
        assert_eq!(sb.scroll_offset, 0);
    }

    #[test]
    fn test_sidebar_incremental_toggle_expand_and_collapse() {
        let root = std::env::temp_dir().join(format!("bmvp_sb_test_{}", std::process::id()));
        let sub = root.join("src");
        let nested = sub.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(sub.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(nested.join("mod.rs"), "").unwrap();
        std::fs::write(root.join("Cargo.toml"), "").unwrap();

        let mut sb = Sidebar {
            root_path: Some(root.clone()),
            items: Vec::new(),
            scroll_offset: 0,
            gitignore_names: Vec::new(),
        };
        sb.rebuild_tree(&root);
        assert_eq!(sb.items.len(), 2);
        assert_eq!(sb.items[0].name, "src");
        assert_eq!(sb.items[1].name, "Cargo.toml");

        // 展开 src（仅增加其直接子项 nested 与 main.rs）
        sb.toggle_expand(0);
        assert!(sb.items[0].is_expanded);
        assert_eq!(sb.items.len(), 4);
        assert_eq!(sb.items[1].name, "nested");
        assert_eq!(sb.items[1].depth, 1);
        assert_eq!(sb.items[2].name, "main.rs");
        assert_eq!(sb.items[2].depth, 1);
        assert_eq!(sb.items[3].name, "Cargo.toml");
        assert_eq!(sb.items[3].depth, 0);

        // 展开 nested
        sb.toggle_expand(1);
        assert_eq!(sb.items.len(), 5);
        assert_eq!(sb.items[2].name, "mod.rs");
        assert_eq!(sb.items[2].depth, 2);

        // 折叠 src 时应一次性移除所有后代节点，保留 Cargo.toml
        sb.toggle_expand(0);
        assert!(!sb.items[0].is_expanded);
        assert_eq!(sb.items.len(), 2);
        assert_eq!(sb.items[1].name, "Cargo.toml");

        let _ = std::fs::remove_dir_all(&root);
    }
}
