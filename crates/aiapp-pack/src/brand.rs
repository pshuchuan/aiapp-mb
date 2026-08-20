//! Brand customization: derive brand info from the `.aiapp` manifest, overridable from the command line;
//! and generate a branded placeholder icon (pure Rust PNG encoding, no external font dependency).

use std::path::{Path, PathBuf};

use aiapp_format::AppManifest;

/// Brand info (the "face" an app presents to the outside world).
#[derive(Debug, Clone)]
pub struct Brand {
    /// Display name (brand name). Defaults to the manifest `name`.
    pub name: String,
    /// Unique identifier (e.g. `com.example.hello`). Defaults to `aiapp.<app_id>`.
    pub identifier: String,
    /// App version (semantic). Defaults to the manifest `version`.
    pub version: String,
    /// App description.
    pub description: String,
    /// Author / developer.
    pub author: String,
    /// Homepage / user custom domain (a brand customization item).
    pub homepage: Option<String>,
    /// Optional custom icon (copied into the shell project if present; otherwise a branded placeholder icon is generated).
    pub icon: Option<PathBuf>,
}

impl Brand {
    /// Build a brand from a manifest + optional overrides.
    ///
    /// Fields that are `None` in `overrides` fall back to the manifest/defaults.
    pub fn from_manifest(
        m: &AppManifest,
        overrides: &BrandOverrides,
    ) -> Result<Self, String> {
        let name = overrides
            .name
            .clone()
            .unwrap_or_else(|| m.name.clone());
        let identifier = overrides
            .identifier
            .clone()
            .unwrap_or_else(|| format!("aiapp.{}", default_identifier(m)));
        let version = overrides.version.clone().unwrap_or_else(|| m.version.clone());
        let description = overrides
            .description
            .clone()
            .unwrap_or_else(|| m.description.clone());
        let author = overrides.author.clone().unwrap_or_default();
        let homepage = overrides.homepage.clone();
        let icon = overrides.icon.clone();

        if !valid_identifier(&identifier) {
            return Err(format!(
                "Invalid identifier `{identifier}`: should look like com.example.hello (lowercase alphanumeric + dot-separated segments)"
            ));
        }
        if name.trim().is_empty() {
            return Err("Brand name must not be empty".into());
        }

        Ok(Brand {
            name,
            identifier,
            version,
            description,
            author,
            homepage,
            icon,
        })
    }

    /// Derive a stable primary color from the brand name (used for the placeholder icon / shell theme).
    pub fn color(&self) -> (u8, u8, u8) {
        color_from_name(&self.name)
    }
}

/// Brand fields overridable from the command line (`None` = use the default).
#[derive(Debug, Clone, Default)]
pub struct BrandOverrides {
    pub name: Option<String>,
    pub identifier: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub icon: Option<PathBuf>,
}

/// Derive a valid identifier segment from the app_id: `app_12345678` → `app12345678` (underscores removed).
fn default_identifier(m: &AppManifest) -> String {
    m.app_id.replace('_', "")
}

/// Identifier validity: `^[a-z0-9]+(\.[a-z0-9]+)*$` (reverse-DNS style).
fn valid_identifier(id: &str) -> bool {
    let mut parts = 0;
    for seg in id.split('.') {
        if seg.is_empty() || !seg.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return false;
        }
        parts += 1;
    }
    parts >= 2
}

/// Derive a stable, readable primary color by hashing the name.
fn color_from_name(name: &str) -> (u8, u8, u8) {
    let mut h: u64 = 0x811c9dc5;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x01000193);
    }
    // Map into a warm/neutral readable range: hue 20°..=40° (orange-brown) + lightness 40%..=70%
    let hue = 18.0 + (h % 18) as f64; // 18..36°
    let light = 0.45 + ((h >> 16) % 20) as f64 / 100.0; // 0.45..0.64
    let (r, g, b) = hsl_to_rgb(hue, 0.55, light);
    (r, g, b)
}

/// HSL → RGB (h∈[0,360), s,l∈[0,1]).
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

