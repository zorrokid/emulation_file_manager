use std::sync::OnceLock;

use regex::Regex;

#[derive(Debug, Clone)]
pub struct DatFile {
    pub header: DatHeader,
    pub games: Vec<DatGame>,
}

#[derive(Debug, Clone)]
pub struct DatHeader {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub version: String,
    pub date: Option<String>,
    pub author: String,
    pub homepage: Option<String>,
    pub url: Option<String>,
    pub subset: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DatGame {
    pub name: String,
    pub id: Option<String>,
    pub cloneof: Option<String>,
    pub cloneofid: Option<String>,
    pub categories: Vec<String>,
    pub description: String,
    pub roms: Vec<DatRom>,
    pub releases: Vec<DatRelease>,
}

#[derive(Debug, Clone)]
pub struct DatRom {
    pub name: String,
    pub size: u64,
    pub crc: String,
    pub md5: String,
    pub sha1: String,
    pub sha256: Option<String>,
    pub status: Option<String>,
    pub serial: Option<String>,
    pub header: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DatRelease {
    pub name: String,
    pub region: String,
}

impl DatGame {
    /// Get the file set display name.
    pub fn get_file_set_name(&self) -> String {
        self.name.clone()
    }

    /// Get the file name for the file set. Extension is not included. It will be decided by the
    /// container format (e.g. zip).
    pub fn get_file_set_file_name(&self) -> String {
        self.name.clone()
    }

    /// Formalize a software title name from the game name. This may involve removing
    /// parentheticals, normalizing punctuation, and applying title case.
    pub fn get_software_title_name(&self) -> String {
        // Remove all parenthetical content
        let title = self.name.split('(').next().unwrap_or(&self.name).trim();

        // Handle "Title, The" -> "The Title" pattern
        // Also handle "Title, A" and "Title, An"
        if let Some(comma_pos) = title.rfind(", ") {
            let base = &title[..comma_pos];
            let article = &title[comma_pos + 2..];
            if article == "The" || article == "A" || article == "An" {
                format!("{} {}", article, base)
            } else {
                title.to_string()
            }
        } else {
            title.to_string()
        }
    }

    /// Get a list of release name for this game. Release names may include region or other
    /// qualifiers.
    pub fn get_release_name(&self) -> String {
        let tags = Self::extract_release_tags(&self.name);
        tags.join(" - ")
    }

    fn extract_release_tags(input: &str) -> Vec<String> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"\(([^)]+)\)").unwrap());

        re.captures_iter(input)
            .map(|cap| cap[1].trim().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_dat_game_with_name(name: &str) -> DatGame {
        DatGame {
            name: name.to_string(),
            id: None,
            cloneof: None,
            cloneofid: None,
            categories: vec![],
            description: "".to_string(),
            roms: vec![],
            releases: vec![],
        }
    }

    #[test]
    fn test_get_file_set_name() {
        let game = create_dat_game_with_name("Example Game (Europe)");
        assert_eq!(game.get_file_set_name(), "Example Game (Europe)");
    }

    #[test]
    fn test_get_file_set_file_name() {
        let game = create_dat_game_with_name("Example Game (Europe)");
        assert_eq!(game.get_file_set_file_name(), "Example Game (Europe)");
    }

    #[test]
    fn test_get_software_title_name() {
        let test_cases = &[
            ("A.E. (USA) (Proto)", "A.E."),
            (
                "Activision Decathlon, The (USA)",
                "The Activision Decathlon",
            ),
            ("Game, The (Europe)", "The Game"),
            ("Simple Game, A (Europe)", "A Simple Game"),
            ("Another Game, An (Europe)", "An Another Game"),
            (
                "Antarctic Adventure (USA, Europe) (Beta)",
                "Antarctic Adventure",
            ),
            (
                "BC's Quest for Tires II - Grog's Revenge (Canada)",
                "BC's Quest for Tires II - Grog's Revenge",
            ),
            ("Donkey Kong (USA, Europe) (v1.1)", "Donkey Kong"),
        ];
        for (input, expected) in test_cases {
            let game = create_dat_game_with_name(input);
            assert_eq!(game.get_software_title_name(), *expected);
        }
    }
    #[test]
    fn test_get_release_name() {
        let test_cases = &[
            ("A.E. (USA) (Proto)", "USA - Proto"),
            ("Activision Decathlon, The (USA)", "USA"),
            (
                "Antarctic Adventure (USA, Europe) (Beta)",
                "USA, Europe - Beta",
            ),
            (
                "BC's Quest for Tires II - Grog's Revenge (Canada)",
                "Canada",
            ),
            ("Donkey Kong (USA, Europe) (v1.1)", "USA, Europe - v1.1"),
        ];
        for (input, expected) in test_cases {
            let game = create_dat_game_with_name(input);
            assert_eq!(game.get_release_name(), *expected);
        }
    }
}
