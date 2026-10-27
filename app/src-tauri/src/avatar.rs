//! Account pictures and launcher icons. Pictures live in Facet's own data
//! folder (`~/.local/share/facet/avatars/`); nothing here reads Claude's
//! folders except the public app icon the Claude package installed.

use std::fs;
use std::path::{Path, PathBuf};

use crate::adapter::linux::{FACET_HEX, ON_FACET_HEX};
use crate::error::{Error, Result};
use crate::profile::{Account, LauncherIcon};

pub const MAX_IMAGE_BYTES: usize = 2 * 1024 * 1024;

pub fn avatars_dir(home: &Path) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("facet/avatars")
}

/// Picture type from its first bytes, never from the file name.
pub fn sniff(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(("png", "image/png"));
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(("jpg", "image/jpeg"));
    }
    if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(("webp", "image/webp"));
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(("gif", "image/gif"));
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_lowercase();
    let head = head.trim_start_matches('\u{feff}').trim_start();
    if (head.starts_with("<svg") || head.starts_with("<?xml")) && head.contains("<svg") {
        return Some(("svg", "image/svg+xml"));
    }
    None
}

/// SVG pictures may not carry scripts, event handlers or outside references.
fn check_svg(bytes: &[u8]) -> Result<()> {
    let text = String::from_utf8_lossy(bytes).to_lowercase();
    let squashed: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let bad = [
        "<script",
        "javascript:",
        "<foreignobject",
        "href=\"http",
        "href='http",
        "href=\"//",
        "href='//",
        "@import",
        "url(http",
        "url(//",
        "<!entity",
    ];
    if bad.iter().any(|b| squashed.contains(b)) || has_event_handler(&text) {
        return Err(Error::Invalid(
            "This SVG has scripts or outside links, so Facet will not use it. Use a PNG or JPEG instead.".into(),
        ));
    }
    Ok(())
}

/// An attribute named `on…` (onload, onclick, …) anywhere in the markup.
fn has_event_handler(s: &str) -> bool {
    let b = s.as_bytes();
    (1..b.len().saturating_sub(2)).any(|i| {
        if !(b[i] == b'o' && b[i + 1] == b'n')
            || !(b[i - 1].is_ascii_whitespace()
                || b[i - 1] == b'/'
                || b[i - 1] == b'"'
                || b[i - 1] == b'\'')
        {
            return false;
        }
        let name = b[i + 2..]
            .iter()
            .take_while(|c| c.is_ascii_alphabetic())
            .count();
        name >= 2 && b[i + 2 + name..].iter().find(|c| !c.is_ascii_whitespace()) == Some(&b'=')
    })
}

/// Store a picture for an account and return its file name. The old file,
/// if any, is removed by the caller after the store points at the new one.
pub fn save(dir: &Path, id: &str, bytes: &[u8]) -> Result<String> {
    if bytes.is_empty() {
        return Err(Error::Invalid("The picture is empty.".into()));
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(Error::Invalid("Pick a picture under 2 MB.".into()));
    }
    let (ext, _) = sniff(bytes).ok_or_else(|| {
        Error::Invalid("Facet takes PNG, JPEG, WebP, GIF or SVG pictures.".into())
    })?;
    if ext == "svg" {
        check_svg(bytes)?;
    }
    fs::create_dir_all(dir)?;
    let name = format!("{id}-{:016x}.{ext}", fnv1a(bytes));
    let path = dir.join(&name);
    let tmp = dir.join(format!(".{name}.tmp"));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, &path)?;
    Ok(name)
}

/// Remove a stored picture. Only plain file names inside the avatars folder.
pub fn remove(dir: &Path, name: &str) {
    if is_plain_name(name) {
        let _ = fs::remove_file(dir.join(name));
    }
}

fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// Read a stored picture as `(mime, bytes)`.
pub fn load(dir: &Path, name: &str) -> Option<(&'static str, Vec<u8>)> {
    if !is_plain_name(name) {
        return None;
    }
    let path = dir.join(name);
    let meta = fs::symlink_metadata(&path).ok()?;
    if !meta.is_file() || meta.len() as usize > MAX_IMAGE_BYTES {
        return None;
    }
    let bytes = fs::read(&path).ok()?;
    let (_, mime) = sniff(&bytes)?;
    Some((mime, bytes))
}

pub fn data_url(dir: &Path, name: &str) -> Option<String> {
    let (mime, bytes) = load(dir, name)?;
    Some(format!("data:{mime};base64,{}", base64(&bytes)))
}

pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn base64_decode(s: &str) -> Result<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let clean: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        let pad = chunk.iter().rev().take_while(|c| **c == b'=').count();
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            let v = if *c == b'=' {
                0
            } else {
                val(*c).ok_or_else(|| Error::Invalid("The picture data is damaged.".into()))?
            };
            n |= v << (18 - 6 * i);
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(
            &bytes[..3usize
                .saturating_sub(pad)
                .min(chunk.len().saturating_sub(1))],
        );
    }
    Ok(out)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// The Claude app icon installed by the official package (public file).
