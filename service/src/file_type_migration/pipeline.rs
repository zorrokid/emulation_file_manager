use crate::{
    file_type_migration::{
        context::FileTypeMigrationContext,
        steps::{
            AddItemsToFileSetsStep, CollectCloudFileSetsStep, CollectFileSetsStep,
            MoveCloudFilesStep, MoveLocalFilesStep, UpdateFileInfosStep, UpdateFileSetsStep,
        },
    },
    pipeline::{cloud_connection::ConnectToCloudStep, generic_pipeline::Pipeline},
};

impl Pipeline<FileTypeMigrationContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CollectFileSetsStep),
            Box::new(CollectCloudFileSetsStep),
            Box::new(MoveLocalFilesStep),
            Box::new(ConnectToCloudStep::<FileTypeMigrationContext>::new()),
            Box::new(MoveCloudFilesStep),
            Box::new(UpdateFileInfosStep),
            Box::new(UpdateFileSetsStep),
            Box::new(AddItemsToFileSetsStep),
        ])
    }
}
