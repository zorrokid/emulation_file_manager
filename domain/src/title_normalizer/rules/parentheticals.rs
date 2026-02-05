use regex::Regex;
use std::sync::OnceLock;

pub fn remove_parentheticals(s: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\s*\([^)]*\)").unwrap());

    re.replace_all(s, "").to_string()
}
