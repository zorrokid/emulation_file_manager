pub fn strip_extension(s: &str) -> String {
    s.rsplit_once('.')
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| s.to_string())
}
