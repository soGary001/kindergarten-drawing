use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ImageMeta {
    pub id: String,    // file name
    pub path: String,  // absolute path
}

const EXTS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

pub fn scan(dir: &Path) -> Vec<ImageMeta> {
    let mut out = vec![];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            let is_img = p.extension()
                .and_then(|x| x.to_str())
                .map(|x| EXTS.contains(&x.to_lowercase().as_str()))
                .unwrap_or(false);
            if is_img {
                if let (Some(name), Some(path)) = (p.file_name().and_then(|n| n.to_str()), p.to_str()) {
                    out.push(ImageMeta { id: name.to_string(), path: path.to_string() });
                }
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Indices of `all` that are NOT in `recent` (so the same card isn't drawn twice in a row).
/// Falls back to ALL indices if everything is recent.
pub fn candidate_indices(all_len: usize, recent: &[usize]) -> Vec<usize> {
    let avail: Vec<usize> = (0..all_len).filter(|i| !recent.contains(i)).collect();
    if avail.is_empty() { (0..all_len).collect() } else { avail }
}

pub fn draw_random(all: &[ImageMeta], recent: &[usize]) -> Option<(usize, ImageMeta)> {
    use rand::seq::SliceRandom;
    let cands = candidate_indices(all.len(), recent);
    let idx = *cands.choose(&mut rand::thread_rng())?;
    Some((idx, all[idx].clone()))
}

#[allow(dead_code)]
pub fn default_dir(resource_dir: &Path) -> PathBuf { resource_dir.join("gallery") }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidates_exclude_recent() {
        assert_eq!(candidate_indices(5, &[1, 3]), vec![0, 2, 4]);
    }
    #[test]
    fn candidates_fallback_when_all_recent() {
        assert_eq!(candidate_indices(3, &[0, 1, 2]), vec![0, 1, 2]);
    }
}
