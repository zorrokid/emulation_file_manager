use crate::title_normalizer::{
    case::title_case,
    rules::{
        articles::normalize_articles, parentheticals::remove_parentheticals,
        whitespace::normalize_whitespace,
    },
    search_keys::{generate_search_keys, normalize_for_search},
};

pub struct SoftwareTitle {
    pub release_name: String,
    pub software_title_name: String,
}

pub fn get_software_title(release_name: &str) -> SoftwareTitle {
    let normalizer = TitleNormalizer;
    let normalized = normalizer.normalize(release_name);
    SoftwareTitle {
        release_name: normalized.original,
        software_title_name: normalized.canonical,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NormalizedTitle {
    pub original: String,
    pub canonical: String,
    pub search_keys: Vec<String>,
}

pub struct TitleNormalizer;

impl TitleNormalizer {
    pub fn normalize(&self, input: &str) -> NormalizedTitle {
        let mut s = input.to_string();

        s = remove_parentheticals(&s);
        s = normalize_articles(&s);
        s = normalize_whitespace(&s);
        s = title_case(&s);

        let normalized = normalize_for_search(&s);
        let search_keys = generate_search_keys(&normalized);

        NormalizedTitle {
            original: input.to_string(),
            canonical: s,
            search_keys,
        }
    }
}
/*
A.E. (USA) (Proto)
Activision Decathlon, The (USA)
Adam's Musicbox Demo (USA) (Demo)
Alcazar - The Forgotten Fortress (USA)
Antarctic Adventure (USA, Europe)
BC's Quest for Tires (USA)
Bump 'n' Jump (USA, Europe) (Beta)
Castelo (Brazil) (En) (Unl)
CAT S.O.S. Game, The (USA) (Promo)
Choplifter! (USA)
ColecoVision Monitor Test (USA, Europe)
Donkey Kong (USA, Europe) (v1.1)
Dr. Seuss - Fix-Up the Mix-Up Puzzler (USA)
Energy Quiz (Canada) (En,Fr-CA) (1983-06-06) (Proto)
Frogger II - ThreeeDeep! (USA) (Beta) (1984-06-15)
Front Line (USA, Europe) (Super Action Controller)
*/
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_normalize_titles() {
        let test_cases = vec![
            (
                "A.E. (USA) (Proto)",
                NormalizedTitle {
                    original: "A.E. (USA) (Proto)".to_string(),
                    canonical: "A.E.".to_string(),
                    search_keys: vec!["ae".to_string()],
                },
            ),
            (
                "Activision Decathlon, The (USA)",
                NormalizedTitle {
                    original: "Activision Decathlon, The (USA)".to_string(),
                    canonical: "The Activision Decathlon".to_string(),
                    search_keys: vec![
                        "the activision decathlon".to_string(),
                        "theactivisiondecathlon".to_string(),
                    ],
                },
            ),
            (
                "Adam's Musicbox Demo (USA) (Demo)",
                NormalizedTitle {
                    original: "Adam's Musicbox Demo (USA) (Demo)".to_string(),
                    canonical: "Adam's Musicbox Demo".to_string(),
                    search_keys: vec![
                        "adams musicbox demo".to_string(),
                        "adamsmusicboxdemo".to_string(),
                    ],
                },
            ),
            (
                "Alcazar - The Forgotten Fortress (USA)",
                NormalizedTitle {
                    original: "Alcazar - The Forgotten Fortress (USA)".to_string(),
                    canonical: "Alcazar - The Forgotten Fortress".to_string(),
                    search_keys: vec![
                        "alcazar the forgotten fortress".to_string(),
                        "alcazartheforgottenfortress".to_string(),
                    ],
                },
            ),
            (
                "Antarctic Adventure (USA, Europe)",
                NormalizedTitle {
                    original: "Antarctic Adventure (USA, Europe)".to_string(),
                    canonical: "Antarctic Adventure".to_string(),
                    search_keys: vec![
                        "antarctic adventure".to_string(),
                        "antarcticadventure".to_string(),
                    ],
                },
            ),
            (
                "BC's Quest for Tires (USA)",
                NormalizedTitle {
                    original: "BC's Quest for Tires (USA)".to_string(),
                    canonical: "BC's Quest For Tires".to_string(),
                    search_keys: vec![
                        "bcs quest for tires".to_string(),
                        "bcsquestfortires".to_string(),
                    ],
                },
            ),
            (
                "Bump 'n' Jump (USA, Europe) (Beta)",
                NormalizedTitle {
                    original: "Bump 'n' Jump (USA, Europe) (Beta)".to_string(),
                    canonical: "Bump 'n' Jump".to_string(),
                    search_keys: vec!["bump n jump".to_string(), "bumpnjump".to_string()],
                },
            ),
            (
                "Castelo (Brazil) (En) (Unl)",
                NormalizedTitle {
                    original: "Castelo (Brazil) (En) (Unl)".to_string(),
                    canonical: "Castelo".to_string(),
                    search_keys: vec!["castelo".to_string()],
                },
            ),
            (
                "CAT S.O.S. Game, The (USA) (Promo)",
                NormalizedTitle {
                    original: "CAT S.O.S. Game, The (USA) (Promo)".to_string(),
                    canonical: "The CAT S.O.S. Game".to_string(),
                    search_keys: vec!["the cat sos game".to_string(), "thecatsosgame".to_string()],
                },
            ),
            (
                "Choplifter! (USA)",
                NormalizedTitle {
                    original: "Choplifter! (USA)".to_string(),
                    canonical: "Choplifter!".to_string(),
                    search_keys: vec!["choplifter".to_string()],
                },
            ),
            (
                "ColecoVision Monitor Test (USA, Europe)",
                NormalizedTitle {
                    original: "ColecoVision Monitor Test (USA, Europe)".to_string(),
                    canonical: "ColecoVision Monitor Test".to_string(),
                    search_keys: vec![
                        "colecovision monitor test".to_string(),
                        "colecovisionmonitortest".to_string(),
                    ],
                },
            ),
            (
                "Donkey Kong (USA, Europe) (v1.1)",
                NormalizedTitle {
                    original: "Donkey Kong (USA, Europe) (v1.1)".to_string(),
                    canonical: "Donkey Kong".to_string(),
                    search_keys: vec!["donkey kong".to_string(), "donkeykong".to_string()],
                },
            ),
            (
                "Dr. Seuss - Fix-Up the Mix-Up Puzzler (USA)",
                NormalizedTitle {
                    original: "Dr. Seuss - Fix-Up the Mix-Up Puzzler (USA)".to_string(),
                    canonical: "Dr. Seuss - Fix-Up The Mix-Up Puzzler".to_string(),
                    search_keys: vec![
                        "dr seuss fixup the mixup puzzler".to_string(),
                        "drseussfixupthemixuppuzzler".to_string(),
                    ],
                },
            ),
            (
                "Energy Quiz (Canada) (En,Fr-CA) (1983-06-06) (Proto)",
                NormalizedTitle {
                    original: "Energy Quiz (Canada) (En,Fr-CA) (1983-06-06) (Proto)".to_string(),
                    canonical: "Energy Quiz".to_string(),
                    search_keys: vec!["energy quiz".to_string(), "energyquiz".to_string()],
                },
            ),
            (
                "Frogger II - ThreeeDeep! (USA) (Beta) (1984-06-15)",
                NormalizedTitle {
                    original: "Frogger II - ThreeeDeep! (USA) (Beta) (1984-06-15)".to_string(),
                    canonical: "Frogger II - ThreeeDeep!".to_string(),
                    search_keys: vec![
                        "frogger ii threeedeep".to_string(),
                        "froggeriithreeedeep".to_string(),
                    ],
                },
            ),
            (
                "Front Line (USA, Europe) (Super Action Controller)",
                NormalizedTitle {
                    original: "Front Line (USA, Europe) (Super Action Controller)".to_string(),
                    canonical: "Front Line".to_string(),
                    search_keys: vec!["front line".to_string(), "frontline".to_string()],
                },
            ),
        ];

        let normalizer = TitleNormalizer;

        for (input, expected) in test_cases {
            let normalized = normalizer.normalize(input);
            assert_eq!(normalized, expected);
        }
    }
}
