//! Zabaleni do ZIP a rozbaleni ZIP. Jen deflate (ciste Rust), bez C knihoven.
//! Pri rozbalovani se kontroluje "zip-slip" - zadna cesta nesmi utect mimo cilovou slozku.

use crate::ops::unique_target;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

/// Dny od 1970-01-01 -> (rok, mesic, den). Algoritmus "civil_from_days" (H. Hinnant).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Cas zmeny souboru -> ZIP DateTime (ZIP neumi casove pasmo, ukladame UTC).
fn zip_time(md: &fs::Metadata) -> DateTime {
    let secs = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, mo, d) = civil_from_days(secs.div_euclid(86_400));
    let rem = secs.rem_euclid(86_400);
    DateTime::from_date_and_time(
        y.clamp(1980, 2107) as u16,
        mo as u8,
        d as u8,
        (rem / 3600) as u8,
        (rem % 3600 / 60) as u8,
        (rem % 60) as u8,
    )
    .unwrap_or_default()
}

fn system_time(dt: DateTime) -> SystemTime {
    let days = days_from_civil(dt.year() as i64, dt.month() as u32, dt.day() as u32);
    let secs = days * 86_400
        + dt.hour() as i64 * 3600
        + dt.minute() as i64 * 60
        + dt.second() as i64;
    UNIX_EPOCH + Duration::from_secs(secs.max(0) as u64)
}

fn file_options(md: &fs::Metadata) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(zip_time(md))
        .large_file(md.len() >= 0xFFFF_FFFF)
}

/// Nazev archivu: doplni .zip a odstrani nepovolene znaky.
fn archive_file_name(name: &str) -> io::Result<String> {
    let clean = name.trim();
    if clean.is_empty() || clean.contains('/') || clean.contains('\\') || clean.contains("..") {
        return Err(invalid("Neplatný název archivu"));
    }
    if clean.to_lowercase().ends_with(".zip") {
        Ok(clean.to_string())
    } else {
        Ok(format!("{}.zip", clean))
    }
}

/// Zabali soubory a slozky (rekurzivne) do noveho ZIP v `dest_dir`.
/// Pri kolizi nazvu vznikne "archiv (1).zip". Vraci cestu k archivu.
pub fn zip_entries<P: AsRef<Path>>(paths: &[P], dest_dir: &Path, name: &str) -> io::Result<PathBuf> {
    if paths.is_empty() {
        return Err(invalid("Není co zabalit"));
    }
    if !dest_dir.is_dir() {
        return Err(invalid("Cíl není složka"));
    }
    let file_name = archive_file_name(name)?;
    let target = unique_target(dest_dir, std::ffi::OsStr::new(&file_name));
    let result = write_zip(paths, &target);
    if result.is_err() {
        let _ = fs::remove_file(&target);
    }
    result.map(|_| target)
}

fn write_zip<P: AsRef<Path>>(paths: &[P], target: &Path) -> io::Result<()> {
    let file = fs::File::create(target)?;
    let mut zip = ZipWriter::new(io::BufWriter::new(file));
    let target_abs = target.canonicalize().unwrap_or_else(|_| target.to_path_buf());
    for p in paths {
        let p = p.as_ref();
        let md = fs::symlink_metadata(p)?;
        let base = p.file_name().ok_or_else(|| invalid("Nelze určit název"))?;
        let base = base.to_string_lossy().to_string();
        if md.is_dir() {
            zip.add_directory(format!("{}/", base), file_options(&md)).map_err(io::Error::other)?;
            for item in walkdir::WalkDir::new(p).min_depth(1).follow_links(false) {
                let item = item?;
                if item.file_type().is_symlink() {
                    continue;
                }
                // Archiv sam sebe nebalime (kdyby lezel uvnitr balene slozky)
                if item.path().canonicalize().map(|c| c == target_abs).unwrap_or(false) {
                    continue;
                }
                let rel = item.path().strip_prefix(p).map_err(io::Error::other)?;
                let rel: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect();
                let zip_name = format!("{}/{}", base, rel.join("/"));
                let imd = item.metadata()?;
                if item.file_type().is_dir() {
                    zip.add_directory(format!("{}/", zip_name), file_options(&imd))
                        .map_err(io::Error::other)?;
                } else {
                    zip.start_file(zip_name, file_options(&imd)).map_err(io::Error::other)?;
                    io::copy(&mut fs::File::open(item.path())?, &mut zip)?;
                }
            }
        } else if md.is_file() {
            zip.start_file(base, file_options(&md)).map_err(io::Error::other)?;
            io::copy(&mut fs::File::open(p)?, &mut zip)?;
        }
    }
    zip.finish().map_err(io::Error::other)?;
    Ok(())
}

