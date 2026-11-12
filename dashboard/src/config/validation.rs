use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use thiserror::Error;

use super::types::{
    ButtonConfig, ButtonListsConfig, ChatCommandsConfig, ChatConfig, ChatModelsConfig, RootConfig,
};
use crate::commands::{CommandTemplate, Placeholder, PlainTemplate, TemplateError};

#[derive(Clone, Debug)]
pub struct ValidatedButton {
    pub command: Option<CommandTemplate>,
    pub url: Option<PlainTemplate>,
}

#[derive(Clone, Debug)]
pub struct ValidatedButtonLists {
    pub advanced: Vec<ValidatedButton>,
    pub always: Vec<ValidatedButton>,
}

/// Holds the chat configuration with its compiled command templates.
#[derive(Clone, Debug)]
pub struct ValidatedChat {
    pub acknowledgement: Option<CommandTemplate>,
    pub config: ChatConfig,
    pub listener: CommandTemplate,
    pub recovery: Option<CommandTemplate>,
    pub reply: CommandTemplate,
    pub resume: CommandTemplate,
    pub start: CommandTemplate,
}

#[derive(Clone, Debug)]
pub struct ValidatedRootConfig {
    pub autocomplete: CommandTemplate,
    pub chat: Option<Arc<ValidatedChat>>,
    pub issue_buttons: ValidatedButtonLists,
    pub pull_request_buttons: ValidatedButtonLists,
    pub root: Arc<RootConfig>,
    pub sections: Vec<ValidatedSection>,
}

#[derive(Clone, Debug)]
pub struct ValidatedSection {
    pub command: CommandTemplate,
}

