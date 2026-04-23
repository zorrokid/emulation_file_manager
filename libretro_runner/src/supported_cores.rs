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

pub fn get_supported_core_definition(core_name: &str) -> Option<&'static SupportedCoreDefinition> {
    SUPPORTED_CORES
        .iter()
        .find(|def| def.core_name == core_name)
}
