#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputProfile {
    Standard,
    Intellivision, // Right stick maps to keypad directions, not actually needed at the moment
                   // since core does the keypad mapping internally.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportedCoreDefinition {
    pub core_name: &'static str,
    pub input_profile: InputProfile,
}

pub const SUPPORTED_CORES: &[SupportedCoreDefinition] = &[
    SupportedCoreDefinition {
        core_name: "fceumm_libretro",
        input_profile: InputProfile::Standard,
    },
    SupportedCoreDefinition {
        core_name: "freeintv_libretro",
        input_profile: InputProfile::Standard,
    },
];

pub fn get_supported_core(core_name: &str) -> Option<&'static SupportedCoreDefinition> {
    SUPPORTED_CORES
        .iter()
        .find(|def| def.core_name == core_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_supported_core() {
        let core = get_supported_core("fceumm_libretro");
        assert!(core.is_some());
        assert_eq!(core.unwrap().input_profile, InputProfile::Standard);
        let core = get_supported_core("freeintv_libretro");
        assert!(core.is_some());
        assert_eq!(core.unwrap().input_profile, InputProfile::Standard);
        let core = get_supported_core("nonexistent_core");
        assert!(core.is_none());
    }

    #[test]
    fn test_supported_cores_list() {
        assert_eq!(SUPPORTED_CORES.len(), 2);
        assert_eq!(SUPPORTED_CORES[0].core_name, "fceumm_libretro");
        assert_eq!(SUPPORTED_CORES[1].core_name, "freeintv_libretro");
    }
}
