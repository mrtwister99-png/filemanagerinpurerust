//! Rekurzivni hledani s filtry (typ, velikost, datum, obsah).
//! Strom se neprochazi cely do pameti - walkdir ho postupne generuje a rayon
//! (par_bridge) zpracovava polozky paralelne, jak prichazeji.

use crate::format::extension_of;
use crate::models::{FileEntry, FileKind, SearchResult};
use crate::scanner::to_entry;
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::Deserialize;
use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Hledani v obsahu jen u souboru do teto velikosti.
pub const CONTENT_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SearchOptions {
    /// Text, ktery se hleda v nazvu (a pripadne v obsahu). Bez ohledu na velikost pismen.
    pub query: String,
    /// Prazdne = libovolny typ. Pri nastavenem typu se slozky nevraci.
    pub kinds: Vec<FileKind>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    /// Unix cas - jen polozky zmenene od tohoto okamziku
    pub modified_after: Option<u64>,
    /// Hledat i v obsahu textovych souboru (do CONTENT_MAX_BYTES, binarni se preskakuji)
    pub content: bool,
    pub max_depth: usize,
    pub max_results: usize,
    pub show_hidden: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            query: String::new(),
            kinds: Vec::new(),
            min_size: None,
            max_size: None,
            modified_after: None,
            content: false,
            max_depth: 12,
            max_results: 500,
            show_hidden: false,
        }
    }
}

impl SearchOptions {
    fn has_file_filter(&self) -> bool {
        !self.kinds.is_empty() || self.min_size.is_some() || self.max_size.is_some()
    }
}

