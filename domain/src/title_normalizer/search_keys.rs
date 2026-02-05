use crate::title_normalizer::rules::{
    punctuation::remove_non_alphanumeric, separators::replace_separators,
    whitespace::normalize_whitespace,
};

pub fn normalize_for_search(canonical: &str) -> String {
    let s = canonical.to_lowercase();
    let s = replace_separators(&s);
    let s = remove_non_alphanumeric(&s);
    let s = normalize_whitespace(&s);
    s.trim().to_string()
}

pub fn generate_search_keys(normalized: &str) -> Vec<String> {
    let mut keys = Vec::new();
    
    // If the normalized string has no spaces, check if it's single-word-with-punctuation
    let spaced = normalized.to_string();
    let collapsed = normalized.replace(' ', "");
    
    // For strings with spaces, generate both spaced and collapsed versions
    if spaced != collapsed {
        keys.push(spaced);
        keys.push(collapsed);
    } else {
        // Single word - just add it once
        keys.push(spaced);
    }
    
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_normalize_for_search() {
        let test_cases = vec![
            ("A.E.", "ae"),
            ("Bump 'n' Jump", "bump n jump"),
            ("Choplifter!", "choplifter"),
            (
                "Dr. Seuss - Fix-Up the Mix-Up Puzzler",
                "dr seuss fixup the mixup puzzler",
            ),
            ("Energy Quiz", "energy quiz"),
            ("Frogger II - ThreeeDeep!", "frogger ii threeedeep"),
            ("Front Line", "front line"),
            ("Rock 'n' Roll", "rock n roll"),
            ("Cats & Dogs", "cats and dogs"),
        ];
        for (input, expected) in test_cases {
            let normalized = normalize_for_search(input);
            assert_eq!(normalized, expected.to_string());
        }
    }
}
