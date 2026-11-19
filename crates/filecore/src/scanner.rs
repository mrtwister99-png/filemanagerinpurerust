use crate::format::{extension_of, format_size};
use crate::models::{FileEntry, FileKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn modified_unix(md: &fs::Metadata) -> u64 {
    md.modified()
        .unwrap_or(SystemTime::now())
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn to_entry(path: PathBuf, md: fs::Metadata) -> FileEntry {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let is_dir = md.is_dir();
    let extension = if is_dir { String::new() } else { extension_of(&name) };
    let kind = FileKind::from_path(is_dir, &extension);
    let size = if is_dir { 0 } else { md.len() };
    FileEntry {
        name: name.clone(),
        path: path.clone(),
        is_dir,
        kind,
        size,
        size_human: format_size(size),
        modified_unix: modified_unix(&md),
        extension,
        is_hidden: name.starts_with('.'),
    }
}

/// Vypise adresar. Slozky prvni, pak soubory, oboji podle nazvu.
/// Skryte soubory (zacinajici .) se ve výchozim stavu neukazuji, pokud show_hidden = false.
/// Tohle je to, co bude volat UI - rychle, synchronni, bez async bordelu.
pub fn list_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<Vec<FileEntry>> {
    list_dir_opts(dir, false)
}

pub fn list_dir_opts<P: AsRef<Path>>(
    dir: P,
    show_hidden: bool,
) -> std::io::Result<Vec<FileEntry>> {
    let mut entries: Vec<FileEntry> = Vec::new();
    let rd = fs::read_dir(dir)?;
    for item in rd.flatten() {
        let path = item.path();
        let Ok(md) = item.metadata() else { continue };
        let e = to_entry(path, md);
        if !show_hidden && e.is_hidden {
            continue;
        }
        entries.push(e);
    }
    // Slozky prvni, pak abecedne. Tohle je to, co uzivatel ceka.
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

/// Korenove cesty podle platformy.
/// Na Androidu je to /storage/emulated/0, na desktopu home + disky.
pub fn list_roots() -> Vec<PathBuf> {
    #[cfg(target_os = "android")]
    {
        let mut roots = vec![PathBuf::from("/storage/emulated/0")];
        for name in ["Download", "Pictures", "Music", "Movies", "Documents"] {
            let p = PathBuf::from("/storage/emulated/0").join(name);
            if p.exists() {
                roots.push(p);
            }
        }
        return roots;
    }

    #[cfg(not(target_os = "android"))]
    {
        let mut roots = Vec::new();
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let home = PathBuf::from(home);
            roots.push(home.clone());
            for name in ["Downloads", "Pictures", "Music", "Movies", "Documents", "Desktop"] {
                let p = home.join(name);
                if p.exists() {
                    roots.push(p);
                }
            }
        }
        // Windows disky
        #[cfg(target_os = "windows")]
        {
            for letter in 'C'..='Z' {
                let p = PathBuf::from(format!("{}:\\", letter));
                if p.exists() {
                    roots.push(p);
                }
            }
        }
        if roots.is_empty() {
            roots.push(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        }
        return roots;
    }
}