/// Generate a branded placeholder icon (512×512 RGBA PNG).
///
/// Graphic: rounded-square background (brand primary color) + centered white ring (abstract "a"/app orb) +
/// a small dot at the top right of the ring, with 2×2 supersampling anti-aliasing for a smooth look.
/// Written to `dest`.
pub fn generate_icon(dest: &Path, brand: &Brand) -> Result<(), String> {
    const SIZE: u32 = 512;
    const AA: u32 = 2; // 2×2 subsamples per pixel
    let (br, bg, bb) = brand.color();

    let mut data = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut r = 0.0f64;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut a = 0.0;
            for sy in 0..AA {
                for sx in 0..AA {
                    let px = x as f64 + (sx as f64 + 0.5) / AA as f64;
                    let py = y as f64 + (sy as f64 + 0.5) / AA as f64;
                    let (cr, cg, cb, ca) = sample(px, py, SIZE, br, bg, bb);
                    r += cr * ca;
                    g += cg * ca;
                    b += cb * ca;
                    a += ca;
                }
            }
            let inv = 1.0 / (AA * AA) as f64;
            let idx = ((y * SIZE + x) * 4) as usize;
            data[idx] = (r * inv) as u8;
            data[idx + 1] = (g * inv) as u8;
            data[idx + 2] = (b * inv) as u8;
            data[idx + 3] = (a * inv * 255.0) as u8;
        }
    }

    let file = std::fs::File::create(dest).map_err(|e| format!("Failed to create icon file: {e}"))?;
    let mut enc = png::Encoder::new(file, SIZE, SIZE);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| format!("PNG header failed: {e}"))?;
    writer
        .write_image_data(&data)
        .map_err(|e| format!("PNG data write failed: {e}"))?;
    Ok(())
}

/// Color and coverage of a single subsample point (RGBA each in 0..1).
fn sample(x: f64, y: f64, size: u32, br: u8, bg: u8, bb: u8) -> (f64, f64, f64, f64) {
    let s = size as f64;
    // 1) Rounded-square background (only drawn inside the square; transparent outside)
    let corner = 0.22 * s;
    let inside = inside_round_rect(x, y, corner, s);
    if !inside {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let c = s / 2.0;

    // 2) White ring (abstract "app orb", centered)
    let outer = 0.36 * s;
    let inner = 0.22 * s;
    let d = ((x - c).powi(2) + (y - c).powi(2)).sqrt();
    if d <= outer && d >= inner {
        return (1.0, 1.0, 1.0, 1.0);
    }

    // 3) Small dot at the top right of the ring (brand accent)
    let dot_cx = c + 0.42 * s;
    let dot_cy = c - 0.42 * s;
    let dd = ((x - dot_cx).powi(2) + (y - dot_cy).powi(2)).sqrt();
    if dd <= 0.075 * s {
        return (1.0, 1.0, 1.0, 1.0);
    }

    (br as f64 / 255.0, bg as f64 / 255.0, bb as f64 / 255.0, 1.0)
}

/// Whether a point falls inside the rounded rectangle (coverage approximated as 0/1).
fn inside_round_rect(x: f64, y: f64, corner: f64, s: f64) -> bool {
    if x < 0.0 || y < 0.0 || x > s || y > s {
        return false;
    }
    let c = corner;
    let r = c;
    // Check each of the four corners
    let cx = if x < c { c } else if x > s - c { s - c } else { x };
    let cy = if y < c { c } else if y > s - c { s - c } else { y };
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= r * r
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiapp_format::AppManifest;

    #[test]
    fn brand_defaults_from_manifest() {
        let m = AppManifest::new("My Todo", "todo", "my_todo");
        let b = Brand::from_manifest(&m, &BrandOverrides::default()).unwrap();
        assert_eq!(b.name, "my todo");
        assert_eq!(b.identifier, format!("aiapp.{}", m.app_id.replace('_', "")));
        assert_eq!(b.version, m.version);
        assert_eq!(b.color(), color_from_name("my todo"));
    }

    #[test]
    fn overrides_win() {
        let m = AppManifest::new("x", "minimal", "x");
        let o = BrandOverrides {
            name: Some("My Brand".into()),
            identifier: Some("com.example.hello".into()),
            version: Some("9.9.9".into()),
            author: Some("me".into()),
            homepage: Some("https://example.com".into()),
            ..Default::default()
        };
        let b = Brand::from_manifest(&m, &o).unwrap();
        assert_eq!(b.name, "My Brand");
        assert_eq!(b.identifier, "com.example.hello");
        assert_eq!(b.version, "9.9.9");
        assert_eq!(b.homepage.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn rejects_bad_identifier() {
        let m = AppManifest::new("x", "minimal", "x");
        let o = BrandOverrides {
            identifier: Some("Bad Identifier!".into()),
            ..Default::default()
        };
        assert!(Brand::from_manifest(&m, &o).is_err());
        // A single segment is also invalid
        let o2 = BrandOverrides {
            identifier: Some("hello".into()),
            ..Default::default()
        };
        assert!(Brand::from_manifest(&m, &o2).is_err());
    }

    #[test]
    fn icon_png_valid() {
        let m = AppManifest::new("Demo App", "todo", "demo_app");
        let b = Brand::from_manifest(&m, &BrandOverrides::default()).unwrap();
        let dir = std::env::temp_dir().join(format!("aiapp-pack-icon-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("icon.png");
        generate_icon(&p, &b).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        // PNG magic number
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