fn is_hidden_name(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

fn modified_unix(md: &std::fs::Metadata) -> u64 {
    md.modified()
        .unwrap_or(SystemTime::now())
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Obsahuje textovy soubor hledany retezec? Binarni soubory (s nulovym bajtem) preskoci.
fn file_contains(path: &Path, needle_lc: &str) -> bool {
    let Ok(file) = std::fs::File::open(path) else { return false };
    let mut data = Vec::new();
    if file.take(CONTENT_MAX_BYTES).read_to_end(&mut data).is_err() {
        return false;
    }
    let head = &data[..data.len().min(8192)];
    if head.contains(&0) {
        return false;
    }
    String::from_utf8_lossy(&data).to_lowercase().contains(needle_lc)
}

fn match_entry(entry: &walkdir::DirEntry, opts: &SearchOptions, q: &str) -> Option<FileEntry> {
    let name = entry.file_name().to_string_lossy();
    let name_hit = q.is_empty() || name.to_lowercase().contains(q);
    let is_dir = entry.file_type().is_dir();
    if is_dir && (opts.has_file_filter() || !name_hit) {
        // Slozky jen podle nazvu a jen kdyz neni filtr typu / velikosti
        return None;
    }
    if !name_hit && !opts.content {
        return None;
    }
    if !opts.show_hidden && is_hidden_name(entry.file_name()) {
        return None;
    }
    let md = entry.metadata().ok()?;
    if !is_dir {
        if !opts.kinds.is_empty() {
            let kind = FileKind::from_path(false, &extension_of(&name));
            if !opts.kinds.contains(&kind) {
                return None;
            }
        }
        if opts.min_size.is_some_and(|min| md.len() < min) {
            return None;
        }
        if opts.max_size.is_some_and(|max| md.len() > max) {
            return None;
        }
    }
    if opts.modified_after.is_some_and(|t| modified_unix(&md) < t) {
        return None;
    }
    if !name_hit {
        // Nazev nesedi, zkusime obsah
        if !md.is_file() || md.len() > CONTENT_MAX_BYTES || !file_contains(entry.path(), q) {
            return None;
        }
    }
    Some(to_entry(entry.path().to_path_buf(), md))
}

/// Hledani s filtry. `is_cancelled` se vola prubezne - kdyz vrati true (napr. prisel
/// novejsi dotaz), hledani skonci a vrati, co ma, s `cancelled = true`.
pub fn search_with<P: AsRef<Path>>(
    dir: P,
    opts: &SearchOptions,
    is_cancelled: &(dyn Fn() -> bool + Sync),
) -> SearchResult {
    let q = opts.query.trim().to_lowercase();
    if q.is_empty() && !opts.has_file_filter() && opts.modified_after.is_none() {
        return SearchResult { hits: Vec::new(), truncated: false, cancelled: false };
    }
    let show_hidden = opts.show_hidden;
    let max_results = opts.max_results.max(1);
    let hits: Mutex<Vec<FileEntry>> = Mutex::new(Vec::new());
    let count = AtomicUsize::new(0);
    let truncated = AtomicBool::new(false);
    let cancelled = AtomicBool::new(false);

    let walker = walkdir::WalkDir::new(dir.as_ref())
        .min_depth(1)
        .max_depth(opts.max_depth.max(1))
        .follow_links(false)
        .into_iter()
        // Skryte slozky (.git, .cache...) se bez "Skryte" vubec neprochazeji
        .filter_entry(move |e| e.depth() == 0 || show_hidden || !is_hidden_name(e.file_name()));

    let _ = walker.par_bridge().try_for_each(|item| -> Result<(), ()> {
        if is_cancelled() {
            cancelled.store(true, Ordering::Relaxed);
            return Err(());
        }
        let Ok(entry) = item else { return Ok(()) };
        if let Some(hit) = match_entry(&entry, opts, &q) {
            if count.fetch_add(1, Ordering::Relaxed) >= max_results {
                truncated.store(true, Ordering::Relaxed);
                return Err(());
            }
            hits.lock().unwrap().push(hit);
        }
        Ok(())
    });

    let mut hits = hits.into_inner().unwrap_or_default();
    hits.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    SearchResult {
        hits,
        truncated: truncated.load(Ordering::Relaxed),
        cancelled: cancelled.load(Ordering::Relaxed),
    }
}

/// Jednoduche hledani jen podle nazvu (puvodni API, pouziva ho CLI).
pub fn search<P: AsRef<Path>>(dir: P, query: &str, max_results: usize) -> Vec<FileEntry> {
    let opts = SearchOptions {
        query: query.to_string(),
        max_results,
        ..SearchOptions::default()
    };
    search_with(dir, &opts, &|| false).hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("a/b/c")).unwrap();
        fs::create_dir_all(r.join(".skryta")).unwrap();
        fs::write(r.join("fotka.jpg"), vec![1u8; 2000]).unwrap();
        fs::write(r.join("a/poznamky.txt"), "nakoupit mleko a chleba").unwrap();
        fs::write(r.join("a/b/c/hluboko.txt"), "nic").unwrap();
        fs::write(r.join("a/b/binarni.dat"), b"mleko\0\0\0").unwrap();
        fs::write(r.join(".skryta/fotka2.jpg"), b"x").unwrap();
        fs::write(r.join("video.mp4"), vec![0u8; 50]).unwrap();
        t
    }

    fn names(r: &SearchResult) -> Vec<String> {
        let mut v: Vec<String> = r.hits.iter().map(|h| h.name.clone()).collect();
        v.sort();
        v
    }

    #[test]
    fn hledani_podle_nazvu_a_skrytych() {
        let t = setup();
        let mut o = SearchOptions { query: "FOTKA".into(), ..Default::default() };
        assert_eq!(names(&search_with(t.path(), &o, &|| false)), vec!["fotka.jpg"]);
        o.show_hidden = true;
        assert_eq!(
            names(&search_with(t.path(), &o, &|| false)),
            vec!["fotka.jpg", "fotka2.jpg"]
        );
    }

    #[test]
    fn filtr_typu_velikosti_a_hloubky() {
        let t = setup();
        let o = SearchOptions { kinds: vec![FileKind::Image], ..Default::default() };
        assert_eq!(names(&search_with(t.path(), &o, &|| false)), vec!["fotka.jpg"]);
        let o = SearchOptions { min_size: Some(100), ..Default::default() };
        assert_eq!(names(&search_with(t.path(), &o, &|| false)), vec!["fotka.jpg"]);
        let o = SearchOptions { max_size: Some(60), query: "o".into(), ..Default::default() };
        assert_eq!(
            names(&search_with(t.path(), &o, &|| false)),
            vec!["hluboko.txt", "poznamky.txt", "video.mp4"]
        );
        let o = SearchOptions { query: "hluboko".into(), max_depth: 2, ..Default::default() };
        assert!(search_with(t.path(), &o, &|| false).hits.is_empty());
        let o = SearchOptions { modified_after: Some(u64::MAX / 2), query: "a".into(), ..Default::default() };
        assert!(search_with(t.path(), &o, &|| false).hits.is_empty());
    }

    #[test]
    fn hledani_v_obsahu_preskoci_binarni() {
        let t = setup();
        let mut o = SearchOptions { query: "mleko".into(), ..Default::default() };
        assert!(search_with(t.path(), &o, &|| false).hits.is_empty());
        o.content = true;
        assert_eq!(names(&search_with(t.path(), &o, &|| false)), vec!["poznamky.txt"]);
    }

    #[test]
    fn limit_a_zruseni() {
        let t = setup();
        let o = SearchOptions { query: "o".into(), max_results: 2, ..Default::default() };
        let r = search_with(t.path(), &o, &|| false);
        assert_eq!(r.hits.len(), 2);
        assert!(r.truncated);
        let r = search_with(t.path(), &o, &|| true);
        assert!(r.cancelled);
        assert!(r.hits.is_empty());
    }
}
