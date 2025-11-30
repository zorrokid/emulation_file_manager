use std::sync::Arc;

use core_types::{ArgumentType, DocumentType};
use sqlx::{Pool, Row, Sqlite};

use crate::{
    database_error::{DatabaseError, Error},
    models::DocumentViewer,
};

#[derive(Debug)]
pub struct DocumentViewerRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl DocumentViewerRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn get_document_viewers(&self) -> Result<Vec<DocumentViewer>, DatabaseError> {
        let document_viewers = sqlx::query(
            "SELECT id, name, executable, document_type, arguments, cleanup_temp_files 
             FROM document_viewer",
        )
        .fetch_all(&*self.pool)
        .await?;

        let document_viewers = document_viewers
            .into_iter()
            .map(|row| {
                let document_type: i64 = row.get("document_type");
                let document_type = DocumentType::try_from(document_type).map_err(|_e| {
                    DatabaseError::DbError(format!(
                        "Couldn't convert {} to DocumentType",
                        document_type
                    ))
                })?;

                Ok(DocumentViewer {
                    id: row.get("id"),
                    name: row.get("name"),
                    executable: row.get("executable"),
                    document_type,
                    arguments: row.get("arguments"),
                    cleanup_temp_files: row.get("cleanup_temp_files"),
                })
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?;

        Ok(document_viewers)
    }

    pub async fn add_document_viewer(
        &self,
        name: &String,
        executable: &String,
        arguments: &[ArgumentType],
        document_type: &DocumentType,
        cleanup_temp_files: bool,
    ) -> Result<i64, DatabaseError> {
        let document_type: i64 = (*document_type).into();
        // TODO: deserialize when fetching
        let serialized_arguments = serde_json::to_string(&arguments)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let result = sqlx::query!(
            "INSERT INTO document_viewer (
                name, 
                executable, 
                arguments,
                document_type,
                cleanup_temp_files
            ) VALUES (?, ?, ?, ?, ?)",
            name,
            executable,
            serialized_arguments,
            document_type,
            cleanup_temp_files,
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn update_document_viewer(
        &self,
        id: i64,
        name: &String,
        executable: &String,
        arguments: &Vec<ArgumentType>,
        document_type: &DocumentType,
        cleanup_temp_files: bool,
    ) -> Result<i64, DatabaseError> {
        let document_type: i64 = (*document_type).into();
        let arguments = serde_json::to_string(&arguments)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let result = sqlx::query!(
            "UPDATE document_viewer SET 
             name = ?, 
             executable = ?, 
             arguments = ?,
             document_type = ?,
             cleanup_temp_files = ?
             WHERE id = ?",
            name,
            executable,
            arguments,
            document_type,
            cleanup_temp_files,
            id
        )
        .execute(&*self.pool)
        .await?;
        Ok(id)
    }

    pub async fn delete(&self, id: i64) -> Result<i64, DatabaseError> {
        sqlx::query!("DELETE FROM document_viewer WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(id)
    }
}
