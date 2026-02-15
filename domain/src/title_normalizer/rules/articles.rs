pub fn normalize_articles(s: &str) -> String {
    if let Some((title, article)) = s.rsplit_once(", ")
        && matches!(article, "The" | "A" | "An")
    {
        return format!("{article} {title}");
    }
    s.to_string()
}
