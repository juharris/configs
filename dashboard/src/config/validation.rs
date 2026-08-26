use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;

use super::types::{ButtonConfig, ButtonListsConfig, RootConfig};
use crate::commands::{CommandTemplate, Placeholder, PlainTemplate, TemplateError};

#[derive(Clone, Debug)]
pub struct ValidatedButton {
    pub command: Option<CommandTemplate>,
    pub url: Option<PlainTemplate>,
    pub working_directory: Option<PlainTemplate>,
}

#[derive(Clone, Debug)]
pub struct ValidatedButtonLists {
    pub advanced: Vec<ValidatedButton>,
    pub always: Vec<ValidatedButton>,
}

#[derive(Clone, Debug)]
pub struct ValidatedRootConfig {
    pub autocomplete: CommandTemplate,
    pub issue_buttons: ValidatedButtonLists,
    pub pull_request_buttons: ValidatedButtonLists,
    pub root: Arc<RootConfig>,
    pub sections: Vec<CommandTemplate>,
}

impl ValidatedRootConfig {
    pub fn validate(root: RootConfig) -> Result<Self, ConfigError> {
        validate_application(&root)?;
        validate_repositories(&root)?;

        let shell = Path::new(&root.application.shell);
        let autocomplete = CommandTemplate::compile(
            root.autocomplete.command.clone(),
            &Placeholder::autocomplete(),
        )
        .and_then(|template| {
            if !template.contains(Placeholder::AutocompleteRequest) {
                return Err(TemplateError::UnavailablePlaceholder {
                    placeholder: "autocomplete.request is required".to_owned(),
                });
            }
            template.validate_bash_syntax(shell)?;
            Ok(template)
        })
        .map_err(|error| ConfigError::field("autocomplete.command", error))?;

        let issue_buttons = validate_button_lists(&root.buttons.issues, "buttons.issues", shell)?;
        let pull_request_buttons =
            validate_button_lists(&root.buttons.pull_requests, "buttons.pull_requests", shell)?;
        let sections = validate_sections(&root, shell)?;

        Ok(Self {
            autocomplete,
            issue_buttons,
            pull_request_buttons,
            root: Arc::new(root),
            sections,
        })
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("invalid configuration at {field}: {message}")]
pub struct ConfigError {
    pub field: String,
    pub message: String,
}

impl ConfigError {
    pub fn field(field: impl Into<String>, error: impl std::fmt::Display) -> Self {
        Self {
            field: field.into(),
            message: error.to_string(),
        }
    }
}

fn validate_application(root: &RootConfig) -> Result<(), ConfigError> {
    for (field, value) in [
        (
            "application.command_timeout_seconds",
            root.application.command_timeout_seconds,
        ),
        (
            "application.default_refresh_seconds",
            root.application.default_refresh_seconds,
        ),
    ] {
        if value == 0 {
            return Err(ConfigError::field(field, "must be greater than zero"));
        }
    }
    for (field, value) in [
        (
            "application.max_concurrent_commands",
            root.application.max_concurrent_commands,
        ),
        (
            "application.max_output_bytes_per_run",
            root.application.max_output_bytes_per_run,
        ),
        (
            "autocomplete.minimum_characters",
            root.autocomplete.minimum_characters,
        ),
    ] {
        if value == 0 {
            return Err(ConfigError::field(field, "must be greater than zero"));
        }
    }
    if root.autocomplete.debounce_milliseconds == 0 {
        return Err(ConfigError::field(
            "autocomplete.debounce_milliseconds",
            "must be greater than zero",
        ));
    }
    require_nonblank("autocomplete.instruction", &root.autocomplete.instruction)?;

    let shell = Path::new(&root.application.shell);
    if !shell.is_absolute() {
        return Err(ConfigError::field(
            "application.shell",
            "must be an absolute path",
        ));
    }
    let metadata = fs::metadata(shell).map_err(|error| {
        ConfigError::field(
            "application.shell",
            format!("cannot read {}: {error}", shell.display()),
        )
    })?;
    if !metadata.is_file() {
        return Err(ConfigError::field(
            "application.shell",
            format!("{} is not a file", shell.display()),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(ConfigError::field(
                "application.shell",
                format!("{} is not executable", shell.display()),
            ));
        }
    }
    Ok(())
}

fn validate_button(
    button: &ButtonConfig,
    field: &str,
    shell: &Path,
) -> Result<ValidatedButton, ConfigError> {
    require_nonblank(&format!("{field}.label"), &button.label)?;
    if button.command.is_some() == button.url.is_some() {
        return Err(ConfigError::field(
            field,
            "must define exactly one of command or url",
        ));
    }

    if let Some(url) = &button.url {
        if button.confirm || button.prompt.is_some() || button.working_directory.is_some() {
            return Err(ConfigError::field(
                field,
                "confirm, prompt, and working_directory are available only for command buttons",
            ));
        }
        let template = PlainTemplate::compile(url.clone(), &Placeholder::item())
            .and_then(|template| {
                template.validate_https_url()?;
                Ok(template)
            })
            .map_err(|error| ConfigError::field(format!("{field}.url"), error))?;
        return Ok(ValidatedButton {
            command: None,
            url: Some(template),
            working_directory: None,
        });
    }

    let command = CommandTemplate::compile(
        button
            .command
            .clone()
            .expect("command existence was checked"),
        &Placeholder::button(),
    )
    .and_then(|template| {
        let declares_prompt = button.prompt.is_some();
        if declares_prompt != template.contains(Placeholder::Prompt) {
            return Err(TemplateError::UnavailablePlaceholder {
                placeholder: "prompt declarations and {prompt} must be used together".to_owned(),
            });
        }
        template.validate_bash_syntax(shell)?;
        Ok(template)
    })
    .map_err(|error| ConfigError::field(format!("{field}.command"), error))?;

    if let Some(prompt) = &button.prompt {
        require_nonblank(&format!("{field}.prompt.label"), &prompt.label)?;
        require_nonblank(&format!("{field}.prompt.placeholder"), &prompt.placeholder)?;
    }
    let working_directory = button
        .working_directory
        .as_ref()
        .map(|template| {
            PlainTemplate::compile(
                template.clone(),
                &HashSet::from([Placeholder::RepositoryPath]),
            )
            .map_err(|error| ConfigError::field(format!("{field}.working_directory"), error))
        })
        .transpose()?;

    Ok(ValidatedButton {
        command: Some(command),
        url: None,
        working_directory,
    })
}

fn validate_button_lists(
    buttons: &ButtonListsConfig,
    field: &str,
    shell: &Path,
) -> Result<ValidatedButtonLists, ConfigError> {
    let validate = |(index, button): (usize, &ButtonConfig), list_name: &str| {
        validate_button(button, &format!("{field}.{list_name}[{index}]"), shell)
    };
    Ok(ValidatedButtonLists {
        advanced: buttons
            .advanced
            .iter()
            .enumerate()
            .map(|button| validate(button, "advanced"))
            .collect::<Result<_, _>>()?,
        always: buttons
            .always
            .iter()
            .enumerate()
            .map(|button| validate(button, "always"))
            .collect::<Result<_, _>>()?,
    })
}

fn validate_repositories(root: &RootConfig) -> Result<(), ConfigError> {
    for (repository, config) in &root.repositories {
        let field = format!("repositories.{repository}.path");
        let mut parts = repository.split('/');
        if parts.next().is_none_or(str::is_empty)
            || parts.next().is_none_or(str::is_empty)
            || parts.next().is_some()
        {
            return Err(ConfigError::field(
                format!("repositories.{repository}"),
                "repository key must have the owner/name form",
            ));
        }
        let path = PathBuf::from(&config.path);
        if !path.is_absolute() {
            return Err(ConfigError::field(field, "must be an absolute path"));
        }
    }
    Ok(())
}

fn validate_sections(root: &RootConfig, shell: &Path) -> Result<Vec<CommandTemplate>, ConfigError> {
    let mut ids = HashSet::new();
    root.sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let field = format!("sections[{index}]");
            require_nonblank(&format!("{field}.id"), &section.id)?;
            require_nonblank(&format!("{field}.title"), &section.title)?;
            if !ids.insert(&section.id) {
                return Err(ConfigError::field(
                    format!("{field}.id"),
                    format!("duplicate section ID {}", section.id),
                ));
            }
            if section.refresh_seconds == Some(0) {
                return Err(ConfigError::field(
                    format!("{field}.refresh_seconds"),
                    "must be greater than zero",
                ));
            }
            CommandTemplate::compile(section.command.clone(), &HashSet::new())
                .and_then(|template| {
                    template.validate_bash_syntax(shell)?;
                    Ok(template)
                })
                .map_err(|error| ConfigError::field(format!("{field}.command"), error))
        })
        .collect()
}

fn require_nonblank(field: &str, value: &str) -> Result<(), ConfigError> {
    if value.trim().is_empty() {
        return Err(ConfigError::field(field, "cannot be blank"));
    }
    Ok(())
}
