mod case;
mod normalizer;
mod rules;
mod search_keys;

pub use normalizer::{NormalizedTitle, TitleNormalizer};

/// Get the canonical software title from a release name,
/// e.g. "Super Mario Bros. (USA)" -> "Super Mario Bros."
/// or "castelo (Brazil) (En) (Unl)" -> "Castelo"
pub fn get_canonical_software_title(release_name: &str) -> String {
    let normalizer = TitleNormalizer;
    let normalized = normalizer.normalize(release_name);
    normalized.canonical
}
