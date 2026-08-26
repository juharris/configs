use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use thiserror::Error;

const EMBEDDED_SCHEMA: &[u8] = include_bytes!("../../configs/.optify/schema.json");

/// Owns the packaged schema file for as long as an Optify watcher can reread it.
pub struct RuntimeSchema {
    _directory: TempDir,
    path: PathBuf,
}

impl RuntimeSchema {
    pub fn materialize() -> Result<Self, RuntimeSchemaError> {
        let directory = tempfile::Builder::new()
            .prefix("personal-dashboard-schema-")
            .tempdir()
            .map_err(RuntimeSchemaError::CreateDirectory)?;
        let path = directory.path().join("schema.json");
        fs::write(&path, EMBEDDED_SCHEMA).map_err(RuntimeSchemaError::WriteSchema)?;
        Ok(Self {
            _directory: directory,
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Error)]
pub enum RuntimeSchemaError {
    #[error("could not create the runtime schema directory: {0}")]
    CreateDirectory(std::io::Error),
    #[error("could not write the packaged runtime schema: {0}")]
    WriteSchema(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::{EMBEDDED_SCHEMA, RuntimeSchema};

    #[test]
    fn materializes_the_embedded_schema_for_the_process_lifetime() {
        let runtime_schema = RuntimeSchema::materialize().unwrap();

        assert_eq!(
            std::fs::read(runtime_schema.path()).unwrap(),
            EMBEDDED_SCHEMA
        );
        assert!(runtime_schema.path().is_file());
    }
}
