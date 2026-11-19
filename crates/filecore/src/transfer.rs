//! Kopirovani a presun s prubehem a moznosti zruseni.
//!
//! Soubory se kopiruji po blocich, takze jde hlasit postup i u jednoho velkeho souboru
//! a zruseni zabere do zlomku vteriny. Zrusena nebo neuspesna polozka po sobe nenecha
//! nedokonceny cil - ten je vzdy novy (unikatni nazev), takze ho jde bezpecne smazat.

use crate::ops::unique_target;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const CHUNK: usize = 1024 * 1024;
const TICK: Duration = Duration::from_millis(80);

/// Stav prenosu, jak ho vidi UI.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TransferProgress {
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub done_files: u64,
    pub total_files: u64,
    /// Nazev prave zpracovavaneho souboru
    pub current: String,
}

/// Vysledek prenosu vice polozek.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TransferSummary {
    /// Kolik polozek ze zadaneho seznamu se dokoncilo cele
    pub done_items: usize,
    pub total_items: usize,
    pub cancelled: bool,
    pub targets: Vec<PathBuf>,
}

pub(crate) struct Ctx<'a> {
    p: TransferProgress,
    on_progress: &'a dyn Fn(&TransferProgress),
    is_cancelled: &'a dyn Fn() -> bool,
    last: Instant,
    cancelled: bool,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(
        on_progress: &'a dyn Fn(&TransferProgress),
        is_cancelled: &'a dyn Fn() -> bool,
    ) -> Self {
        Ctx {
            p: TransferProgress::default(),
            on_progress,
            is_cancelled,
            last: Instant::now(),
            cancelled: false,
        }
    }

    fn tick(&mut self, force: bool) {
        if force || self.last.elapsed() >= TICK {
            self.last = Instant::now();
            (self.on_progress)(&self.p);
        }
    }

    /// Vrati chybu, kdyz uzivatel prenos zrusil.
    fn check(&mut self) -> io::Result<()> {
        if self.cancelled || (self.is_cancelled)() {
            self.cancelled = true;
            return Err(io::Error::other("Zrušeno"));
        }
        Ok(())
    }
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

/// Velikost a pocet souboru pod cestou (symlinky se preskakuji, stejne jako pri kopirovani).
fn measure(path: &Path) -> (u64, u64) {
    let Ok(md) = fs::symlink_metadata(path) else { return (0, 0) };
    if !md.is_dir() {
        return (md.len(), 1);
    }
    let mut bytes = 0;
    let mut files = 0;
    for item in walkdir::WalkDir::new(path).min_depth(1).into_iter().flatten() {
        if item.file_type().is_file() {
            bytes += item.metadata().map(|m| m.len()).unwrap_or(0);
            files += 1;
        }
    }
    (bytes, files)
}

fn copy_file(ctx: &mut Ctx, src: &Path, dst: &Path) -> io::Result<()> {
    ctx.check()?;
    ctx.p.current = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    ctx.tick(false);

    let mut input = File::open(src)?;
    let md = input.metadata()?;
    let mut output = File::create(dst)?;
    let mut buf = vec![0u8; CHUNK.min(md.len().max(1) as usize)];
    let result = (|| -> io::Result<()> {
        loop {
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            output.write_all(&buf[..n])?;
            ctx.p.done_bytes += n as u64;
            ctx.tick(false);
            ctx.check()?;
        }
        output.flush()
    })();
    if let Err(e) = result {
        drop(output);
        let _ = fs::remove_file(dst);
        return Err(e);
    }
    // Zachovat cas zmeny a prava, jako to dela fs::copy. Kdyz to nejde, neni to chyba.
    if let Ok(t) = md.modified() {
        let _ = output.set_modified(t);
    }
    drop(output);
    let _ = fs::set_permissions(dst, md.permissions());
    ctx.p.done_files += 1;
    Ok(())
}

/// Zkopiruje soubor nebo cely adresar (vcetne podadresaru) na cilove jmeno.
fn copy_tree(ctx: &mut Ctx, from: &Path, target: &Path) -> io::Result<()> {
    if !from.is_dir() {
        return copy_file(ctx, from, target);
    }
    fs::create_dir_all(target)?;
    for item in walkdir::WalkDir::new(from).min_depth(1) {
        ctx.check()?;
        let item = item?;
        let rel = item.path().strip_prefix(from).map_err(io::Error::other)?;
        let dest = target.join(rel);
        if item.file_type().is_dir() {
            fs::create_dir_all(&dest)?;
        } else if item.file_type().is_symlink() {
            continue;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            copy_file(ctx, item.path(), &dest)?;
        }
    }
    Ok(())
}

