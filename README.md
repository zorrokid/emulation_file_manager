# Why?

I had lots of emulation related files (disk images, manuals, cover art, etc) stored in different locations on my computer or different computers and drives. Usually downloading them over and over again. I wanted to have a single application to manage these files and launch them with emulators and document viewers. I also wanted to be able to sync these files to cloud storage so that I can access them from multiple devices.

# Emulation File Manager and Launcher

Emulation File Manager can be used to manage your emulation related files and launch them with emulators and document viewers. Files are stored under collection root folder and can be synced to S3 compatible cloud storage. File meta data is stored to local SQLite database. Files can be added from local file system or providing a download URL (for example to Internet Archive). 

Emulators and document viewers can be configured for laumching files with. Bitmap-file types can be viewed directly in application.


## Technologies

- [Rust](https://www.rust-lang.org/): The primary programming language used for development.
- [relm4](https://relm4.org/): GTK4 UI 
- [SQLx](https://github.com/launchbadge/sqlx): Used for SQLite database management, providing an asynchronous interface to SQLite database.
- [rust-s3](https://github.com/durch/rust-s3): S3 compatible cloud storage

## Architecture

The application is split in four main layers consisting of:
- the core creates - these shouldn't have dependencies on other crates, they are used by the other layers
- the database crate - this may have dependencies on the core crates, but not on the GUI crate
- the service crate - this may have dependencies on the core crates and the database crate, but not on the GUI crate
- the GUI crate - this may have dependencies to the layers below, the core crates and the database crate (but not the cli crate which will be removed)

## Emulation files

Emulation files can be different kind of files (see `core_types::FileType` enum). Some of the files are used with emulators, some with external viewers, some internal viewers in this application.

Files are currently stored in local file system, collection directory is determined in `file_system` crate with `ProjectDirs`. I'm planning to add support for cloud storage in the future.

When files are imported for software release, files are imported as file set whether it's a single file or multiple files. One file in file set can belong to multiple file sets. Each file set can have only files of certain `FileType`. Because of this, each file in file set is stored separately and different kind of file sets can be composed from files. 

When storing files, each file gets a unique file name to avoid conflicts in file names. Original file name is stored in database as part of oringal imported file set. Files are also compressed using zstd compression. 

File info and file set meta data is stored in database with this structure:

```plaintext

   +-----------+ 1   * +----------------------+
   | file_info |-------|  file_set_file_info  | 
   +-----------+       +----------------------+
         | 1                     | *
         |                       | 
         | *                     | 1
   +------------------+   +------------+  +----------+ 
   | file_info_system |   |  file_set  |  |  release |
   +------------------+   +------------+  +----------+
          | *                    | *         1| 
          |                      |            | 
          | 1                    | 1        * | 
    +-------------+         +-------------------+ 
    |   system    |         |  release_file_set |
    +-------------+         +-------------------+

```


- `file_info`: represents a single file, with attributes like original sha1 checksum, file size and unique archive file name
- `system`: represents a system, like Commodore 64, NES, etc. 
- `release`: represents a software release, like a game or application. Release can have multiple file sets, for example one file set for disk images, one for manual and one for cover art.
- `file_info_system`: file can be linked to multiple systems (for example same manual can be used for multiple systems)
- `file_set`: represents a set of files, for example a set of disk images for a software release. File set has a single FileType so file set can contain only files of same type. File set also has a name which is used when file set is exported from application.
- `file_set_file_info`: links file set to files. One file set can have multiple files and one file can belong to multiple file sets. This contains also a file set spefic file name for the file which is used when exporting file from application. 
- `release_file_set`: links release to file sets. This enables file sets to be reused for multiple releases. 

When file is used with emulator or external viewer, file is exported to a temporary directory. File is exported with file set specific file name. Emulator files are exported either as a complete file set as a zip archive or as individual files depending on the emulator configuration.


