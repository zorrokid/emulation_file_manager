# Why?

I had lots of emulation related files (disk images, manuals, cover art, etc) stored in different locations on my computer or different computers and drives. Usually downloading them over and over again. I wanted to have a single application to manage these files and launch them with emulators and document viewers. I also wanted to be able to sync these files to cloud storage so that I can access them from multiple devices.

# Emulation File Manager and Launcher

Emulation File Manager can be used to manage your emulation related files and launch them with emulators and document viewers. Files are stored under collection root folder and can be synced to S3 compatible cloud storage. File meta data is stored to local SQLite database. Files can be added from local file system or providing a download URL (for example to Internet Archive). 

Emulators and document viewers can be configured for laumching files with. Bitmap-file types can be viewed directly in application.

# Concepts (Domain Language) 

## System

## Software Title

### Software Release

## File Set

### File

### File Type

## Emulator

## Document Viewer

# Settings

## Collection Root directory

This is automatically determined by `directories` crate with `ProjectDirs`. This is the root directory where all files are stored. This is also the directory that can be synced to cloud storage.

User can change this directory in settings. Currently when changing the collection root directory, user should manually copy the files from old collection root directory to new collection root directory and before deleting them from original location, user should make sure that all files are moved and available in new collection root directory. 

This can be done with rsync for example:

```bash
rsync -avz --checksum ~/.local/share/efm/files/ /path/to/new/collection/root/
```


In the future, I will add support for moving files to new collection root directory automatically when user changes the collection root directory in settings and / or support for multiple collection root directories. 

# Technologies

