use std::time::Duration;

use thiserror::Error;

use crate::buttons::{ButtonError, prompt_button_label};
use crate::commands::{TemplateError, TemplateValues};
use crate::config::ConfigurationSnapshot;
use crate::messages::{ButtonList, DashboardItem};
use crate::processes::AutocompleteInvocation;

const MAX_AUTOCOMPLETE_OUTPUT_BYTES: usize = 32 * 1024;
const MAX_AUTOCOMPLETE_REQUEST_BYTES: usize = 64 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 256;

#[derive(Debug, Error)]
pub enum AutocompleteError {
    #[error(transparent)]
    Button(#[from] ButtonError),
    #[error("the autocomplete draft is shorter than the configured minimum")]
    DraftTooShort,
    #[error("autocomplete identifiers must contain 1 to 256 bytes")]
    Identifier,
    #[error("the autocomplete request is too large")]
    RequestTooLarge,
    #[error("the autocomplete selection is outside the draft")]
    Selection,
    #[error(transparent)]
    Template(#[from] TemplateError),
}

#[allow(clippy::too_many_arguments)]
pub fn resolve_autocomplete(
    configuration: &ConfigurationSnapshot,
    item: &DashboardItem,
    button_list: ButtonList,
    button_index: usize,
    draft: &str,
    selection_end: usize,
    selection_start: usize,
) -> Result<AutocompleteInvocation, AutocompleteError> {
    let autocomplete = &configuration.configuration.root.autocomplete;
    if draft.chars().count() < autocomplete.minimum_characters {
        return Err(AutocompleteError::DraftTooShort);
    }
    let draft_code_units = draft.encode_utf16().count();
    if selection_start > selection_end || selection_end > draft_code_units {
        return Err(AutocompleteError::Selection);
    }
    let button_label =
        prompt_button_label(configuration, item.item_kind, button_list, button_index)?;
    let request = format!(
        "{}\n\nAction: {}\nItem: {}#{}\nURL: {}\nSelection: {}..{}\n\nDraft:\n{}",
        autocomplete.instruction,
        button_label,
        item.repository,
        item.number,
        item.url,
        selection_start,
        selection_end,
        draft,
    );
    if request.len() > MAX_AUTOCOMPLETE_REQUEST_BYTES {
        return Err(AutocompleteError::RequestTooLarge);
    }
    let values = TemplateValues::for_item(item).with_autocomplete_request(&request);
    let template = &configuration.configuration.autocomplete;
    Ok(AutocompleteInvocation {
        arguments: template.resolve_arguments(&values)?,
        max_concurrent_commands: configuration
            .configuration
            .root
            .application
            .max_concurrent_commands,
        max_output_bytes: configuration
            .configuration
            .root
            .application
            .max_output_bytes_per_run
            .min(MAX_AUTOCOMPLETE_OUTPUT_BYTES),
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

pub fn validate_identifier(value: &str) -> Result<(), AutocompleteError> {
    if value.trim().is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
        return Err(AutocompleteError::Identifier);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::resolve_autocomplete;
    use crate::config::{ConfigService, ItemKind, RuntimeSchema};
    use crate::messages::{ButtonList, DashboardItem, OptifySetup};

    #[test]
    fn resolves_untrusted_drafts_only_as_positional_values() {
        let (config_service, _reload_service) =
            ConfigService::new(RuntimeSchema::materialize().unwrap());
        let configuration = config_service
            .apply_setup(OptifySetup {
                config_directories: vec![
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("configs")
                        .display()
                        .to_string(),
                ],
                features: vec!["dashboard".to_owned()],
            })
            .unwrap();
        let item = DashboardItem {
            advanced_buttons: Vec::new(),
            approved_by: Vec::new(),
            assignees: Vec::new(),
            always_buttons: Vec::new(),
            author: Some("octocat".to_owned()),
            checks_status: None,
            is_draft: Some(false),
            item_kind: ItemKind::PullRequest,
            labels: Vec::new(),
            merge_status: None,
            number: 42,
            repository: "example/project".to_owned(),
            source: Some("github".to_owned()),
            state: "open".to_owned(),
            target_branch: None,
            title: "Improve boundary handling".to_owned(),
            updated_at: "2026-08-26T12:00:00Z".to_owned(),
            url: "https://example.test/pulls/42".to_owned(),
        };
        let draft = "focus on tests; printf unsafe";

        let invocation = resolve_autocomplete(
            &configuration,
            &item,
            ButtonList::Always,
            0,
            draft,
            draft.len(),
            draft.len(),
        )
        .unwrap();

        assert!(!invocation.script.contains(draft));
        assert_eq!(invocation.arguments.len(), 1);
        assert!(invocation.arguments[0].contains("Action: Review"));
        assert!(invocation.arguments[0].contains("Item: example/project#42"));
        assert!(invocation.arguments[0].contains(draft));
    }
}
