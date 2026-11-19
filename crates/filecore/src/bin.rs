//! Kos: vypis, obnoveni a vysypani.
//!
//! * Windows (a Linux): systemovy kos pres crate `trash` (`os_limited`).
//! * Android: system zadny kos nema, proto appka drzi vlastni - skryta slozka
//!   `.FileManagerTrash` v korenu kazdeho uloziste. Kazda smazana polozka ma vlastni
//!   podslozku `<id>/` s `meta.json` (puvodni cesta, cas) a samotnou polozkou.
//!   Kod vlastniho kose (`appbin`) se prekladaji vsude, aby sel testovat na PC.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Jedna polozka v kosi, jak ji vidi UI.
#[derive(Debug, Clone, Serialize)]
pub struct BinItem {
    pub id: String,
    pub name: String,
    /// Slozka, ve ktere polozka puvodne lezela
    pub original_dir: String,
    pub deleted_unix: u64,
    /// Velikost souboru; u slozek 0 (nepocita se)
    pub size: u64,
    pub is_dir: bool,
}

/// Ma platforma nejaky kos pouzitelny z appky?
pub fn bin_available() -> bool {
    cfg!(any(
        target_os = "windows",
        target_os = "android",
        all(unix, not(target_os = "macos"), not(target_os = "ios"))
    ))
}

#[cfg(not(any(
    target_os = "android",
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"))
)))]
fn unsupported() -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, "Na této platformě není kos dostupný")
}

/// Presune polozku do kose (Android: vlastni kos, jinde systemovy).
pub fn bin_put(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "android")]
    {
        appbin::put(path, &android_bin_dir_for(path))
    }
    #[cfg(not(target_os = "android"))]
    {
        crate::ops::trash_entry(path)
    }
}

pub fn bin_list() -> io::Result<Vec<BinItem>> {
    #[cfg(target_os = "android")]
    {
        appbin::list(&android_bin_dirs())
    }
    #[cfg(not(target_os = "android"))]
    {
        sys::list()
    }
}

/// Obnovi polozky na puvodni misto. Vraci pocet obnovenych.
pub fn bin_restore(ids: &[String]) -> io::Result<usize> {
    #[cfg(target_os = "android")]
    {
        appbin::restore(&android_bin_dirs(), ids)
    }
    #[cfg(not(target_os = "android"))]
    {
        sys::restore(ids)
    }
}

/// Nenavratne smaze vybrane polozky z kose.
pub fn bin_purge(ids: &[String]) -> io::Result<()> {
    #[cfg(target_os = "android")]
    {
        appbin::purge(&android_bin_dirs(), ids)
    }
    #[cfg(not(target_os = "android"))]
    {
        sys::purge(ids)
    }
}

/// Vysype cely kos.
pub fn bin_empty() -> io::Result<()> {
    let ids: Vec<String> = bin_list()?.into_iter().map(|i| i.id).collect();
    bin_purge(&ids)
}

/* ---------------- Android: umisteni kose ---------------- */

#[cfg(target_os = "android")]
const ANDROID_INTERNAL: &str = "/storage/emulated/0";

#[cfg(target_os = "android")]
fn android_bin_dir_for(path: &Path) -> PathBuf {
    // /storage/<svazek>/... -> kos primo na tom svazku (rename je pak rychly)
    let mut comps = path.components();
    if let (Some(_), Some(a), Some(b)) = (comps.next(), comps.next(), comps.next()) {
        if a.as_os_str() == "storage" && b.as_os_str() != "self" {
            return Path::new("/storage").join(b.as_os_str()).join(appbin::BIN_NAME);
        }
    }
    Path::new(ANDROID_INTERNAL).join(appbin::BIN_NAME)
}

#[cfg(target_os = "android")]
fn android_bin_dirs() -> Vec<PathBuf> {
    let mut v = vec![Path::new(ANDROID_INTERNAL).join(appbin::BIN_NAME)];
    for vol in crate::places::external_volumes("/storage") {
        v.push(vol.join(appbin::BIN_NAME));
    }
    v
}

/* ---------------- systemovy kos (Windows, Linux) ---------------- */

#[cfg(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
))]
mod sys {
    use super::*;

    fn io_err(e: trash::Error) -> io::Error {
        io::Error::other(e.to_string())
    }

