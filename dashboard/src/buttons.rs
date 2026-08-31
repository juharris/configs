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
    pub detached: bool,
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

pub fn prompt_button_label(
    configuration: &ConfigurationSnapshot,
    item_kind: ItemKind,
    list: ButtonList,
    index: usize,
) -> Result<&str, ButtonError> {
    let (button, _) = configured_button(configuration, item_kind, list, index)?;
    if button.command.is_none() || button.prompt.is_none() {
        return Err(ButtonError::InvalidPrompt);
    }
    Ok(&button.label)
}

pub fn resolve_command(
    configuration: &ConfigurationSnapshot,
    item: &DashboardItem,
    list: ButtonList,
    index: usize,
    prompt: Option<&str>,
) -> Result<ResolvedCommand, ButtonError> {
    let (button, validated) = configured_button(configuration, item.item_kind, list, index)?;
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
        detached: button.detached,
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

fn configured_button(
    configuration: &ConfigurationSnapshot,
    item_kind: ItemKind,
    list: ButtonList,
    index: usize,
) -> Result<(&ButtonConfig, &ValidatedButton), ButtonError> {
    let (buttons, validated) = button_lists(configuration, item_kind);
    let (button, validated) = match list {
        ButtonList::Advanced => (buttons.advanced.get(index), validated.advanced.get(index)),
        ButtonList::Always => (buttons.always.get(index), validated.always.get(index)),
    };
    Ok((
        button.ok_or(ButtonError::InvalidButton)?,
        validated.ok_or(ButtonError::InvalidButton)?,
    ))
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
        let values = TemplateValues::for_item(item).with_prompt(
            button
                .prompt
                .as_ref()
                .map(|prompt| prompt.default.as_deref().unwrap_or("")),
        );
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
        disabled,
        index,
        label: button.label.clone(),
        prompt: button.prompt.as_ref().map(|prompt| PromptPresentation {
            default: prompt.default.clone(),
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::presentation;
    use crate::commands::{CommandTemplate, Placeholder};
    use crate::config::{ButtonConfig, ItemKind, ValidatedButton};
    use crate::messages::DashboardItem;

    #[test]
    fn presents_the_configured_prompt_default_in_the_button_title() {
        let button: ButtonConfig = serde_json::from_value(json!({
            "command": "tool {prompt}",
            "label": "Review",
            "prompt": {
                "default": "start in a new work tree",
                "label": "Review focus",
                "placeholder": "Add areas to inspect"
            }
        }))
        .unwrap();
        let item = DashboardItem {
            advanced_buttons: Vec::new(),
            approved_by: Vec::new(),
            assignees: Vec::new(),
            always_buttons: Vec::new(),
            author: None,
            checks_status: None,
            is_draft: Some(false),
            item_kind: ItemKind::PullRequest,
            labels: Vec::new(),
            merge_status: None,
            number: 42,
            repository: "example/project".to_owned(),
            source: None,
            state: "open".to_owned(),
            target_branch: None,
            title: "Keep the dashboard dense".to_owned(),
            updated_at: "2026-08-26T12:00:00Z".to_owned(),
            url: "https://example.test/pull/42".to_owned(),
        };
        let validated = ValidatedButton {
            command: Some(
                CommandTemplate::compile("tool {prompt}".to_owned(), &Placeholder::button())
                    .unwrap(),
            ),
            url: None,
        };

        let presentation = presentation(&item, &button, 0, &validated);

        assert_eq!(presentation.title, "tool 'start in a new work tree'");
        assert_eq!(
            presentation.prompt.unwrap().default.as_deref(),
            Some("start in a new work tree")
        );
    }
}
