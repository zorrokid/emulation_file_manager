use async_trait::async_trait;
use serde::Deserialize;
use std::fmt::Display;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[async_trait]
pub trait DatFileParserOps: Send + Sync {
    fn parse_dat_file(&self, path: &Path) -> Result<DatFile, DatFileParserError>;
}

pub struct DefaultDatParser;

#[async_trait]
impl DatFileParserOps for DefaultDatParser {
    fn parse_dat_file(&self, path: &Path) -> Result<DatFile, DatFileParserError> {
        parse_dat_file(path).map_err(|err| {
            DatFileParserError::IoError(format!("Error while parsing path {:?}: {:?}", path, err))
        })
    }
}

pub struct MockDatParser {
    parse_result: Result<DatFile, DatFileParserError>,
}

impl MockDatParser {
    pub fn new(parse_result: Result<DatFile, DatFileParserError>) -> Self {
        Self { parse_result }
    }
    pub fn set_parse_result(&mut self, parse_result: Result<DatFile, DatFileParserError>) {
        self.parse_result = parse_result;
    }
}

impl DatFileParserOps for MockDatParser {
    fn parse_dat_file(&self, path: &Path) -> Result<DatFile, DatFileParserError> {
        self.parse_result.clone()
    }
}

#[derive(Debug, Clone)]
pub enum DatFileParserError {
    IoError(String),
    ParseError(String),
}

impl Display for DatFileParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatFileParserError::IoError(message) => write!(f, "IO error: {}", message),
            DatFileParserError::ParseError(message) => write!(f, "Parse error: {}", message),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename = "datafile")]
pub struct DatFile {
    pub header: DatHeader,
    #[serde(rename = "game", default)]
    pub games: Vec<DatGame>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Default)]
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

#[derive(Debug, Deserialize, PartialEq, Clone)]
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

#[derive(Debug, Deserialize, PartialEq, Clone)]
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

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct DatRelease {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@region")]
    pub region: String,
}

pub fn parse_dat_file(path: &Path) -> Result<DatFile, DatFileParserError> {
    let file = File::open(path).map_err(|e| {
        DatFileParserError::IoError(format!("Failed opening path {:?}: {}", path, e))
    })?;
    let reader = BufReader::new(file);
    let dat_file: DatFile = quick_xml::de::from_reader(reader).map_err(|e| {
        DatFileParserError::ParseError(format!("Failed parsing file {:?}: {}", path, e))
    })?;
    Ok(dat_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example_dat() {
        let path = Path::new("example-data/coleco.dat");
        let result = parse_dat_file(path);
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
