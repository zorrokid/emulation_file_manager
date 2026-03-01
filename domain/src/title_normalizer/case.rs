const SMALL_WORDS: &[&str] = &[
    "the", "and", "of", "in", "to", "a", "is", "that", "it", "with", "as", "for", "was", "on",
    "are", "by", "this", "be", "or",
];

pub fn title_case(s: &str) -> String {
    use regex::Regex;

    // Split on subtitle delimiters (dash or colon) while preserving them
    let delimeter_regex = Regex::new(r"\s[-–]\s|:\s").unwrap();

    let mut result = String::new();
    let mut last_end = 0;

    for delimiter_match in delimeter_regex.find_iter(s) {
        // Process the text before the delimiter
        let part = &s[last_end..delimiter_match.start()];
        result.push_str(&title_case_part(part));

        // Add the delimiter as-is
        result.push_str(delimiter_match.as_str());

        last_end = delimiter_match.end();
    }

    // Process the final part after the last delimiter
    let final_part = &s[last_end..];
    result.push_str(&title_case_part(final_part));

    result
}

fn title_case_part(s: &str) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    let len = words.len();
    words
        .iter()
        .enumerate()
        .map(|(i, &w)| {
            println!("Processing word: '{}'", w);
            if (i == 0 || i == len - 1) || !SMALL_WORDS.contains(&w.to_lowercase().as_str()) {
                println!("Capitalizing word: '{}'", w);
                // Always capitalize the first and last word
                let mut chars = w.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            } else {
                println!("Lowercasing small word: '{}'", w);
                w.to_lowercase()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_case() {
        let test_cases = vec![
            ("the legend of zelda", "The Legend of Zelda"),
            ("super mario bros.", "Super Mario Bros."),
            ("a tale of two cities", "A Tale of Two Cities"),
            ("the lord of the rings", "The Lord of the Rings"),
            ("to kill a mockingbird", "To Kill a Mockingbird"),
            (
                "harry potter and the sorcerer's stone",
                "Harry Potter and the Sorcerer's Stone",
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(title_case(input), expected);
        }
    }

    #[test]
    fn test_title_case_with_subtitles() {
        let test_cases = vec![
            // Subtitle with dash
            (
                "alcazar - the forgotten fortress",
                "Alcazar - The Forgotten Fortress",
            ),
            // Subtitle with colon
            (
                "alcazar: the forgotten fortress",
                "Alcazar: The Forgotten Fortress",
            ),
            // Multiple subtitles
            (
                "the world - a guide: the forgotten lands",
                "The World - A Guide: The Forgotten Lands",
            ),
            // Articles capitalized at start of subtitle
            ("game - a story of adventure", "Game - A Story of Adventure"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(title_case(input), expected);
        }
    }
}
