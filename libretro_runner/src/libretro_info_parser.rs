use std::{collections::HashMap, path::Path};

use crate::{
    error::LibretroError,
    model::{LibretroFirmwareInfo, LibretroSystemInfo},
};

pub fn get_libretro_info_file_name(core_name: &str) -> String {
    format!("{}.info", core_name)
}

fn get_info_content_map(info_content: &str) -> HashMap<String, String> {
    info_content
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, '=');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                Some((
                    key.trim().to_string(),
                    value.trim().trim_matches('"').to_string(),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn get_firmware_count(info_content_map: &HashMap<String, String>) -> Result<usize, LibretroError> {
    info_content_map
        .get("firmware_count")
        .map_or(Ok(0), |info_count_str| {
            info_count_str.parse::<usize>().map_err(|e| {
                LibretroError::LibretroInfoParserError(format!(
                    "Failed to parse firmware_count: {}",
                    e
                ))
            })
        })
}

fn get_firmware_info(
    info_content_map: &HashMap<String, String>,
) -> Result<Vec<LibretroFirmwareInfo>, LibretroError> {
    let mut firmware_info = Vec::new();
    let firmware_count = get_firmware_count(info_content_map)?;
    for i in 0..firmware_count {
        let desc_key = format!("firmware{}_desc", i);
        let path_key = format!("firmware{}_path", i);
        let opt_key = format!("firmware{}_opt", i);

        if let (Some(desc), Some(path), Some(opt)) = (
            info_content_map.get(&desc_key),
            info_content_map.get(&path_key),
            info_content_map.get(&opt_key),
        ) {
            firmware_info.push(LibretroFirmwareInfo {
                desc: desc.clone(),
                path: path.clone(),
                opt: opt == "true",
            });
        } else {
            return Err(LibretroError::LibretroInfoParserError(format!(
                "Missing firmware information for index {}",
                i
            )));
        }
    }
    Ok(firmware_info)
}

pub async fn parse_libretro_info(
    core_name: &str,
    libretro_core_path: &Path,
) -> Result<LibretroSystemInfo, LibretroError> {
    let info_path = libretro_core_path.join(get_libretro_info_file_name(core_name));
    println!("Parsing libretro info from path: {:?}", info_path);
    let info_content = tokio::fs::read_to_string(info_path)
        .await
        .map_err(|e| LibretroError::LibretroInfoParserError(e.to_string()))?;

    let info_content_map = get_info_content_map(&info_content);

    let firmare_info: Vec<LibretroFirmwareInfo> = get_firmware_info(&info_content_map)?;

    let system_info = LibretroSystemInfo {
        display_name: info_content_map
            .get("display_name")
            .cloned()
            .unwrap_or_default(),
        authors: info_content_map.get("authors").cloned().unwrap_or_default(),
        supported_extensions: info_content_map
            .get("supported_extensions")
            .map(|exts| exts.split('|').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        core_name: info_content_map
            .get("corename")
            .cloned()
            .unwrap_or_default(),
        categories: info_content_map
            .get("categories")
            .map(|cats| cats.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        license: info_content_map.get("license").cloned().unwrap_or_default(),
        permissions: info_content_map
            .get("permissions")
            .cloned()
            .unwrap_or_default(),
        display_version: info_content_map
            .get("display_version")
            .cloned()
            .unwrap_or_default(),
        manufacturer: info_content_map
            .get("manufacturer")
            .cloned()
            .unwrap_or_default(),
        system_name: info_content_map
            .get("systemname")
            .cloned()
            .unwrap_or_default(),
        system_id: info_content_map
            .get("systemid")
            .cloned()
            .unwrap_or_default(),
        database: info_content_map
            .get("database")
            .cloned()
            .unwrap_or_default(),
        supports_no_game: info_content_map
            .get("supports_no_game")
            .map(|val| val == "true")
            .unwrap_or(false),
        firmware: firmare_info,
        description: info_content_map
            .get("description")
            .cloned()
            .unwrap_or_default(),
    };

    Ok(system_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_libretro_info() {
        // TODO: maybe use this example file instead: https://github.com/libretro/libretro-core-info/blob/master/00_example_libretro.info
        let core_name = "freeintv_libretro";
        let libretro_core_path = Path::new("example-data");
        let result = parse_libretro_info(core_name, libretro_core_path).await;
        assert!(result.is_ok());
        let system_info = result.unwrap();
        assert_eq!(
            system_info.display_name,
            "Mattel - Intellivision (FreeIntv)"
        );
        assert_eq!(system_info.authors, "David Richardson");
        assert_eq!(system_info.supported_extensions, vec!["int", "bin", "rom"]);
        assert_eq!(system_info.core_name, "FreeIntv");
        assert_eq!(system_info.categories, vec!["Emulator"]);
        assert_eq!(system_info.license, "GPLv3");
        assert_eq!(system_info.permissions, "");
        assert_eq!(system_info.display_version, "2018.1.5");
        assert_eq!(system_info.manufacturer, "Mattel");
        assert_eq!(system_info.system_name, "Intellivision");
        assert_eq!(system_info.system_id, "intellivision");
        assert_eq!(system_info.database, "Mattel - Intellivision");
        assert!(!system_info.supports_no_game);
        let firmware_info = system_info.firmware;
        assert_eq!(firmware_info.len(), 2);
        assert_eq!(firmware_info[0].desc, "exec.bin");
        assert_eq!(firmware_info[0].path, "exec.bin");
        assert!(!firmware_info[0].opt);
        assert_eq!(firmware_info[1].desc, "grom.bin");
        assert_eq!(firmware_info[1].path, "grom.bin");
        assert!(!firmware_info[1].opt);
        assert_eq!(
            system_info.description,
            "A libretro emulation core for the Mattel Intellivision computer (but not the Entertainment Computer System or Intellivoice). Many Intellivision games relied on controller overlays to provide context for the controls, and many of these can be found online for reference, including at https://arcadepunks.com/intellivision-controller-overlays."
        );
    }
}