    fn id_of(item: &trash::TrashItem) -> String {
        item.id.to_string_lossy().to_string()
    }

    pub fn list() -> io::Result<Vec<BinItem>> {
        let items = trash::os_limited::list().map_err(io_err)?;
        let mut out: Vec<BinItem> = items
            .iter()
            .map(|it| {
                let (size, is_dir) = match trash::os_limited::metadata(it) {
                    Ok(md) => match md.size {
                        trash::TrashItemSize::Bytes(b) => (b, false),
                        trash::TrashItemSize::Entries(_) => (0, true),
                    },
                    Err(_) => (0, false),
                };
                BinItem {
                    id: id_of(it),
                    name: it.name.to_string_lossy().to_string(),
                    original_dir: it.original_parent.to_string_lossy().to_string(),
                    deleted_unix: it.time_deleted.max(0) as u64,
                    size,
                    is_dir,
                }
            })
            .collect();
        out.sort_by(|a, b| b.deleted_unix.cmp(&a.deleted_unix));
        Ok(out)
    }

    fn pick(ids: &[String]) -> io::Result<Vec<trash::TrashItem>> {
        let want: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
        Ok(trash::os_limited::list()
            .map_err(io_err)?
            .into_iter()
            .filter(|it| want.contains(id_of(it).as_str()))
            .collect())
    }

    pub fn restore(ids: &[String]) -> io::Result<usize> {
        let items = pick(ids)?;
        let n = items.len();
        trash::os_limited::restore_all(items).map_err(io_err)?;
        Ok(n)
    }

    pub fn purge(ids: &[String]) -> io::Result<()> {
        trash::os_limited::purge_all(pick(ids)?).map_err(io_err)
    }
}

#[cfg(not(any(
    target_os = "android",
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"))
)))]
mod sys {
    use super::*;

    pub fn list() -> io::Result<Vec<BinItem>> {
        Err(unsupported())
    }
    pub fn restore(_ids: &[String]) -> io::Result<usize> {
        Err(unsupported())
    }
    pub fn purge(_ids: &[String]) -> io::Result<()> {
        Err(unsupported())
    }
}

/* ---------------- vlastni kos appky ---------------- */

pub mod appbin {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    pub const BIN_NAME: &str = ".FileManagerTrash";
    const META: &str = "meta.json";

    #[derive(Serialize, Deserialize)]
    struct Meta {
        original_path: String,
        name: String,
        deleted_unix: u64,
        is_dir: bool,
        size: u64,
    }

    fn new_id() -> String {
        static N: AtomicU64 = AtomicU64::new(0);
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        format!("{}-{}", ms, N.fetch_add(1, Ordering::Relaxed))
    }

