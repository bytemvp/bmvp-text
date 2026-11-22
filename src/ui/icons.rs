use crate::ui::theme::Theme;
use gpui::*;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconName {
    SidebarLeft,
    Terminal,
    Search,
    Settings,
    Folder,
    FolderOpen,
    File,
    FilePlus,
    Refresh,
    Plus,
    Close,
    ChevronRight,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ArrowUp,
    ArrowDown,
    GitBranch,
    Hash,
    ExternalLink,
    Trash,
    Stop,
    Check,
    WindowMinimize,
    WindowMaximize,
    WindowRestore,
    WindowClose,
    AlertCircle,
}

impl IconName {
    pub const ALL: &'static [IconName] = &[
        IconName::SidebarLeft,
        IconName::Terminal,
        IconName::Search,
        IconName::Settings,
        IconName::Folder,
        IconName::FolderOpen,
        IconName::File,
        IconName::FilePlus,
        IconName::Refresh,
        IconName::Plus,
        IconName::Close,
        IconName::ChevronRight,
        IconName::ChevronDown,
        IconName::ChevronUp,
        IconName::ChevronLeft,
        IconName::ArrowUp,
        IconName::ArrowDown,
        IconName::GitBranch,
        IconName::Hash,
        IconName::ExternalLink,
        IconName::Trash,
        IconName::Stop,
        IconName::Check,
        IconName::WindowMinimize,
        IconName::WindowMaximize,
        IconName::WindowRestore,
        IconName::WindowClose,
        IconName::AlertCircle,
    ];

    pub fn path(self) -> &'static str {
        match self {
            IconName::SidebarLeft => "icons/sidebar-left.svg",
            IconName::Terminal => "icons/terminal.svg",
            IconName::Search => "icons/search.svg",
            IconName::Settings => "icons/settings.svg",
            IconName::Folder => "icons/folder.svg",
            IconName::FolderOpen => "icons/folder-open.svg",
            IconName::File => "icons/file.svg",
            IconName::FilePlus => "icons/file-plus.svg",
            IconName::Refresh => "icons/refresh.svg",
            IconName::Plus => "icons/plus.svg",
            IconName::Close => "icons/close.svg",
            IconName::ChevronRight => "icons/chevron-right.svg",
            IconName::ChevronDown => "icons/chevron-down.svg",
            IconName::ChevronUp => "icons/chevron-up.svg",
            IconName::ChevronLeft => "icons/chevron-left.svg",
            IconName::ArrowUp => "icons/arrow-up.svg",
            IconName::ArrowDown => "icons/arrow-down.svg",
            IconName::GitBranch => "icons/git-branch.svg",
            IconName::Hash => "icons/hash.svg",
            IconName::ExternalLink => "icons/external-link.svg",
            IconName::Trash => "icons/trash.svg",
            IconName::Stop => "icons/stop.svg",
            IconName::Check => "icons/check.svg",
            IconName::WindowMinimize => "icons/window-minimize.svg",
            IconName::WindowMaximize => "icons/window-maximize.svg",
            IconName::WindowRestore => "icons/window-restore.svg",
            IconName::WindowClose => "icons/window-close.svg",
            IconName::AlertCircle => "icons/alert-circle.svg",
        }
    }

    pub fn svg_bytes(self) -> &'static [u8] {
        match self {
            IconName::SidebarLeft => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="2" y="2.5" width="12" height="11" rx="1.5" stroke="#000000" stroke-width="1.35"/><path d="M6 2.5V13.5" stroke="#000000" stroke-width="1.35"/></svg>"##,
            IconName::Terminal => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="1.75" y="2.5" width="12.5" height="11" rx="1.5" stroke="#000000" stroke-width="1.35"/><path d="M4.5 6L7 8L4.5 10" stroke="#000000" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/><path d="M8.5 10.25H11.5" stroke="#000000" stroke-width="1.35" stroke-linecap="round"/></svg>"##,
            IconName::Search => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="7" cy="7" r="4.5" stroke="#000000" stroke-width="1.4"/><path d="M10.5 10.5L13.75 13.75" stroke="#000000" stroke-width="1.4" stroke-linecap="round"/></svg>"##,
            IconName::Settings => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="8" cy="8" r="2.25" stroke="#000000" stroke-width="1.35"/><path d="M8 1.75V3.1M8 12.9V14.25M1.75 8H3.1M12.9 8H14.25M3.58 3.58L4.55 4.55M11.45 11.45L12.42 12.42M12.42 3.58L11.45 4.55M4.55 11.45L3.58 12.42" stroke="#000000" stroke-width="1.35" stroke-linecap="round"/></svg>"##,
            IconName::Folder => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M2 4.25C2 3.42 2.67 2.75 3.5 2.75H6.15C6.55 2.75 6.93 2.91 7.21 3.19L8.12 4.1C8.26 4.24 8.45 4.32 8.65 4.32H12.5C13.33 4.32 14 4.99 14 5.82V11.75C14 12.58 13.33 13.25 12.5 13.25H3.5C2.67 13.25 2 12.58 2 11.75V4.25Z" stroke="#000000" stroke-width="1.35" stroke-linejoin="round"/></svg>"##,
            IconName::FolderOpen => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M2 4.25C2 3.42 2.67 2.75 3.5 2.75H6.15C6.55 2.75 6.93 2.91 7.21 3.19L8.12 4.1C8.26 4.24 8.45 4.32 8.65 4.32H12.5C13.33 4.32 14 4.99 14 5.82V6.75H3.9C3.22 6.75 2.62 7.21 2.45 7.87L2 9.6V4.25Z" stroke="#000000" stroke-width="1.3" stroke-linejoin="round"/><path d="M2.15 11.9L3.1 7.95C3.22 7.45 3.67 7.1 4.18 7.1H13.35C13.97 7.1 14.41 7.69 14.24 8.29L13.15 12.15C13.01 12.65 12.55 13 12.03 13H3.05C2.44 13 2.01 12.48 2.15 11.9Z" stroke="#000000" stroke-width="1.3" stroke-linejoin="round"/></svg>"##,
            IconName::File => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3.5 2.25C3.5 1.84 3.84 1.5 4.25 1.5H9.5L12.5 4.5V13.75C12.5 14.16 12.16 14.5 11.75 14.5H4.25C3.84 14.5 3.5 14.16 3.5 13.75V2.25Z" stroke="#000000" stroke-width="1.35" stroke-linejoin="round"/><path d="M9.25 1.75V4.75H12.25" stroke="#000000" stroke-width="1.35" stroke-linejoin="round"/></svg>"##,
            IconName::FilePlus => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3.5 2.25C3.5 1.84 3.84 1.5 4.25 1.5H9.5L12.5 4.5V13.75C12.5 14.16 12.16 14.5 11.75 14.5H4.25C3.84 14.5 3.5 14.16 3.5 13.75V2.25Z" stroke="#000000" stroke-width="1.35" stroke-linejoin="round"/><path d="M8 7V11M6 9H10" stroke="#000000" stroke-width="1.35" stroke-linecap="round"/></svg>"##,
            IconName::Refresh => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M13.25 8A5.25 5.25 0 1 1 11.7 4.28L13.5 6" stroke="#000000" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/><path d="M13.5 2.75V6H10.25" stroke="#000000" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::Plus => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 3V13M3 8H13" stroke="#000000" stroke-width="1.4" stroke-linecap="round"/></svg>"##,
            IconName::Close => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M4 4L12 12M12 4L4 12" stroke="#000000" stroke-width="1.45" stroke-linecap="round"/></svg>"##,
            IconName::ChevronRight => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M6 3.75L10.25 8L6 12.25" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::ChevronDown => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3.75 6L8 10.25L12.25 6" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::ChevronUp => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3.75 10L8 5.75L12.25 10" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::ChevronLeft => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M10 3.75L5.75 8L10 12.25" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::ArrowUp => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 13V3.5M4 7.25L8 3.25L12 7.25" stroke="#000000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::ArrowDown => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 3V12.5M4 8.75L8 12.75L12 8.75" stroke="#000000" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::GitBranch => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="4.5" cy="3.5" r="1.5" stroke="#000000" stroke-width="1.35"/><circle cx="4.5" cy="12.5" r="1.5" stroke="#000000" stroke-width="1.35"/><circle cx="11.5" cy="5.5" r="1.5" stroke="#000000" stroke-width="1.35"/><path d="M4.5 5V11M11.5 7C11.5 9.2 9.5 10 4.5 10" stroke="#000000" stroke-width="1.35" stroke-linecap="round"/></svg>"##,
            IconName::Hash => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3 5.75H13.5M2.5 10.25H13M6.25 2.5L4.75 13.5M11.25 2.5L9.75 13.5" stroke="#000000" stroke-width="1.35" stroke-linecap="round"/></svg>"##,
            IconName::ExternalLink => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M12.5 9V12.5C12.5 13.05 12.05 13.5 11.5 13.5H3.5C2.95 13.5 2.5 13.05 2.5 12.5V4.5C2.5 3.95 2.95 3.5 3.5 3.5H7" stroke="#000000" stroke-width="1.35" stroke-linecap="round"/><path d="M9.5 2.5H13.5V6.5M13.25 2.75L7.5 8.5" stroke="#000000" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::Trash => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M2.75 4.25H13.25M6 4.25V2.75C6 2.34 6.34 2 6.75 2H9.25C9.66 2 10 2.34 10 2.75V4.25M4.25 4.25L4.85 12.8C4.9 13.48 5.46 14 6.14 14H9.86C10.54 14 11.1 13.48 11.15 12.8L11.75 4.25" stroke="#000000" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::Stop => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="3.25" y="3.25" width="9.5" height="9.5" rx="1.5" fill="#000000"/></svg>"##,
            IconName::Check => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3.25 8.5L6.5 11.75L13 4.5" stroke="#000000" stroke-width="1.55" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
            IconName::WindowMinimize => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3.25 8.5H12.75" stroke="#000000" stroke-width="1.35" stroke-linecap="round"/></svg>"##,
            IconName::WindowMaximize => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="3.25" y="3.25" width="9.5" height="9.5" rx="1" stroke="#000000" stroke-width="1.35"/></svg>"##,
            IconName::WindowRestore => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="3" y="5" width="7.75" height="7.75" rx="1" stroke="#000000" stroke-width="1.3"/><path d="M5.25 5V3.75C5.25 3.34 5.59 3 6 3H12C12.55 3 13 3.45 13 4V10C13 10.41 12.66 10.75 12.25 10.75H10.75" stroke="#000000" stroke-width="1.3" stroke-linecap="round"/></svg>"##,
            IconName::WindowClose => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3.75 3.75L12.25 12.25M12.25 3.75L3.75 12.25" stroke="#000000" stroke-width="1.35" stroke-linecap="round"/></svg>"##,
            IconName::AlertCircle => br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="8" cy="8" r="6" stroke="#000000" stroke-width="1.35"/><path d="M8 4.75V8.5" stroke="#000000" stroke-width="1.45" stroke-linecap="round"/><circle cx="8" cy="11.2" r="0.85" fill="#000000"/></svg>"##,
        }
    }
}

