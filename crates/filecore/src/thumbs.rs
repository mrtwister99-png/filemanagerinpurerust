//! Nahledy obrazku pro mrizku. Dekoduje jen ciste Rust formaty (jpeg, png, gif,
//! webp, bmp), zmensi na pozadovanou velikost a vrati JPEG bajty.

use image::{DynamicImage, ImageDecoder, ImageReader};
use std::io::{self, Cursor};
use std::path::Path;

/// Vetsi soubory se na nahled nedekoduji - zbytecne by to zralo pamet.
pub const THUMB_MAX_FILE: u64 = 64 * 1024 * 1024;

/// Pripony, pro ktere umime nahled vyrobit.
pub fn thumbnail_supported(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp"
    )
}

fn err<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// Zmenseny obrazek (delsi strana max `max_side` px) jako JPEG. Respektuje EXIF orientaci.
pub fn thumbnail_jpeg<P: AsRef<Path>>(path: P, max_side: u32) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    let md = std::fs::metadata(path)?;
    if md.len() > THUMB_MAX_FILE {
        return Err(err("Soubor je na náhled moc velký"));
    }
    let max_side = max_side.clamp(16, 1024);
    let mut decoder = ImageReader::open(path)?
        .with_guessed_format()?
        .into_decoder()
        .map_err(err)?;
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(err)?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    let thumb = if img.width() > max_side || img.height() > max_side {
        img.thumbnail(max_side, max_side)
    } else {
        img
    };
    let rgb = DynamicImage::ImageRgb8(thumb.to_rgb8());
    let mut out = Cursor::new(Vec::new());
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80);
    rgb.write_with_encoder(enc).map_err(err)?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nahled_png() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("obr.png");
        let img = image::RgbImage::from_fn(400, 200, |x, y| image::Rgb([x as u8, y as u8, 128]));
        img.save(&p).unwrap();
        let jpg = thumbnail_jpeg(&p, 100).unwrap();
        assert_eq!(&jpg[..2], &[0xFF, 0xD8]);
        let back = image::load_from_memory(&jpg).unwrap();
        assert_eq!((back.width(), back.height()), (100, 50));
        assert!(thumbnail_jpeg(t.path().join("neni.png"), 100).is_err());
    }
}
