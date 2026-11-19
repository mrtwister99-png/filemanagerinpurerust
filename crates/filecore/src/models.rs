use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Jeden zaznam v adresari - soubor nebo slozka
#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub kind: FileKind,
    pub size: u64,
    pub size_human: String,
    pub modified_unix: u64,
    pub extension: String,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Dir,
    Image,
    Video,
    Audio,
    Document,
    Archive,
    Code,
    Other,
}

impl FileKind {
    pub fn from_path(is_dir: bool, extension: &str) -> Self {
        if is_dir {
            return FileKind::Dir;
        }
        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "gif" | "heic" | "heif" | "bmp" | "svg" | "tif"
            | "tiff" | "ico" => FileKind::Image,
            "mp4" | "mkv" | "mov" | "webm" | "avi" | "m4v" | "3gp" | "wmv" => FileKind::Video,
            "mp3" | "flac" | "wav" | "ogg" | "m4a" | "opus" | "aac" | "wma" => FileKind::Audio,
            "pdf" | "txt" | "md" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt"
            | "ods" | "odp" | "rtf" | "csv" | "epub" => FileKind::Document,
            "zip" | "rar" | "7z" | "tar" | "gz" | "xz" | "bz2" | "tgz" | "apk" => {
                FileKind::Archive
            }
            "rs" | "js" | "mjs" | "ts" | "py" | "html" | "css" | "json" | "toml" | "yaml"
            | "yml" | "xml" | "c" | "h" | "cpp" | "hpp" | "java" | "kt" | "go" | "sh" | "bat"
            | "ps1" | "sql" => FileKind::Code,
            _ => FileKind::Other,
        }
    }
}

/// Detail jednoho souboru / slozky
#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub entry: FileEntry,
    pub children_count: Option<usize>,
    pub parent: Option<PathBuf>,
}

/// Volne / celkove misto na svazku
#[derive(Debug, Clone, Serialize)]
pub struct SpaceInfo {
    pub free: u64,
    pub total: u64,
    pub free_human: String,
    pub total_human: String,
}

/// Polozka postranniho panelu "Mista" (domov, Stazene, disky, SD karty...)
#[derive(Debug, Clone, Serialize)]
pub struct Place {
    pub label: String,
    pub path: PathBuf,
    /// "home" | "folder" | "drive" | "storage" | "sd"
    pub kind: String,
    pub space: Option<SpaceInfo>,
}

/// Velikost jedne polozky pro "Analyzu mista"
#[derive(Debug, Clone, Serialize)]
pub struct SizeEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub size_human: String,
    pub files: u64,
}

/// Vysledek hledani - zasahy + jestli se narazilo na limit / zruseni
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub hits: Vec<FileEntry>,
    pub truncated: bool,
    pub cancelled: bool,
}
