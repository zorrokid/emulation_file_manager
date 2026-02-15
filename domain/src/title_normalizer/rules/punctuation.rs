use std::sync::OnceLock;

use regex::Regex;

pub fn normalize_punctuation(s: &str) -> String {
    s.replace(" - ", ": ").replace("'", "’")
}

/// Removes all non-alphanumeric characters from the input string `s`.
/// Alphanumeric characters include letters and numbers from all Unicode scripts.
/// Regular spaces are preserved.
pub fn remove_non_alphanumeric(s: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[^\p{L}\p{N} ]+").unwrap());
    re.replace_all(s, "").to_string()
}
