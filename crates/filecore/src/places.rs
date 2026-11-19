//! "Mista" pro postranni panel: domovska slozka, standardni slozky, disky,
//! na Androidu interni uloziste a SD karty. U disku/svazku i volne misto.

use crate::format::format_size;
use crate::models::{Place, SpaceInfo};
use std::path::{Path, PathBuf};

/// Volne a celkove misto na svazku, kde lezi `path`.
pub fn disk_space<P: AsRef<Path>>(path: P) -> std::io::Result<SpaceInfo> {
    let stats = fs4::statvfs(path.as_ref())?;
    let free = stats.available_space();
    let total = stats.total_space();
    Ok(SpaceInfo {
        free,
        total,
        free_human: format_size(free),
        total_human: format_size(total),
    })
}

fn place(label: &str, path: PathBuf, kind: &str) -> Place {
    let space = if matches!(kind, "drive" | "storage" | "sd") {
        disk_space(&path).ok()
    } else {
        None
    };
    Place { label: label.to_string(), path, kind: kind.to_string(), space }
}

/// Dalsi svazky v /storage (SD karty, USB) - vse krome "emulated" a "self".
pub fn external_volumes<P: AsRef<Path>>(storage_root: P) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(storage_root.as_ref()) {
        for item in rd.flatten() {
            let name = item.file_name().to_string_lossy().to_string();
            if name == "emulated" || name == "self" || name.starts_with('.') {
                continue;
            }
            if item.path().is_dir() {
                out.push(item.path());
            }
        }
    }
    out.sort();
    out
}

/// Seznam mist pro postranni panel. Muze trvat (zjistovani mista na discich),
/// proto ho Tauri vola asynchronne.
pub fn list_places() -> Vec<Place> {
    #[cfg(target_os = "android")]
    {
        let internal = PathBuf::from("/storage/emulated/0");
        let mut out = vec![place("Interní úložiště", internal.clone(), "storage")];
        for (name, label) in [
            ("Download", "Stažené"),
            ("DCIM", "Fotoaparát"),
            ("Pictures", "Obrázky"),
            ("Music", "Hudba"),
            ("Movies", "Videa"),
            ("Documents", "Dokumenty"),
        ] {
            let p = internal.join(name);
            if p.exists() {
                out.push(place(label, p, "folder"));
            }
        }
        for vol in external_volumes("/storage") {
            let id = vol.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            out.push(place(&format!("SD karta ({})", id), vol, "sd"));
        }
        out
    }

    #[cfg(not(target_os = "android"))]
    {
        let mut out = Vec::new();
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let home = PathBuf::from(home);
            out.push(place("Domů", home.clone(), "home"));
            for (name, label) in [
                ("Desktop", "Plocha"),
                ("Downloads", "Stažené"),
                ("Documents", "Dokumenty"),
                ("Pictures", "Obrázky"),
                ("Music", "Hudba"),
                ("Videos", "Videa"),
                ("Movies", "Videa"),
            ] {
                // Windows s OneDrive casto presmeruje Plochu/Dokumenty do OneDrive
                let p = [home.join(name), home.join("OneDrive").join(name)]
                    .into_iter()
                    .find(|p| p.is_dir());
                if let Some(p) = p {
                    if !out.iter().any(|x: &Place| x.label == label) {
                        out.push(place(label, p, "folder"));
                    }
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            for letter in 'C'..='Z' {
                let p = PathBuf::from(format!("{}:\\", letter));
                if p.exists() {
                    out.push(place(&format!("Disk {}:", letter), p, "drive"));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            out.push(place("Systém (/)", PathBuf::from("/"), "drive"));
            for media in ["/media", "/mnt", "/Volumes"] {
                for vol in external_volumes(media) {
                    let label = vol.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    out.push(place(&label, vol, "drive"));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn externi_svazky_bez_emulated_a_self() {
        let t = tempfile::tempdir().unwrap();
        for d in ["emulated", "self", "1234-ABCD"] {
            std::fs::create_dir(t.path().join(d)).unwrap();
        }
        let v = external_volumes(t.path());
        assert_eq!(v, vec![t.path().join("1234-ABCD")]);
    }

    #[test]
    fn misto_na_disku() {
        let t = tempfile::tempdir().unwrap();
        let s = disk_space(t.path()).unwrap();
        assert!(s.total > 0 && s.free <= s.total);
    }
}
