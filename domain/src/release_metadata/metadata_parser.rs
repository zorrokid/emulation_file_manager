use regex::Regex;
use std::sync::OnceLock;

pub struct ReleaseMetadata {
    pub raw_tags: Vec<String>,
    pub countries: Vec<String>,
    pub languages: Vec<String>,
    pub version: Option<String>,
    pub is_beta: bool,
    pub is_promo: bool,
    pub is_demo: bool,
    pub is_unlicensed: bool,
}

pub fn extract_release_metadata(input: &str) -> ReleaseMetadata {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\(([^)]+)\)").unwrap());

    let raw_tags = re
        .captures_iter(input)
        .map(|cap| cap[1].trim().to_string())
        .collect();

    ReleaseMetadata {
        raw_tags,
        countries: vec![],
        languages: vec![],
        version: None,
        is_beta: false,
        is_promo: false,
        is_demo: false,
        is_unlicensed: false,
    }
}