impl ValidatedRootConfig {
    pub fn validate(root: RootConfig) -> Result<Self, ConfigError> {
        validate_application(&root)?;

        let shell = Path::new(&root.application.shell);
        let autocomplete = compile_command(
            "autocomplete.command",
            &root.autocomplete.command,
            &Placeholder::autocomplete(),
            &[Placeholder::AutocompleteRequest],
            shell,
        )?;

        let chat = root
            .chat
            .as_ref()
            .map(|chat| validate_chat(chat, &root.application.working_directories, shell))
            .transpose()?
            .map(Arc::new);
        let issue_buttons = validate_button_lists(&root.buttons.issues, "buttons.issues", shell)?;
        let pull_request_buttons =
            validate_button_lists(&root.buttons.pull_requests, "buttons.pull_requests", shell)?;
        let sections = validate_sections(&root, shell)?;

        Ok(Self {
            autocomplete,
            chat,
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

/// Compiles a command template, requires its mandatory placeholders, and checks its Bash syntax.
fn compile_command(
    field: &str,
    source: &str,
    allowed_placeholders: &HashSet<Placeholder>,
    required_placeholders: &[Placeholder],
    shell: &Path,
) -> Result<CommandTemplate, ConfigError> {
    let template = CommandTemplate::compile(source, allowed_placeholders)
        .map_err(|error| ConfigError::field(field, error))?;
    if let Some(missing) = required_placeholders
        .iter()
        .find(|placeholder| !template.contains(**placeholder))
    {
        return Err(ConfigError::field(
            field,
            format!("must use the {{{missing}}} placeholder"),
        ));
    }
    template
        .validate_bash_syntax(shell)
        .map_err(|error| ConfigError::field(field, error))?;
    Ok(template)
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

    if root.application.working_directories.is_empty() {
        return Err(ConfigError::field(
            "application.working_directories",
            "must contain at least one directory",
        ));
    }
    let mut working_directories = HashSet::new();
    for (index, directory) in root.application.working_directories.iter().enumerate() {
        let field = format!("application.working_directories[{index}]");
        let path = Path::new(directory);
        if !path.is_absolute() {
            return Err(ConfigError::field(field, "must be an absolute path"));
        }
        let metadata = fs::metadata(path).map_err(|error| {
            ConfigError::field(
                field.clone(),
                format!("cannot read {}: {error}", path.display()),
            )
        })?;
        if !metadata.is_dir() {
            return Err(ConfigError::field(
                field,
                format!("{} is not a directory", path.display()),
            ));
        }
        if !working_directories.insert(directory) {
            return Err(ConfigError::field(
                "application.working_directories",
                format!("contains the duplicate directory {directory}"),
            ));
        }
    }

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
        if button.detached {
            return Err(ConfigError::field(
                field,
                "detached is available only for command buttons",
            ));
        }
        if button.prompt.is_some() {
            return Err(ConfigError::field(
                field,
                "prompt is available only for command buttons",
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
        if let Some(default) = &prompt.default {
            require_nonblank(&format!("{field}.prompt.default"), default)?;
        }
        require_nonblank(&format!("{field}.prompt.label"), &prompt.label)?;
        require_nonblank(&format!("{field}.prompt.placeholder"), &prompt.placeholder)?;
    }
    Ok(ValidatedButton {
        command: Some(command),
        url: None,
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

fn validate_chat(
    chat: &ChatConfig,
    working_directories: &[String],
    shell: &Path,
) -> Result<ValidatedChat, ConfigError> {
    if !working_directories.contains(&chat.working_directory) {
        return Err(ConfigError::field(
            "chat.working_directory",
            "must be one of application.working_directories",
        ));
    }
    for (field, value) in [
        ("chat.agent.timeout_seconds", chat.agent.timeout_seconds),
        (
            "chat.listener.restart_seconds",
            chat.listener.restart_seconds,
        ),
        (
            "chat.listener.recovery_timeout_seconds",
            chat.listener.recovery_timeout_seconds,
        ),
        (
            "chat.listener.startup_timeout_seconds",
            chat.listener.startup_timeout_seconds,
        ),
    ] {
        if value == 0 {
            return Err(ConfigError::field(field, "must be greater than zero"));
        }
    }
    validate_chat_commands(&chat.commands)?;
    validate_chat_models(&chat.models)?;
    require_nonblank("chat.reply.prefix", &chat.reply.prefix)?;

    let agent = Placeholder::agent();
    Ok(ValidatedChat {
        acknowledgement: chat
            .acknowledgement
            .as_deref()
            .map(|source| {
                compile_command(
                    "chat.acknowledgement",
                    source,
                    &Placeholder::acknowledgement(),
                    &[Placeholder::MessageId],
                    shell,
                )
            })
            .transpose()?,
        config: chat.clone(),
        listener: compile_command(
            "chat.listener.command",
            &chat.listener.command,
            &HashSet::new(),
            &[],
            shell,
        )?,
        recovery: chat
            .listener
            .recovery
            .as_deref()
            .map(|source| {
                compile_command(
                    "chat.listener.recovery",
                    source,
                    &HashSet::new(),
                    &[],
                    shell,
                )
            })
            .transpose()?,
        reply: compile_command(
            "chat.reply.command",
            &chat.reply.command,
            &Placeholder::reply(),
            &[Placeholder::ConversationId, Placeholder::ReplyText],
            shell,
        )?,
        resume: compile_command(
            "chat.agent.resume",
            &chat.agent.resume,
            &agent,
            &[Placeholder::MessageText],
            shell,
        )?,
        start: compile_command(
            "chat.agent.start",
            &chat.agent.start,
            &agent,
            &[Placeholder::MessageText],
            shell,
        )?,
    })
}

fn validate_chat_commands(commands: &ChatCommandsConfig) -> Result<(), ConfigError> {
    let mut names = HashSet::new();
    for (key, command) in commands.entries() {
        let field = format!("chat.commands.{key}");
        require_nonblank(&format!("{field}.description"), &command.description)?;
        require_word(&format!("{field}.name"), &command.name)?;
        if !names.insert(&command.name) {
            return Err(ConfigError::field(
                format!("{field}.name"),
                format!("duplicate command name {}", command.name),
            ));
        }
    }
    Ok(())
}

fn validate_chat_models(models: &ChatModelsConfig) -> Result<(), ConfigError> {
    if models.available.is_empty() {
        return Err(ConfigError::field(
            "chat.models.available",
            "must contain at least one model",
        ));
    }
    let mut available = HashSet::new();
    for (index, model) in models.available.iter().enumerate() {
        let field = format!("chat.models.available[{index}]");
        require_word(&field, model)?;
        if !available.insert(model) {
            return Err(ConfigError::field(
                field,
                format!("duplicate model {model}"),
            ));
        }
    }
    if !available.contains(&models.default) {
        return Err(ConfigError::field(
            "chat.models.default",
            "must be one of chat.models.available",
        ));
    }
    Ok(())
}

fn validate_sections(
    root: &RootConfig,
    shell: &Path,
) -> Result<Vec<ValidatedSection>, ConfigError> {
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
            if section.cache_ttl_seconds == 0 {
                return Err(ConfigError::field(
                    format!("{field}.cache_ttl_seconds"),
                    "must be greater than zero",
                ));
            }
            if section.refresh_seconds == Some(0) {
                return Err(ConfigError::field(
                    format!("{field}.refresh_seconds"),
                    "must be greater than zero",
                ));
            }
            if section.items_per_page == 0 {
                return Err(ConfigError::field(
                    format!("{field}.items_per_page"),
                    "must be greater than zero",
                ));
            }
            let command = compile_command(
                &format!("{field}.command"),
                &section.command,
                &HashSet::new(),
                &[],
                shell,
            )?;
            Ok(ValidatedSection { command })
        })
        .collect()
}

fn require_nonblank(field: &str, value: &str) -> Result<(), ConfigError> {
    if value.trim().is_empty() {
        return Err(ConfigError::field(field, "cannot be blank"));
    }
    Ok(())
}

/// Requires a single word because chat messages separate a command or model name from its argument with whitespace.
fn require_word(field: &str, value: &str) -> Result<(), ConfigError> {
    require_nonblank(field, value)?;
    if value.chars().any(char::is_whitespace) {
        return Err(ConfigError::field(field, "cannot contain whitespace"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{validate_button, validate_chat};
    use crate::chat::tests::chat_config;
    use crate::config::types::{ButtonConfig, ChatConfig, PromptConfig};

    /// Pairs a change that invalidates the chat configuration with the expected field and message.
    type InvalidChatCase = (fn(&mut ChatConfig), &'static str, &'static str);

    #[test]
    fn rejects_a_blank_prompt_default() {
        let button = ButtonConfig {
            command: Some("printf '%s' '{prompt}'".to_owned()),
            detached: false,
            label: "Review".to_owned(),
            prompt: Some(PromptConfig {
                default: Some("  ".to_owned()),
                label: "Review focus".to_owned(),
                placeholder: "Add areas to inspect".to_owned(),
            }),
            url: None,
        };

        let error =
            validate_button(&button, "buttons.always[0]", Path::new("/bin/bash")).unwrap_err();

        assert_eq!(error.field, "buttons.always[0].prompt.default");
        assert_eq!(error.message, "cannot be blank");
    }

    #[test]
    fn validates_chat_directories_commands_models_and_templates() {
        let directories = ["/tmp".to_owned()];
        let shell = Path::new("/bin/bash");
        assert!(validate_chat(&chat_config(), &directories, shell).is_ok());

        let cases: [InvalidChatCase; 9] = [
            (
                |chat| chat.working_directory = "/elsewhere".to_owned(),
                "chat.working_directory",
                "must be one of application.working_directories",
            ),
            (
                |chat| chat.agent.timeout_seconds = 0,
                "chat.agent.timeout_seconds",
                "must be greater than zero",
            ),
            (
                |chat| chat.commands.stop.name = "/ai".to_owned(),
                "chat.commands.stop.name",
                "duplicate command name /ai",
            ),
            (
                |chat| chat.commands.help.name = "/h elp".to_owned(),
                "chat.commands.help.name",
                "cannot contain whitespace",
            ),
            (
                |chat| chat.models.available.clear(),
                "chat.models.available",
                "must contain at least one model",
            ),
            (
                |chat| chat.models.default = "gpt".to_owned(),
                "chat.models.default",
                "must be one of chat.models.available",
            ),
            (
                |chat| chat.reply.prefix = " ".to_owned(),
                "chat.reply.prefix",
                "cannot be blank",
            ),
            (
                |chat| chat.agent.start = "agent".to_owned(),
                "chat.agent.start",
                "must use the {message.text} placeholder",
            ),
            (
                |chat| chat.listener.command = "listen {message.text}".to_owned(),
                "chat.listener.command",
                "placeholder {message.text} is not available in this context",
            ),
        ];
        for (change, field, message) in cases {
            let mut chat = chat_config();
            change(&mut chat);

            let error = validate_chat(&chat, &directories, shell).unwrap_err();

            assert_eq!(
                (error.field.as_str(), error.message.as_str()),
                (field, message)
            );
        }
    }
}
