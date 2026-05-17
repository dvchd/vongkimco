use std::path::PathBuf;

use anyhow::{anyhow, Result};
use image::{codecs::jpeg::JpegEncoder, ColorType, ImageEncoder};

/// Capture the primary monitor, downscale to a privacy-friendly size, and encode as JPEG.
pub fn capture_primary_jpeg(out_dir: &std::path::Path, quality: u8, max_width: u32) -> Result<(PathBuf, usize, u32, u32)> {
    let monitors = xcap::Monitor::all().map_err(|e| anyhow!("xcap: {e}"))?;
    let mon = monitors
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no monitor found"))?;

    let img = mon.capture_image().map_err(|e| anyhow!("xcap capture: {e}"))?;

    // img is `image::ImageBuffer<image::Rgba<u8>, Vec<u8>>`
    let (w, h) = (img.width(), img.height());

    // Convert RGBA -> RGB and downscale
    let dyn_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let dyn_img = image::DynamicImage::ImageRgb8(dyn_img);
    let new_w = if w > max_width { max_width } else { w };
    let new_h = ((h as f64) * (new_w as f64) / (w as f64)).round() as u32;
    let resized = dyn_img.resize(new_w, new_h, image::imageops::FilterType::Triangle);
    let rgb8 = resized.to_rgb8();

    let id = uuid::Uuid::new_v4().to_string();
    let dt = chrono::Utc::now().format("%Y%m%d").to_string();
    let folder = out_dir.join(&dt);
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(format!("{}.jpg", id));

    let mut out = std::fs::File::create(&path)?;
    let encoder = JpegEncoder::new_with_quality(&mut out, quality);
    encoder.write_image(rgb8.as_raw(), new_w, new_h, ColorType::Rgb8.into())?;
    drop(out);
    let bytes = std::fs::metadata(&path)?.len() as usize;
    Ok((path, bytes, new_w, new_h))
}
