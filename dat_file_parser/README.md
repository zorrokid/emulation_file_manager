# DAT file parser

This project provides a parser for DAT files commonly used in ROM management applications. It allows users to read DAT files and extract information about ROM sets and individual ROM entries to the application's data structures.

The parser reads the given DAT file path and converts its contents into a collection of `DatGame` objects. `DatGame` maps to a `game` field in No-Intro DAT file and contains metadata about the file set (name, description, id) and a collection of `DatRom` structs representing individual files (`rom` fields in No-Intro DAT file) including size and different types of checksums and the file name.

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

# Schema

See example-data folder for schema and example DAT file(s).