fn remove_any(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

fn same_parent(from: &Path, to_dir: &Path) -> bool {
    match (from.parent(), to_dir.canonicalize()) {
        (Some(parent), Ok(dir)) => parent.canonicalize().map(|p| p == dir).unwrap_or(false),
        _ => false,
    }
}

/// Kopie nebo presun jedne polozky do slozky `to_dir`. Nazev se pri kolizi doplni cislovkou.
/// Pri chybe nebo zruseni po sobe uklidi nedokonceny cil a zdroj necha byt.
pub(crate) fn transfer_one(
    ctx: &mut Ctx,
    from: &Path,
    to_dir: &Path,
    moving: bool,
) -> io::Result<PathBuf> {
    if !from.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Zdroj neexistuje"));
    }
    if !to_dir.is_dir() {
        return Err(invalid("Cíl není složka"));
    }
    let name = from.file_name().ok_or_else(|| invalid("Nelze určit název"))?;
    let (bytes, files) = measure(from);
    let start_bytes = ctx.p.done_bytes;
    let start_files = ctx.p.done_files;

    if moving && same_parent(from, to_dir) {
        ctx.p.done_bytes += bytes;
        ctx.p.done_files += files;
        return Ok(from.to_path_buf());
    }
    if from.is_dir() {
        let src = from.canonicalize()?;
        let dir = to_dir.canonicalize()?;
        if dir.starts_with(&src) {
            return Err(invalid(if moving {
                "Složku nelze přesunout do ní samotné"
            } else {
                "Složku nelze kopírovat do ní samotné"
            }));
        }
    }
    ctx.check()?;
    let target = unique_target(to_dir, name);

    // Presun v ramci jednoho disku je jen prejmenovani - okamzite, bez kopirovani.
    if moving && fs::rename(from, &target).is_ok() {
        ctx.p.done_bytes += bytes;
        ctx.p.done_files += files;
        ctx.tick(false);
        return Ok(target);
    }

    if let Err(e) = copy_tree(ctx, from, &target) {
        remove_any(&target);
        // Postup vratit na stav pred touto polozkou, at ukazatel nelze
        ctx.p.done_bytes = start_bytes;
        ctx.p.done_files = start_files;
        return Err(e);
    }
    if moving {
        if from.is_dir() {
            fs::remove_dir_all(from)?;
        } else {
            fs::remove_file(from)?;
        }
    }
    Ok(target)
}

/// Zkopiruje nebo presune vice polozek do `to_dir`.
///
/// `on_progress` se vola nejvyse zhruba 12x za vterinu a jednou na konci.
/// Kdyz `is_cancelled` vrati true, prenos skonci: hotove polozky zustanou,
/// rozdelana se uklidi a vysledek ma `cancelled = true`.
pub fn transfer_entries<P: AsRef<Path>>(
    paths: &[P],
    to_dir: &Path,
    moving: bool,
    on_progress: &dyn Fn(&TransferProgress),
    is_cancelled: &dyn Fn() -> bool,
) -> io::Result<TransferSummary> {
    let mut ctx = Ctx::new(on_progress, is_cancelled);
    for p in paths {
        let (bytes, files) = measure(p.as_ref());
        ctx.p.total_bytes += bytes;
        ctx.p.total_files += files;
    }
    ctx.tick(true);

    let mut summary = TransferSummary {
        total_items: paths.len(),
        ..Default::default()
    };
    for p in paths {
        match transfer_one(&mut ctx, p.as_ref(), to_dir, moving) {
            Ok(target) => {
                summary.done_items += 1;
                summary.targets.push(target);
            }
            Err(_) if ctx.cancelled => {
                summary.cancelled = true;
                break;
            }
            Err(e) => {
                ctx.tick(true);
                return Err(e);
            }
        }
    }
    ctx.tick(true);
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn tree(root: &Path) {
        fs::create_dir_all(root.join("src/a/b")).unwrap();
        fs::write(root.join("src/x.bin"), vec![7u8; 3 * CHUNK + 5]).unwrap();
        fs::write(root.join("src/a/y.txt"), "ahoj").unwrap();
        fs::write(root.join("src/a/b/z.txt"), "svete").unwrap();
        fs::create_dir_all(root.join("dst")).unwrap();
    }

    #[test]
    fn kopie_hlasi_postup_a_sedi_obsah() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        tree(r);
        let last = RefCell::new(TransferProgress::default());
        let s = transfer_entries(
            &[r.join("src")],
            &r.join("dst"),
            false,
            &|p| *last.borrow_mut() = p.clone(),
            &|| false,
        )
        .unwrap();
        assert!(!s.cancelled);
        assert_eq!(s.done_items, 1);
        let p = last.borrow();
        assert_eq!(p.total_files, 3);
        assert_eq!(p.done_files, 3);
        assert_eq!(p.total_bytes, (3 * CHUNK + 5 + 4 + 5) as u64);
        assert_eq!(p.done_bytes, p.total_bytes);
        assert_eq!(fs::read(r.join("dst/src/x.bin")).unwrap().len(), 3 * CHUNK + 5);
        assert_eq!(fs::read_to_string(r.join("dst/src/a/b/z.txt")).unwrap(), "svete");
        assert!(r.join("src/x.bin").exists());
    }

    #[test]
    fn zruseni_uklidi_rozdelany_cil_a_zdroj_necha() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        tree(r);
        fs::write(r.join("maly.txt"), "m").unwrap();
        // Zrusit po prvnim zkopirovanem bloku velkeho souboru
        let calls = Cell::new(0u32);
        let cancel = || {
            calls.set(calls.get() + 1);
            calls.get() > 6
        };
        let s = transfer_entries(
            &[r.join("maly.txt"), r.join("src")],
            &r.join("dst"),
            false,
            &|_| {},
            &cancel,
        )
        .unwrap();
        assert!(s.cancelled);
        assert_eq!(s.done_items, 1);
        assert!(r.join("dst/maly.txt").exists());
        assert!(!r.join("dst/src").exists());
        assert!(r.join("src/x.bin").exists());
    }

    #[test]
    fn presun_prejmenovanim_a_kolize_nazvu() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        tree(r);
        fs::create_dir_all(r.join("dst/src")).unwrap();
        let s = transfer_entries(&[r.join("src")], &r.join("dst"), true, &|_| {}, &|| false).unwrap();
        assert_eq!(s.targets, vec![r.join("dst/src (1)")]);
        assert!(!r.join("src").exists());
        assert!(r.join("dst/src (1)/a/b/z.txt").exists());
    }
}
