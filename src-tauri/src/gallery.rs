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

/// "Shuffle-bag" draw: never repeat a picture until every picture has been drawn this
/// cycle, then start a fresh random cycle. `drawn` = ids already drawn this cycle;
/// `last` = the most recently drawn id (to avoid a back-to-back repeat across cycle reset).
/// Returns (picked, new_drawn).
pub fn next_draw(all: &[ImageMeta], drawn: &[String], last: Option<&str>) -> Option<(ImageMeta, Vec<String>)> {
    use rand::seq::SliceRandom;
    if all.is_empty() {
        return None;
    }
    // Not yet drawn this cycle.
    let remaining: Vec<&ImageMeta> =
        all.iter().filter(|m| !drawn.iter().any(|d| d == &m.id)).collect();

    if remaining.is_empty() {
        // Cycle complete -> new shuffled cycle; avoid immediately repeating `last`.
        let pool: Vec<&ImageMeta> = all
            .iter()
            .filter(|m| all.len() == 1 || Some(m.id.as_str()) != last)
            .collect();
        let pick = (*pool.choose(&mut rand::thread_rng())?).clone();
        Some((pick.clone(), vec![pick.id]))
    } else {
        let pick = (*remaining.choose(&mut rand::thread_rng())?).clone();
        let mut new_drawn = drawn.to_vec();
        new_drawn.push(pick.id.clone());
        Some((pick, new_drawn))
    }
}

#[allow(dead_code)]
pub fn default_dir(resource_dir: &Path) -> PathBuf { resource_dir.join("gallery") }

#[cfg(test)]
mod tests {
    use super::*;
    fn meta(id: &str) -> ImageMeta { ImageMeta { id: id.into(), path: format!("/g/{id}") } }

    #[test]
    fn no_repeat_within_cycle() {
        let all = vec![meta("a"), meta("b"), meta("c")];
        let mut drawn: Vec<String> = vec![];
        let mut seen = vec![];
        for _ in 0..3 {
            let (pick, nd) = next_draw(&all, &drawn, drawn.last().map(|s| s.as_str())).unwrap();
            assert!(!seen.contains(&pick.id), "repeat within cycle: {}", pick.id);
            seen.push(pick.id.clone());
            drawn = nd;
        }
        assert_eq!(drawn.len(), 3); // full cycle
    }

    #[test]
    fn resets_after_full_cycle_and_avoids_back_to_back() {
        let all = vec![meta("a"), meta("b"), meta("c")];
        let last = "c";
        let drawn = vec!["a".to_string(), "b".to_string(), "c".to_string()]; // all drawn
        let (pick, nd) = next_draw(&all, &drawn, Some(last)).unwrap();
        assert_ne!(pick.id, "c", "should not repeat last across reset");
        assert_eq!(nd, vec![pick.id]); // fresh cycle
    }

    #[test]
    fn single_image_always_returns_it() {
        let all = vec![meta("only")];
        let (pick, _) = next_draw(&all, &["only".to_string()], Some("only")).unwrap();
        assert_eq!(pick.id, "only");
    }
}
