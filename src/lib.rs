//! # Overview
//!
//! A session store implementation for `tower-sessions`, that uses the filesystem, to store a session in a `json` file.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;
use tower_sessions_core::{
    session::{Id, Record},
    session_store, SessionStore,
};

#[derive(Debug, thiserror::Error)]
enum FileStoreError {
    // we only return the encoding errors. To avoid mis-using this enum variant, `#[from]` is not used.
    #[error(transparent)]
    Encode(serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<FileStoreError> for session_store::Error {
    fn from(err: FileStoreError) -> Self {
        match err {
            FileStoreError::Encode(inner) => Self::Encode(inner.to_string()),
            FileStoreError::Io(inner) => Self::Backend(inner.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileStore {
    path: &'static Path,
}

impl FileStore {
    pub fn new(path: &'static Path) -> Self {
        Self { path }
    }

    fn record_path(&self, id: &Id) -> PathBuf {
        self.path.join(id.to_string()).with_extension("json")
    }

    async fn store(&self, record: &Record) -> Result<(), FileStoreError> {
        fs::create_dir_all(self.path).await?;

        let json_data = serde_json::to_string(record).map_err(FileStoreError::Encode)?;
        fs::write(self.record_path(&record.id), json_data).await?;

        Ok(())
    }
}

impl Default for FileStore {
    fn default() -> Self {
        Self {
            path: Path::new("sessions"),
        }
    }
}

#[async_trait]
impl SessionStore for FileStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        while fs::try_exists(self.record_path(&record.id))
            .await
            .map_err(FileStoreError::Io)?
        {
            record.id = Id::default();
        }

        self.store(record).await?;
        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        self.store(record).await?;
        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let path = self.record_path(session_id);

        let json = match fs::read(&path).await {
            Ok(json) => json,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(FileStoreError::from(error).into()),
        };

        // broken records are counted as the record does not exist
        let record = serde_json::from_slice::<Record>(&json).ok();

        Ok(record)
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let result = fs::remove_file(self.record_path(session_id)).await;
        match result {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(FileStoreError::from(error).into()),
        }

        Ok(())
    }
}
