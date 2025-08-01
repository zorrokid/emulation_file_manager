# Emulation File Manager and Launcher

## Technologies

- [Rust](https://www.rust-lang.org/): The primary programming language used for development.
- [relm4](https://relm4.org/): Used for building GTK4 GUI 
- [SQLx](https://github.com/launchbadge/sqlx): Used for SQLite database management, providing an asynchronous interface to SQLite database. 

## Crates

### core crates
#### core_types

Common types used across the project.

#### utils

Common utility functions used across the project.

#### emulator_runner

A crate for running emulators with provided arguments.

#### file_import

A crate for importing emulation related files into configured directories. User can import different types of files which are defined in `FileType` enum in `core_types` crate. Imported file is defined with `ImportedFile` struct in `core_types` crate. 

#### file_export 

A crate for exporting emulation related files from configured directories. When emulation files are used with emulators, they are exported to a temporary directory and then deleted after the emulator exits.

#### file_system

A crate for file system related operationgs, for example resolving paths for databse and emulation files.

### database

A crate for database related operations, including migrations for creating and managing the SQLite database used by the application and `models` module for creating objects from database entities.

### service

A crate for providing services to the, such as view model service defined in `view_model_service.rs` and the view model definitions. 

### cli

Will be removed.

### ui

GUI created with Iced - obsolete and will be removed.

### relm4-ui

New GUI created with relm4. This is the main crate for the application, providing the user interface and integrating with other crates.

## Architecture

The application is split in four main layers consisting of:
- the core creates - these shouldn't have dependencies on other crates, they are used by the other layers
- the database crate - this may have dependencies on the core crates, but not on the GUI crate
- the service crate - this may have dependencies on the core crates and the database crate, but not on the GUI crate
- the GUI crate - this may have dependencies to the layers below, the core crates and the database crate (but not the cli crate which will be removed)


