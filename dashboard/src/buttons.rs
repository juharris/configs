use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

use crate::commands::{TemplateError, TemplateValues};
use crate::config::{
    ButtonConfig, ButtonListsConfig, ConfigurationSnapshot, ItemKind, ValidatedButton,
    ValidatedButtonLists,
};
use crate::messages::{ButtonList, DashboardButton, DashboardItem, PromptPresentation};

#[derive(Clone, Debug)]
pub struct ResolvedCommand {
    pub arguments: Vec<String>,
    pub label: String,
    pub max_concurrent_commands: usize,
    pub max_output_bytes: usize,
    pub preview: String,
    pub script: String,
    pub shell: PathBuf,
    pub timeout: Duration,
}

#[derive(Debug, Error)]
pub enum ButtonError {
    #[error("the button index is not configured")]
    InvalidButton,
    #[error("the button prompt is not valid for this action")]
    InvalidPrompt,
    #[error(transparent)]
    Template(#[from] TemplateError),
}

pub fn decorate_item(configuration: &ConfigurationSnapshot, item: &mut DashboardItem) {
    let (buttons, validated) = button_lists(configuration, item.item_kind);
    item.advanced_buttons = presentations(item, &buttons.advanced, &validated.advanced);
    item.always_buttons = presentations(item, &buttons.always, &validated.always);
}

pub fn resolve_command(
    configuration: &ConfigurationSnapshot,
    item: &DashboardItem,
    list: ButtonList,
    index: usize,
    prompt: Option<&str>,
) -> Result<ResolvedCommand, ButtonError> {
    let (buttons, validated) = button_lists(configuration, item.item_kind);
    let (button, validated) = match list {
        ButtonList::Advanced => (buttons.advanced.get(index), validated.advanced.get(index)),
        ButtonList::Always => (buttons.always.get(index), validated.always.get(index)),
    };
    let button = button.ok_or(ButtonError::InvalidButton)?;
    let validated = validated.ok_or(ButtonError::InvalidButton)?;
    if button.command.is_none() {
        return Err(ButtonError::InvalidButton);
    }
    if button.prompt.is_some() != prompt.is_some() {
        return Err(ButtonError::InvalidPrompt);
    }

    let values = TemplateValues::for_item(item).with_prompt(prompt);
    let template = validated
        .command
        .as_ref()
        .ok_or(ButtonError::InvalidButton)?;
    let preview = template.preview(&values)?;
    Ok(ResolvedCommand {
        arguments: template.resolve_arguments(&values)?,
        label: button.label.clone(),
        max_concurrent_commands: configuration
            .configuration
            .root
            .application
            .max_concurrent_commands,
        max_output_bytes: configuration
            .configuration
            .root
            .application
            .max_output_bytes_per_run,
        preview,
        script: template.script().to_owned(),
        shell: configuration
            .configuration
            .root
            .application
            .shell
            .clone()
            .into(),
        timeout: Duration::from_secs(
            configuration
                .configuration
                .root
                .application
                .command_timeout_seconds,
        ),
    })
}

fn button_lists(
    configuration: &ConfigurationSnapshot,
    item_kind: ItemKind,
) -> (&ButtonListsConfig, &ValidatedButtonLists) {
    match item_kind {
        ItemKind::Issue => (
            &configuration.configuration.root.buttons.issues,
            &configuration.configuration.issue_buttons,
        ),
        ItemKind::PullRequest => (
            &configuration.configuration.root.buttons.pull_requests,
            &configuration.configuration.pull_request_buttons,
        ),
    }
}

fn presentation(
    item: &DashboardItem,
    button: &ButtonConfig,
    index: usize,
    validated: &ValidatedButton,
) -> DashboardButton {
    let resolved: Result<(String, Option<String>), ButtonError> = (|| {
        let values = TemplateValues::for_item(item).with_prompt(button.prompt.as_ref().map(|_| ""));
        if let Some(url) = &validated.url {
            let url = url.resolve_https_url(&values)?;
            return Ok((url.clone(), Some(url)));
        }
        let command = validated
            .command
            .as_ref()
            .ok_or(ButtonError::InvalidButton)?;
        Ok((command.preview(&values)?, None))
    })();
    let (disabled, title, url) = match resolved {
        Ok((title, url)) => (false, title, url),
        Err(error) => (true, error.to_string(), None),
    };
    DashboardButton {
        confirm: button.confirm,
        disabled,
        index,
        label: button.label.clone(),
        prompt: button.prompt.as_ref().map(|prompt| PromptPresentation {
            label: prompt.label.clone(),
            placeholder: prompt.placeholder.clone(),
        }),
        title,
        url,
    }
}

fn presentations(
    item: &DashboardItem,
    buttons: &[ButtonConfig],
    validated: &[ValidatedButton],
) -> Vec<DashboardButton> {
    buttons
        .iter()
        .zip(validated)
        .enumerate()
        .map(|(index, (button, validated))| presentation(item, button, index, validated))
        .collect()
}