- [Rust](https://www.rust-lang.org/): The primary programming language used for development.
- [relm4](https://relm4.org/): GTK4 UI 
- [SQLx](https://github.com/launchbadge/sqlx): Used for SQLite database management, providing an asynchronous interface to SQLite database.
- [rust-s3](https://github.com/durch/rust-s3): S3 compatible cloud storage

# Architecture

The application is split in four main layers consisting of:
- the core creates - these shouldn't have dependencies on other crates, they are used by the other layers
- the database crate - this may have dependencies on the core crates, but not on the GUI crate
- the service crate - this may have dependencies on the core crates and the database crate, but not on the GUI crate
- the GUI crate - this may have dependencies to the layers below, the core crates and the database crate (but not the cli crate which will be removed)

# Emulation files

Emulation files can be different kind of files (see `core_types::FileType` enum). Some of the files are used with emulators, some with external viewers, some internal viewers in this application.

Files are stored in local file system, collection directory is determined in `file_system` crate with `ProjectDirs` (for Debian Linux this is under `.local/share`. Collection directory can be also changed in settings. Optionally files can be synced to cloud. 

For cloud sync you need to have a S3 compatible cloud storage, enable cloud sync from settings dialog and set the following environment variables: `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`. For example in Bash this can be set to `.bashrc` file like this:

```bash
export AWS_ACCESS_KEY_ID=your_access_key_id
export AWS_SECRET_ACCESS_KEY=your_secret_access_key
```

(There is also an option to store the credentials in keyring from settings dialog, but for me the credentials have been resetting for some reason and I haven't had time to investigate this issue yet, so I recommend setting the credentials with environment variables for now. Credentials from environment variables are anyway used as a fallback.)

When files are added for a software release, files are imported as file set in both cases whether it's a single file or multiple files. Each file set can have only files of certain `FileType`. 

One file in file set can belong to multiple file sets. Because of this, each file in file set is stored separately and different kind of file sets can be composed from files. 

When storing files, each file gets a unique file name to avoid conflicts in file names. Original file name is stored in database as part of oringal imported file set. Files are also compressed using zstd compression. 

File info and file set meta data is stored in database with this structure:

```plaintext

   +-----------+ 1   * +----------------------+
   | file_info |-------|  file_set_file_info  | 
   +-----------+       +----------------------+
         | 1                     | *
         |                       | 
         | *                     |  
   +------------------+          |                +----------+ 1
   | file_info_system |          |                |  release |----------+
   +------------------+          |                +----------+          |
          | *                    |                     1|               |
          |                      |                      |               |
          | 1                    | 1                  * |               |*
    +-------------+      +------------+ 1 * +-------------------+  +--------------+
    |   system    |      |  file_set  |-----| release_file_set  |  | release_item | <-- items can be with or without file sets
    +-------------+      +------------+     +-------------------+  +--------------+
                                | 1                                   1|
                                | *                                    |
                      +-----------------------+ *                      |
                      | release_item_file_set |------------------------+
                      +-----------------------+
                                        
                                       ^
                                       +----- release file sets can be with or without item
```


- `file_info`: represents a single file, with attributes like original sha1 checksum, file size, unique archive file name, and `file_type`
- `system`: represents a system, like Commodore 64, NES, etc. 
- `release`: represents a software release, like a game or application. Release can have multiple file sets, for example one file set for disk images, one for manual and one for cover art.
- `file_info_system`: file can be linked to multiple systems (for example same manual can be used for multiple systems)
- `file_set`: represents a set of files, for example a set of disk images for a software release. File set has a single `file_type` so file set can contain only files of same type. File set also has a name which is used when file set is exported from application.
- `file_set_file_info`: links file set to files. One file set can have multiple files and one file can belong to multiple file sets. This contains also a file set specific file name for the file which is used when exporting file from application. 
- `release_file_set`: links release to file sets. This is the primary relationship defining what file sets belong to a release. File sets can be reused across multiple releases (e.g., same manual in different versions).
- `release_item`: represents physical items that should be part of a release (e.g., "this release should have 2 Disks, 1 Manual, and 1 Box"). Each item has an `item_type` and optional `notes`. This enables tracking both what physical items exist and assessing completeness of digital files.
- `file_set_item`: categorizes and organizes file sets by linking them to release items. This is optional metadata that helps track which file sets represent which physical items (e.g., "these disk image file sets are for Disk 1" or "this PDF file set is the Manual"). A file set can exist in `release_file_set` without being linked to an item.

The two-level linking approach (`release_file_set` + `file_set_item`) enables:
- Tracking what file sets are in a release (primary relationship via `release_file_set`)
- Organizing those file sets by physical item type (metadata via `file_set_item`)
- Assessing completeness: "This release should have a Manual (release_item), but we haven't linked any file sets to it yet"
- Sharing file sets across releases while maintaining release-specific item categorization

## File Type Design Rationale

Both `file_info` and `file_set` contain a `file_type` field. While this appears redundant, both are necessary for different reasons:

**Why `file_info.file_type` is needed:**
1. **Cloud storage organization**: Used in `generate_cloud_key()` to organize files in cloud storage by type (e.g., `roms/game.zip`, `documents/manual.pdf`)
2. **Independent file operations**: Files can be queried and operated on independently of their file set context
3. **File picker filtering**: When creating a new file set by picking existing files from the database, files can be filtered by type to match the file set's type
4. **Physical file path resolution**: Used to determine the correct subdirectory when storing and retrieving files from the local file system

**Why `file_set.file_type` is needed:**
1. **Collection categorization**: Represents the semantic type of the entire collection (e.g., "this is a ROM set" vs "this is a document set")
2. **Filtering and querying**: Used to find all file sets of a specific type (e.g., show all ROM sets for a system)
3. **Export directory structure**: Determines output directory structure when exporting file sets

**Consistency guarantee**: The application validates that `file_info.file_type` matches `file_set.file_type` when linking them through `file_set_file_info`. This prevents data inconsistencies where a file's type doesn't match its containing file set. A `FileInfo` can only be added to a `FileSet` if their `file_type` values match.

When file is used with emulator or external viewer, file is exported to a temporary directory. File is exported with file set specific file name. Emulator files are exported either as a complete file set as a zip archive or as individual files depending on the emulator configuration.