pub fn installed_claude_icon() -> Option<Vec<u8>> {
    for size in ["256x256", "512x512", "128x128"] {
        for root in ["/usr/share/icons/hicolor", "/usr/local/share/icons/hicolor"] {
            let p = Path::new(root).join(size).join("apps/claude-desktop.png");
            if let Ok(b) = fs::read(&p) {
                if sniff(&b).map(|s| s.0) == Some("png") {
                    return Some(b);
                }
            }
        }
    }
    None
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// What an avatar shows when there is no picture: the badge, else the first
/// letter or digit of the name.
pub fn mark(account: &Account) -> String {
    account
        .badge
        .clone()
        .filter(|b| !b.trim().is_empty())
        .unwrap_or_else(|| {
            account
                .name
                .chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_uppercase().collect())
                .unwrap_or_else(|| "?".into())
        })
}

/// Launcher and dock icon as SVG. `picture` is the account picture, `claude`
/// the installed Claude icon (used only for the `ClaudeBadge` style).
pub fn launcher_svg(
    account: &Account,
    picture: Option<(&str, &[u8])>,
    claude: Option<&[u8]>,
) -> String {
    let i = (account.color.clamp(1, 6) - 1) as usize;
    let (bg, fg) = (FACET_HEX[i], ON_FACET_HEX[i]);
    let text = escape(&mark(account));
    let long = text.chars().count() > 1;
    let mut s = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" viewBox=\"0 0 128 128\">",
    );
    match (account.launcher_icon, claude) {
        (LauncherIcon::ClaudeBadge, Some(png)) => {
            s.push_str(&format!(
                "<image x=\"0\" y=\"0\" width=\"128\" height=\"128\" xlink:href=\"data:image/png;base64,{}\"/>",
                base64(png)
            ));
            s.push_str(&format!(
                "<circle cx=\"96\" cy=\"96\" r=\"31\" fill=\"#FFFFFF\"/><circle cx=\"96\" cy=\"96\" r=\"27\" fill=\"{bg}\"/>"
            ));
            match picture {
                Some((mime, bytes)) => s.push_str(&format!(
                    "<clipPath id=\"b\"><circle cx=\"96\" cy=\"96\" r=\"27\"/></clipPath><image x=\"69\" y=\"69\" width=\"54\" height=\"54\" preserveAspectRatio=\"xMidYMid slice\" clip-path=\"url(#b)\" xlink:href=\"data:{mime};base64,{}\"/>",
                    base64(bytes)
                )),
                None => s.push_str(&format!(
                    "<text x=\"96\" y=\"{y}\" font-family=\"sans-serif\" font-size=\"{fs}\" font-weight=\"700\" text-anchor=\"middle\" fill=\"{fg}\">{text}</text>",
                    y = if long { 104 } else { 108 },
                    fs = if long { 22 } else { 32 },
                )),
            }
        }
        _ => {
            s.push_str(&format!(
                "<clipPath id=\"r\"><rect x=\"8\" y=\"8\" width=\"112\" height=\"112\" rx=\"30\"/></clipPath><rect x=\"8\" y=\"8\" width=\"112\" height=\"112\" rx=\"30\" fill=\"{bg}\"/>"
            ));
            match picture {
                Some((mime, bytes)) => s.push_str(&format!(
                    "<image x=\"8\" y=\"8\" width=\"112\" height=\"112\" preserveAspectRatio=\"xMidYMid slice\" clip-path=\"url(#r)\" xlink:href=\"data:{mime};base64,{}\"/>",
                    base64(bytes)
                )),
                None => s.push_str(&format!(
                    "<text x=\"64\" y=\"{y}\" font-family=\"sans-serif\" font-size=\"{fs}\" font-weight=\"700\" text-anchor=\"middle\" fill=\"{fg}\">{text}</text>",
                    y = if long { 78 } else { 84 },
                    fs = if long { 40 } else { 60 },
                )),
            }
        }
    }
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for n in 0..20u8 {
            let data: Vec<u8> = (0..n).map(|i| i.wrapping_mul(37)).collect();
            assert_eq!(base64_decode(&base64(&data)).unwrap(), data, "{n}");
        }
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
    }

    #[test]
    fn sniffs_by_content_and_rejects_unsafe_svg() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\nxxxx").unwrap().0, "png");
        assert_eq!(
            sniff(b"<?xml version=\"1.0\"?><svg></svg>").unwrap().0,
            "svg"
        );
        assert!(sniff(b"#!/bin/sh\necho").is_none());
        assert!(check_svg(b"<svg><circle r=\"1\"/></svg>").is_ok());
        assert!(check_svg(b"<svg><script>x</script></svg>").is_err());
        assert!(check_svg(b"<svg onload=\"x()\"></svg>").is_err());
        assert!(check_svg(b"<svg><image href=\"https://x/y.png\"/></svg>").is_err());
        assert!(check_svg(b"<svg><g font=\"none\"/></svg>").is_ok());
    }

    #[test]
    fn save_and_load_stay_inside_the_avatars_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("avatars");
        let home = home.as_path();
        let name = save(home, "work", b"\x89PNG\r\n\x1a\nabc").unwrap();
        assert!(name.starts_with("work-") && name.ends_with(".png"));
        assert!(data_url(home, &name)
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(load(home, "../profiles.toml").is_none());
        assert!(save(home, "work", b"plain text").is_err());
        remove(home, &name);
        assert!(load(home, &name).is_none());
    }
}
