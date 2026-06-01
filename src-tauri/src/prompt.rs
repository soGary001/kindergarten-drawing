const STYLE_SUFFIX: &str =
    "cute children's book illustration, soft pastel colors, friendly, simple, clean background";

/// Child's literal words + a fixed hidden style suffix. Content comes only from the child.
pub fn build_prompt(transcript: &str) -> String {
    let t = transcript.trim();
    if t.is_empty() { STYLE_SUFFIX.to_string() } else { format!("{t}, {STYLE_SUFFIX}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appends_suffix() {
        assert_eq!(build_prompt("a pink pig in mud"),
            "a pink pig in mud, cute children's book illustration, soft pastel colors, friendly, simple, clean background");
    }
    #[test]
    fn empty_is_suffix_only() {
        assert_eq!(build_prompt("   "), STYLE_SUFFIX);
    }
}
