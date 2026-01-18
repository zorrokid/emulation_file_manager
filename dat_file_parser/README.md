# DAT file parser

This project provides a parser for DAT files commonly used in ROM management applications. It allows users to read DAT files and extract information about ROM sets and individual ROM entries to the application's data structures.

The parser reads the given DAT file path and converts its contents into a collection of `DatFileSetEntry` objects. `DatFileSetEntry` maps to a `game` field in No-Intro DAT file and contains metadata about the file set (name, description, id) and a collection of `DatFileEntry` structs representing individual files (`rom` fields in No-Intro DAT file) including size and different types of checksums and the file name.

Currently supported DAT format is the No-Intro XML format: https://datomatic.no-intro.org/stuff/schema_nointro_datfile_v3.xsd

Example header:
```xml
	<header>
		<id>3</id>
		<name>Coleco - ColecoVision</name>
		<description>Coleco - ColecoVision</description>
		<version>20250321-153911</version>
		<author>Arctic Circle System, Aringon, C. V. Reynolds, Gefflon, Hiccup, kazumi213, omonim2007, Psychofox11, psykopat, relax, SonGoku, xuom2</author>
		<homepage>No-Intro</homepage>
		<url>https://www.no-intro.org</url>
		<clrmamepro forcenodump="required"/>
	</header>
```

A game entry example:
```xml
	<game name="2010 - The Graphic Action Game (USA)" id="0001">
		<description>2010 - The Graphic Action Game (USA)</description>
		<rom name="2010 - The Graphic Action Game (USA).col" size="32768" crc="c575a831" md5="0dfb83c1353481d297dfcc6123978533" sha1="b1621d39a2a6d1cda7eceb882a612a4baf3da70c"/>
	</game>
```


# Data Structures

```rust
pub struct Dat {
    pub header: DatHeader,
    pub file_sets: Vec<DatFileSetEntry>,
}
```

```rust
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
```

```rust
pub struct DatGame {
    pub name: String,
    pub description: String,
    pub id: Option<String>,
    pub cloneof: Option<String>,
    pub cloneofid: Option<String>,
    pub categories: Vec<String>,
    pub roms: Vec<DatRom>,
    pub releases: Vec<DatRelease>,
}
```

```rust
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
```

```rust
pub struct DatRelease {
    pub name: String,
    pub region: String,
}
```

# Schema

```xml
<?xml version="1.0" encoding="utf-8"?>
<xs:schema attributeFormDefault="unqualified" elementFormDefault="qualified" xmlns:xs="http://www.w3.org/2001/XMLSchema">
	<xs:element name="datafile">
		<xs:complexType>
			<xs:sequence>
				<xs:element name="header">
					<xs:complexType>
						<xs:sequence>
							<xs:element name="id" type="xs:int" />
							<xs:element name="name" type="xs:string" />
							<xs:element name="description" type="xs:string" />
							<xs:element name="version" type="xs:string" />
							<xs:element name="date" type="xs:string" minOccurs="0" />
							<xs:element name="author" type="xs:string" />
							<xs:element name="homepage" type="xs:string" minOccurs="0" />
							<xs:element name="url" type="xs:string" minOccurs="0" />
							<xs:element name="subset" type="xs:string" minOccurs="0" />
							<xs:element name="clrmamepro" minOccurs="0" >
								<xs:complexType>
									<xs:attribute name="forcenodump" default="obsolete" use="optional">
										<xs:simpleType>
											<xs:restriction base="xs:token">
												<xs:enumeration value="obsolete"/>
												<xs:enumeration value="required"/>
												<xs:enumeration value="ignore"/>
											</xs:restriction>
										</xs:simpleType>
									</xs:attribute>
									<xs:attribute name="header" type="xs:string" use="optional" />
								</xs:complexType>
							</xs:element>
							<xs:element name="romcenter" minOccurs="0" >
								<xs:complexType>
									<xs:attribute name="plugin" type="xs:string" use="optional" />
								</xs:complexType>
							</xs:element>
						</xs:sequence>
					</xs:complexType>
				</xs:element>
				<xs:element name="game" minOccurs="0" maxOccurs="unbounded" >
					<xs:complexType>
						<xs:sequence>
							<xs:element name="category" type="xs:string" minOccurs="0" maxOccurs="unbounded" />
							<xs:element name="description" type="xs:string" />
							<xs:element name="rom">
								<xs:complexType>
									<xs:attribute name="name" type="xs:string" use="required" />
									<xs:attribute name="size" type="xs:unsignedInt" use="required" />
									<xs:attribute name="crc" type="xs:string" use="required" />
									<xs:attribute name="md5" type="xs:string" use="required" />
									<xs:attribute name="sha1" type="xs:string" use="required" />
									<xs:attribute name="sha256" type="xs:string" use="optional" />
									<xs:attribute name="status" type="xs:string" use="optional" />
									<xs:attribute name="serial" type="xs:string" use="optional" />
									<xs:attribute name="header" type="xs:string" use="optional" />
								</xs:complexType>
							</xs:element>
							<xs:element name="release" minOccurs="0" maxOccurs="unbounded" >
								<xs:complexType>
									<xs:attribute name="name" type="xs:string" use="required" />
									<xs:attribute name="region" type="xs:string" use="required" />
								</xs:complexType>
							</xs:element>
						</xs:sequence>
						<xs:attribute name="name" type="xs:string" use="required" />
						<xs:attribute name="id" type="xs:string" use="optional" />
						<xs:attribute name="cloneof" type="xs:string" use="optional" />
						<xs:attribute name="cloneofid" type="xs:string" use="optional" />
					</xs:complexType>
				</xs:element>
			</xs:sequence>
		</xs:complexType>
	</xs:element>
</xs:schema>
```