/// Rozbali ZIP do nove slozky pojmenovane podle archivu (vedle archivu, unikatni nazev).
/// Kdyz archiv obsahuje nebezpecnou cestu (../, absolutni), nerozbali nic a vrati chybu.
pub fn unzip_archive<P: AsRef<Path>>(archive: P) -> io::Result<PathBuf> {
    let archive = archive.as_ref();
    let parent = archive.parent().ok_or_else(|| invalid("Archiv nemá nadřazenou složku"))?;
    let stem = archive
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "archiv".to_string());
    let mut zip = ZipArchive::new(io::BufReader::new(fs::File::open(archive)?))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Nejdriv kontrola vsech cest, at po sobe nenechame polovicni slozku
    for i in 0..zip.len() {
        let f = zip.by_index_raw(i).map_err(io::Error::other)?;
        if f.enclosed_name().is_none() {
            return Err(invalid("Archiv obsahuje nebezpečnou cestu (mimo cílovou složku), nic nebylo rozbaleno"));
        }
    }

    let dest = unique_target(parent, std::ffi::OsStr::new(&stem));
    fs::create_dir(&dest)?;
    let result = (|| -> io::Result<()> {
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(io::Error::other)?;
            let rel = f.enclosed_name().ok_or_else(|| invalid("Nebezpečná cesta v archivu"))?;
            let out = dest.join(rel);
            if f.is_dir() {
                fs::create_dir_all(&out)?;
                continue;
            }
            if let Some(dir) = out.parent() {
                fs::create_dir_all(dir)?;
            }
            let mut w = fs::File::create(&out)?;
            io::copy(&mut f, &mut w)?;
            if let Some(dt) = f.last_modified() {
                let _ = w.set_modified(system_time(dt));
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_dir_all(&dest);
        return Err(e);
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn datum_tam_a_zpet() {
        for days in [0i64, 3653, 19_000, 20_000, 50_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }

    #[test]
    fn zip_tam_a_zpet() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("src/sloz/vnorena")).unwrap();
        fs::create_dir_all(r.join("src/prazdna")).unwrap();
        fs::write(r.join("src/sloz/a.txt"), "ahoj").unwrap();
        fs::write(r.join("src/sloz/vnorena/žluťoučký.txt"), "kůň").unwrap();
        fs::write(r.join("src/b.bin"), vec![7u8; 10_000]).unwrap();
        fs::create_dir(r.join("out")).unwrap();

        let paths = vec![r.join("src/sloz"), r.join("src/b.bin"), r.join("src/prazdna")];
        let z = zip_entries(&paths, &r.join("out"), "balik").unwrap();
        assert_eq!(z, r.join("out/balik.zip"));
        // Kolize nazvu -> "balik (1).zip"
        let z2 = zip_entries(&paths, &r.join("out"), "balik.zip").unwrap();
        assert_eq!(z2, r.join("out/balik (1).zip"));

        let d = unzip_archive(&z).unwrap();
        assert_eq!(d, r.join("out/balik"));
        assert_eq!(fs::read_to_string(d.join("sloz/a.txt")).unwrap(), "ahoj");
        assert_eq!(fs::read_to_string(d.join("sloz/vnorena/žluťoučký.txt")).unwrap(), "kůň");
        assert_eq!(fs::read(d.join("b.bin")).unwrap(), vec![7u8; 10_000]);
        assert!(d.join("prazdna").is_dir());
        // Druhe rozbaleni -> "balik (1)"
        assert_eq!(unzip_archive(&z).unwrap(), r.join("out/balik (1)"));
    }

    #[test]
    fn zip_slip_se_odmitne() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        let evil = r.join("zly.zip");
        {
            let mut w = ZipWriter::new(fs::File::create(&evil).unwrap());
            w.start_file("ok.txt", SimpleFileOptions::default()).unwrap();
            w.write_all(b"ok").unwrap();
            w.start_file("../utek.txt", SimpleFileOptions::default()).unwrap();
            w.write_all(b"zle").unwrap();
            w.finish().unwrap();
        }
        assert!(unzip_archive(&evil).is_err());
        assert!(!r.join("utek.txt").exists());
        assert!(!t.path().parent().unwrap().join("utek.txt").exists());
        // Zadna polovicni slozka po sobe
        assert!(!r.join("zly").exists());
    }

    #[test]
    fn spatny_nazev_archivu() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join("a.txt"), "x").unwrap();
        assert!(zip_entries(&[t.path().join("a.txt")], t.path(), "../ven").is_err());
        assert!(zip_entries(&[t.path().join("a.txt")], t.path(), "  ").is_err());
    }
}
