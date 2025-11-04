use crate::{export_files, export_files_zipped, FileExportError, FileSetExportModel};

pub trait FileExportOps {
    fn export(export_model: &FileSetExportModel) -> Result<(), FileExportError>;
    fn export_zipped(export_model: &FileSetExportModel) -> Result<(), FileExportError>;
}

pub struct DefaultFileExportOps;

impl FileExportOps for DefaultFileExportOps {
    fn export(export_model: &FileSetExportModel) -> Result<(), FileExportError> {
        export_files(export_model)
    }

    fn export_zipped(export_model: &FileSetExportModel) -> Result<(), FileExportError> {
        export_files_zipped(export_model)
    }
}

pub struct MockFileExportOps {
    // TODO: Add fields if needed for testing e.g. simulate errors
}

/// Mock implementation for testing
impl FileExportOps for MockFileExportOps {
    fn export(_export_model: &FileSetExportModel) -> Result<(), FileExportError> {
        Ok(())
    }

    fn export_zipped(_export_model: &FileSetExportModel) -> Result<(), FileExportError> {
        Ok(())
    }
}
