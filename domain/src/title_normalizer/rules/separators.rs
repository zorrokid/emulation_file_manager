use std::sync::OnceLock;

use regex::Regex;

pub fn replace_separators(s: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[&]").unwrap());

    re.replace_all(s, " and ").to_string()
}
