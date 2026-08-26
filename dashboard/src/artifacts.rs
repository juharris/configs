use std::fs;
use std::path::{Path, PathBuf};

use schemars::schema_for;
use serde_json::{Value, json};
use thiserror::Error;
use ts_rs::TS;

use crate::config::DashboardFeatureFile;
use crate::messages::{
    ActiveConfiguration, BootstrapResponse, ClientMessage, ClientRequest, ErrorCode, OptifySetup,
    ServerEvent, ServerMessage, ServerResponse, SetupStatus, Theme,
};

const OPTIFY_FEATURE_SCHEMA: &str =
    "https://raw.githubusercontent.com/juharris/optify/refs/heads/main/schemas/feature_file.json";

pub fn check_generated_artifacts() -> Result<(), ArtifactError> {
    check_artifact(schema_path(), &dashboard_schema()?)?;
    check_artifact(transport_path(), &transport_bindings())?;
    Ok(())
}

pub fn dashboard_schema() -> Result<String, ArtifactError> {
    let mut schema = serde_json::to_value(schema_for!(DashboardFeatureFile))?;
    let object = schema
        .as_object_mut()
        .ok_or(ArtifactError::UnexpectedSchemaShape)?;
    object.insert(
        "$id".to_owned(),
        Value::String("https://personal-dashboard.local/schema.json".to_owned()),
    );
    object.insert(
        "allOf".to_owned(),
        json!([{ "$ref": OPTIFY_FEATURE_SCHEMA }]),
    );
    object.insert(
        "description".to_owned(),
        Value::String(
            "Optify feature file whose options are validated by Personal Dashboard.".to_owned(),
        ),
    );
    object.insert(
        "title".to_owned(),
        Value::String("Personal Dashboard Optify Feature".to_owned()),
    );
    let mut output = serde_json::to_string_pretty(&schema)?;
    output.push('\n');
    Ok(output)
}

pub fn transport_bindings() -> String {
    let declarations = [
        ActiveConfiguration::decl(),
        BootstrapResponse::decl(),
        ClientMessage::decl(),
        ClientRequest::decl(),
        ErrorCode::decl(),
        OptifySetup::decl(),
        ServerEvent::decl(),
        ServerMessage::decl(),
        ServerResponse::decl(),
        SetupStatus::decl(),
        Theme::decl(),
    ];
    format!(
        "// This file is generated from Rust transport types.\n// Run `pnpm run bindings:generate` instead of editing it.\n\nexport {}\n",
        declarations.join("\n\nexport ")
    )
}

pub fn write_generated_artifacts() -> Result<(), ArtifactError> {
    write_artifact(schema_path(), &dashboard_schema()?)?;
    write_artifact(transport_path(), &transport_bindings())?;
    Ok(())
}

fn check_artifact(path: PathBuf, expected: &str) -> Result<(), ArtifactError> {
    let actual = fs::read_to_string(&path).map_err(|source| ArtifactError::Read {
        path: path.clone(),
        source,
    })?;
    if actual != expected {
        return Err(ArtifactError::Outdated { path });
    }
    Ok(())
}

fn schema_path() -> PathBuf {
    manifest_path().join("configs/.optify/schema.json")
}

fn transport_path() -> PathBuf {
    manifest_path().join("src/generated/transport.ts")
}

fn manifest_path() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn write_artifact(path: PathBuf, contents: &str) -> Result<(), ArtifactError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ArtifactError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(&path, contents).map_err(|source| ArtifactError::Write { path, source })
}

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("generated artifact is stale: {path}")]
    Outdated { path: PathBuf },
    #[error("could not read generated artifact {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
    #[error("generated schema did not contain a root object")]
    UnexpectedSchemaShape,
    #[error("could not write generated artifact {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::check_generated_artifacts;

    #[test]
    fn generated_artifacts_are_current() {
        check_generated_artifacts().unwrap();
    }
}
