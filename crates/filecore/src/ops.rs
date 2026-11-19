use crate::format::{extension_of, format_size};
use crate::models::{FileEntry, FileInfo, FileKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_info<P: AsRef<Path>>(path: P) -> std::io::Result<FileInfo> {
    let path = path.as_ref().to_path_buf();
    let md = fs::metadata(&path)?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let is_dir = md.is_dir();
    let extension = if is_dir { String::new() } else { extension_of(&name) };
    let entry = FileEntry {
        name: name.clone(),
        path: path.clone(),
        is_dir,
        kind: FileKind::from_path(is_dir, &extension),
        size: if is_dir { 0 } else { md.len() },
        size_human: format_size(if is_dir { 0 } else { md.len() }),
        modified_unix: md
            .modified()
            .unwrap_or(SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        extension,
        is_hidden: name.starts_with('.'),
    };
    let children_count = if is_dir {
        fs::read_dir(&path).map(|rd| rd.count()).ok()
    } else {
        None
    };
    Ok(FileInfo {
        entry,
        children_count,
        parent: path.parent().map(|p| p.to_path_buf()),
    })
}

pub fn create_dir<P: AsRef<Path>>(parent: P, name: &str) -> std::io::Result<PathBuf> {
    let clean = name.trim().trim_matches(|c| c == '/' || c == '\\');
    if clean.is_empty() || clean.contains("..") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Neplatný název složky",
        ));
    }
    let target = parent.as_ref().join(clean);
    fs::create_dir(&target)?;
    Ok(target)
}

pub fn rename_entry<P: AsRef<Path>>(from: P, new_name: &str) -> std::io::Result<PathBuf> {
    let from = from.as_ref();
    let clean = new_name.trim();
    if clean.is_empty() || clean.contains('/') || clean.contains('\\') || clean.contains("..") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Neplatný nový název",
        ));
    }
    let to = from
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(clean);
    fs::rename(from, &to)?;
    Ok(to)
}

/// Jmeno, ktere v cilove slozce jeste neexistuje - "fotka.jpg" -> "fotka (1).jpg".
pub(crate) fn unique_target(dir: &Path, name: &std::ffi::OsStr) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("soubor");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    for i in 1..10_000 {
        let file_name = if ext.is_empty() {
            format!("{} ({})", stem, i)
        } else {
            format!("{} ({}).{}", stem, i, ext)
        };
        let c = dir.join(file_name);
        if !c.exists() {
            return c;
        }
    }
    candidate
}

/// Kopie souboru nebo slozky do jineho adresare. Nazev se pri kolizi doplni cislovkou.
pub fn copy_entry<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to_dir: Q) -> std::io::Result<PathBuf> {
    let mut ctx = crate::transfer::Ctx::new(&|_| {}, &|| false);
    crate::transfer::transfer_one(&mut ctx, from.as_ref(), to_dir.as_ref(), false)
}

/// Presun (prejmenovani cesty). Kdyz to filesystem neumi (jiny disk), zkopiruje a smaze.
pub fn move_entry<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to_dir: Q) -> std::io::Result<PathBuf> {
    let mut ctx = crate::transfer::Ctx::new(&|_| {}, &|| false);
    crate::transfer::transfer_one(&mut ctx, from.as_ref(), to_dir.as_ref(), true)
}

/// Precte zacatek souboru jako text - pro nahled v UI, bez zatezovani pameti.
pub fn read_text_preview<P: AsRef<Path>>(path: P, max_bytes: usize) -> std::io::Result<String> {
    let data = fs::read(path)?;
    let end = data.len().min(max_bytes);
    Ok(String::from_utf8_lossy(&data[..end]).to_string())
}

/// Smaze soubor nebo slozku vcetne obsahu. Pozor, je to nevratne, UI se musi zeptat.
pub fn delete_entry<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    let path = path.as_ref();
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Je k dispozici kos? Desktop: systemovy, Android: vlastni kos appky (viz `bin`).
pub fn trash_available() -> bool {
    crate::bin::bin_available()
}

/// Presune soubor nebo slozku do systemoveho kose (na Windows Kos).
pub fn trash_entry<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    #[cfg(not(target_os = "android"))]
    {
        trash::delete(path.as_ref()).map_err(|e| std::io::Error::other(e.to_string()))
    }
    #[cfg(target_os = "android")]
    {
        // Android: vlastni kos appky
        crate::bin::bin_put(path.as_ref())
    }
}

/// Smazani podle voleb: do kose, kdyz je k dispozici a neni chteno nevratne smazani.
pub fn remove_entry<P: AsRef<Path>>(path: P, permanent: bool) -> std::io::Result<()> {
    if permanent || !trash_available() {
        delete_entry(path)
    } else {
        trash_entry(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nevratne_smazani_a_unikatni_nazev() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::write(r.join("a.txt"), "x").unwrap();
        assert_eq!(unique_target(r, std::ffi::OsStr::new("a.txt")), r.join("a (1).txt"));
        fs::create_dir_all(r.join("d/e")).unwrap();
        remove_entry(r.join("d"), true).unwrap();
        assert!(!r.join("d").exists());
    }
}