    /// Presune `path` do kose `bin_dir` (vytvori ho, kdyz neexistuje).
    pub fn put(path: &Path, bin_dir: &Path) -> io::Result<()> {
        let md = fs::symlink_metadata(path)?;
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Nelze určit název"))?
            .to_string_lossy()
            .to_string();
        // Kos nesmi mazat sam sebe ani vlastni obsah
        if path.starts_with(bin_dir) || bin_dir.starts_with(path) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Koš nelze přesunout do koše",
            ));
        }
        let id = new_id();
        let id_dir = bin_dir.join(&id);
        fs::create_dir_all(&id_dir)?;
        let meta = Meta {
            original_path: path.to_string_lossy().to_string(),
            name,
            deleted_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            is_dir: md.is_dir(),
            size: if md.is_dir() { 0 } else { md.len() },
        };
        // Nejdriv presun, az pak meta - kdyz presun selze, po sobe uklidime
        if let Err(e) = crate::ops::move_entry(path, &id_dir) {
            let _ = fs::remove_dir_all(&id_dir);
            return Err(e);
        }
        let json = serde_json::to_vec_pretty(&meta).map_err(io::Error::other)?;
        fs::write(id_dir.join(META), json)?;
        Ok(())
    }

    fn read_meta(id_dir: &Path) -> Option<Meta> {
        serde_json::from_slice(&fs::read(id_dir.join(META)).ok()?).ok()
    }

    pub fn list(bin_dirs: &[PathBuf]) -> io::Result<Vec<BinItem>> {
        let mut out = Vec::new();
        for dir in bin_dirs {
            let Ok(rd) = fs::read_dir(dir) else { continue };
            for e in rd.flatten() {
                let id_dir = e.path();
                let Some(m) = read_meta(&id_dir) else { continue };
                let original_dir = Path::new(&m.original_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                out.push(BinItem {
                    // id nese i cestu ke kosi, aby sel najit i pri vice uloziste
                    id: id_dir.to_string_lossy().to_string(),
                    name: m.name,
                    original_dir,
                    deleted_unix: m.deleted_unix,
                    size: m.size,
                    is_dir: m.is_dir,
                });
            }
        }
        out.sort_by(|a, b| b.deleted_unix.cmp(&a.deleted_unix));
        Ok(out)
    }

    /// Id z UI musi lezet primo v nekterem z kosu - jinak se odmitne (nikdy nemazat cizi cesty).
    fn checked_dir(bin_dirs: &[PathBuf], id: &str) -> io::Result<PathBuf> {
        let p = PathBuf::from(id);
        match p.parent() {
            Some(parent) if bin_dirs.iter().any(|b| b == parent) && p.file_name().is_some() => Ok(p),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Neplatná položka koše")),
        }
    }

    pub fn restore(bin_dirs: &[PathBuf], ids: &[String]) -> io::Result<usize> {
        let mut n = 0;
        for id in ids {
            let id_dir = checked_dir(bin_dirs, id)?;
            let meta = read_meta(&id_dir)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Položka v koši neexistuje"))?;
            let target_dir = Path::new(&meta.original_path)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();
            if !target_dir.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Původní složka už neexistuje: {}", target_dir.display()),
                ));
            }
            crate::ops::move_entry(id_dir.join(&meta.name), &target_dir)?;
            fs::remove_dir_all(&id_dir)?;
            n += 1;
        }
        Ok(n)
    }

    pub fn purge(bin_dirs: &[PathBuf], ids: &[String]) -> io::Result<()> {
        for id in ids {
            fs::remove_dir_all(checked_dir(bin_dirs, id)?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::appbin;
    use std::fs;

    #[test]
    fn vlastni_kos_smazat_vypsat_obnovit_vysypat() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        let bin = r.join(appbin::BIN_NAME);
        fs::create_dir_all(r.join("data/slozka")).unwrap();
        fs::write(r.join("data/a.txt"), "ahoj").unwrap();
        fs::write(r.join("data/slozka/b.txt"), "svete").unwrap();

        appbin::put(&r.join("data/a.txt"), &bin).unwrap();
        appbin::put(&r.join("data/slozka"), &bin).unwrap();
        assert!(!r.join("data/a.txt").exists());
        assert!(!r.join("data/slozka").exists());

        let bins = vec![bin.clone()];
        let items = appbin::list(&bins).unwrap();
        assert_eq!(items.len(), 2);
        let a = items.iter().find(|i| i.name == "a.txt").unwrap();
        assert_eq!((a.size, a.is_dir), (4, false));
        assert!(items.iter().find(|i| i.name == "slozka").unwrap().is_dir);

        // Obnoveni; na puvodnim miste uz neco lezi -> dostane cislo
        fs::write(r.join("data/a.txt"), "novy").unwrap();
        assert_eq!(appbin::restore(&bins, &[a.id.clone()]).unwrap(), 1);
        assert_eq!(fs::read_to_string(r.join("data/a (1).txt")).unwrap(), "ahoj");
        assert_eq!(fs::read_to_string(r.join("data/a.txt")).unwrap(), "novy");
        assert_eq!(appbin::list(&bins).unwrap().len(), 1);

        // Nenavratne smazani; cizi cesta se odmitne
        let rest = appbin::list(&bins).unwrap();
        assert!(appbin::purge(&bins, &[r.join("data").to_string_lossy().to_string()]).is_err());
        assert!(r.join("data").exists());
        appbin::purge(&bins, &[rest[0].id.clone()]).unwrap();
        assert!(appbin::list(&bins).unwrap().is_empty());
    }

    #[test]
    fn kos_nejde_hodit_do_sebe() {
        let t = tempfile::tempdir().unwrap();
        let bin = t.path().join(appbin::BIN_NAME);
        fs::create_dir_all(&bin).unwrap();
        assert!(appbin::put(&bin, &bin).is_err());
        assert!(appbin::put(t.path(), &bin).is_err());
    }
}
