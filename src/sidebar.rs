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
        };
        if let Some(ref path) = current_dir {
            sidebar.rebuild_tree(path);
        }
        sidebar
    }

    pub fn set_root(&mut self, path: PathBuf) {
        self.root_path = Some(path.clone());
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
        let expanded_paths: std::collections::HashSet<PathBuf> = self
            .items
            .iter()
            .filter(|item| item.is_dir && item.is_expanded)
            .map(|item| item.path.clone())
            .collect();

        self.items.clear();
        Self::collect_children(root, 0, &expanded_paths, &mut self.items);
    }

    fn collect_children(
        dir: &Path,
        depth: usize,
        expanded_paths: &std::collections::HashSet<PathBuf>,
        out: &mut Vec<FileNode>,
    ) {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }
                let is_dir = path.is_dir();
                entries.push((path, name, is_dir));
            }
        }

        // 文件夹在前，其次按名称不区分大小写排序
        entries.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
        });

        for (path, name, is_dir) in entries {
            let is_expanded = is_dir && expanded_paths.contains(&path);
            out.push(FileNode {
                path: path.clone(),
                name,
                is_dir,
                depth,
                is_expanded,
            });

            if is_dir && is_expanded {
                Self::collect_children(&path, depth + 1, expanded_paths, out);
            }
        }
    }

    pub fn toggle_expand(&mut self, idx: usize) {
        if let Some(item) = self.items.get(idx)
            && item.is_dir
        {
            let path = item.path.clone();
            let new_state = !item.is_expanded;
            let mut expanded_paths: std::collections::HashSet<PathBuf> = self
                .items
                .iter()
                .filter(|i| i.is_dir && i.is_expanded)
                .map(|i| i.path.clone())
                .collect();

            if new_state {
                expanded_paths.insert(path);
            } else {
                expanded_paths.remove(&path);
            }

            if let Some(ref root) = self.root_path.clone() {
                self.items.clear();
                Self::collect_children(root, 0, &expanded_paths, &mut self.items);
            }
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
