//! "Analyza mista" - kolik zabiraji jednotlive polozky slozky (rekurzivne, paralelne).
//! Symlinky (a na Windows junctiony) se nenasleduji, aby se nic nepocitalo dvakrat.

use crate::format::format_size;
use crate::models::SizeEntry;
use rayon::prelude::*;
use std::fs;
use std::path::Path;

/// (bajty, pocet souboru) pro slozku vcetne podslozek. Pri zruseni vrati, co stihla.
pub fn dir_size(dir: &Path, is_cancelled: &(dyn Fn() -> bool + Sync)) -> (u64, u64) {
    if is_cancelled() {
        return (0, 0);
    }
    let Ok(rd) = fs::read_dir(dir) else { return (0, 0) };
    let items: Vec<fs::DirEntry> = rd.flatten().collect();
    items
        .par_iter()
        .map(|item| {
            let Ok(ft) = item.file_type() else { return (0, 0) };
            if ft.is_symlink() {
                (0, 0)
            } else if ft.is_dir() {
                dir_size(&item.path(), is_cancelled)
            } else {
                (item.metadata().map(|m| m.len()).unwrap_or(0), 1)
            }
        })
        .reduce(|| (0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
}

/// Velikosti primych potomku slozky, nejvetsi prvni.
pub fn child_sizes<P: AsRef<Path>>(
    dir: P,
    is_cancelled: &(dyn Fn() -> bool + Sync),
) -> std::io::Result<Vec<SizeEntry>> {
    let items: Vec<fs::DirEntry> = fs::read_dir(dir.as_ref())?.flatten().collect();
    let mut out: Vec<SizeEntry> = items
        .par_iter()
        .filter_map(|item| {
            let ft = item.file_type().ok()?;
            let name = item.file_name().to_string_lossy().to_string();
            let path = item.path();
            let (size, files, is_dir) = if ft.is_symlink() {
                (0, 0, false)
            } else if ft.is_dir() {
                let (s, f) = dir_size(&path, is_cancelled);
                (s, f, true)
            } else {
                (item.metadata().map(|m| m.len()).unwrap_or(0), 1, false)
            };
            Some(SizeEntry { name, path, is_dir, size, size_human: format_size(size), files })
        })
        .collect();
    out.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velikosti_slozek() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("velka/vnorena")).unwrap();
        fs::create_dir_all(r.join("prazdna")).unwrap();
        fs::write(r.join("velka/a.bin"), vec![0u8; 3000]).unwrap();
        fs::write(r.join("velka/vnorena/b.bin"), vec![0u8; 2000]).unwrap();
        fs::write(r.join("maly.txt"), vec![0u8; 10]).unwrap();
        let v = child_sizes(r, &|| false).unwrap();
        let names: Vec<&str> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["velka", "maly.txt", "prazdna"]);
        assert_eq!(v[0].size, 5000);
        assert_eq!(v[0].files, 2);
        assert!(v[0].is_dir);
        assert_eq!(v[1].size, 10);
        assert_eq!(dir_size(r, &|| false), (5010, 3));
        assert_eq!(dir_size(r, &|| true), (0, 0));
    }
}