pub struct EmbeddedAssets;

impl AssetSource for EmbeddedAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let found = IconName::ALL
            .iter()
            .find(|icon| icon.path() == path)
            .map(|icon| Cow::Borrowed(icon.svg_bytes()));
        Ok(found)
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        if path.is_empty() || path == "icons" || path == "icons/" {
            Ok(IconName::ALL
                .iter()
                .map(|icon| SharedString::from(icon.path()))
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
}

pub fn render_icon(icon: IconName, size: Pixels, color: Hsla) -> Svg {
    svg()
        .path(icon.path())
        .size(size)
        .flex_shrink_0()
        .text_color(color)
}

pub fn file_badge_style(file_name: &str, theme: &Theme) -> (&'static str, Hsla) {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".rs") {
        ("rs", theme.syntax.number)
    } else if lower.ends_with(".toml") {
        ("cfg", theme.syntax.type_name)
    } else if lower.ends_with(".md") {
        ("md", theme.accent)
    } else if lower.ends_with(".json") {
        ("{}", theme.syntax.type_name)
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        ("yml", theme.syntax.keyword)
    } else if lower.ends_with(".lock") {
        ("·", theme.text_muted)
    } else if lower.ends_with(".gitignore") || lower == ".gitattributes" {
        ("git", theme.syntax.keyword)
    } else if lower.ends_with(".py") {
        ("py", theme.syntax.function)
    } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
        ("js", theme.syntax.type_name)
    } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
        ("ts", theme.accent)
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        ("<>", theme.syntax.number)
    } else if lower.ends_with(".css") || lower.ends_with(".scss") || lower.ends_with(".sass") {
        ("css", theme.syntax.function)
    } else if lower.ends_with(".sh")
        || lower.ends_with(".bash")
        || lower.ends_with(".bat")
        || lower.ends_with(".cmd")
        || lower.ends_with(".ps1")
    {
        ("sh", theme.syntax.string)
    } else if lower.ends_with(".c")
        || lower.ends_with(".cpp")
        || lower.ends_with(".cc")
        || lower.ends_with(".h")
        || lower.ends_with(".hpp")
    {
        ("c", theme.syntax.function)
    } else if lower.ends_with(".svg")
        || lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".ico")
    {
        ("img", theme.syntax.string)
    } else if lower.ends_with(".txt") {
        ("txt", theme.text_secondary)
    } else {
        ("·", theme.text_muted)
    }
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedAssets, IconName, file_badge_style};
    use crate::ui::theme::Theme;
    use gpui::AssetSource;

    #[test]
    fn test_embedded_assets_load_and_list_all_icons() {
        let assets = EmbeddedAssets;
        let listed = assets.list("icons").expect("list icons");
        assert_eq!(listed.len(), IconName::ALL.len());

        for &icon in IconName::ALL {
            let bytes = assets
                .load(icon.path())
                .expect("load icon")
                .expect("icon bytes present");
            let text = std::str::from_utf8(&bytes).expect("valid utf8 svg");
            assert!(text.contains("<svg"), "missing <svg for {:?}", icon);
            assert!(
                text.contains("viewBox=\"0 0 16 16\""),
                "missing viewBox for {:?}",
                icon
            );
            assert!(
                !text.contains("<text"),
                "unexpected <text element in {:?}",
                icon
            );
        }

        assert!(assets.load("icons/nonexistent.svg").unwrap().is_none());
    }

    #[test]
    fn test_file_badge_style_semantic_colors() {
        let theme = Theme::zed_dark();
        let (rs_label, rs_color) = file_badge_style("main.rs", &theme);
        assert_eq!(rs_label, "rs");
        assert_eq!(rs_color, theme.syntax.number);

        let (toml_label, toml_color) = file_badge_style("Cargo.toml", &theme);
        assert_eq!(toml_label, "cfg");
        assert_eq!(toml_color, theme.syntax.type_name);

        let (unknown_label, unknown_color) = file_badge_style("README", &theme);
        assert_eq!(unknown_label, "·");
        assert_eq!(unknown_color, theme.text_muted);
    }
}
