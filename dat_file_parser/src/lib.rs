use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename = "datafile")]
pub struct DatFile {
    pub header: DatHeader,
    #[serde(rename = "game", default)]
    pub games: Vec<DatGame>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DatHeader {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub date: Option<String>,
    pub author: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub subset: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DatGame {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@id", default)]
    pub id: Option<String>,
    #[serde(rename = "@cloneof", default)]
    pub cloneof: Option<String>,
    #[serde(rename = "@cloneofid", default)]
    pub cloneofid: Option<String>,
    #[serde(rename = "category", default)]
    pub categories: Vec<String>,
    pub description: String,
    #[serde(rename = "rom")]
    pub roms: Vec<DatRom>,
    #[serde(rename = "release", default)]
    pub releases: Vec<DatRelease>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DatRom {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@size")]
    pub size: u64,
    #[serde(rename = "@crc")]
    pub crc: String,
    #[serde(rename = "@md5")]
    pub md5: String,
    #[serde(rename = "@sha1")]
    pub sha1: String,
    #[serde(rename = "@sha256", default)]
    pub sha256: Option<String>,
    #[serde(rename = "@status", default)]
    pub status: Option<String>,
    #[serde(rename = "@serial", default)]
    pub serial: Option<String>,
    #[serde(rename = "@header", default)]
    pub header: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DatRelease {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@region")]
    pub region: String,
}

pub fn parse_dat_file<P: AsRef<Path>>(path: P) -> Result<DatFile, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let dat_file: DatFile = quick_xml::de::from_reader(reader)?;
    Ok(dat_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example_dat() {
        let result = parse_dat_file("example-data/coleco.dat");
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let dat = result.unwrap();
        assert_eq!(dat.header.id, 3);
        assert_eq!(dat.header.name, "Coleco - ColecoVision");
        assert_eq!(dat.header.version, "20250321-153911");
        assert!(!dat.games.is_empty());

        let first_game = &dat.games[0];
        assert_eq!(first_game.name, "[BIOS] ColecoVision (USA, Europe)");
        assert_eq!(first_game.id, Some("0029".to_string()));
        assert_eq!(first_game.roms.len(), 1);

        let rom = &first_game.roms[0];
        assert_eq!(rom.size, 8192);
        assert_eq!(rom.crc, "3aa93ef3");
    }
}
