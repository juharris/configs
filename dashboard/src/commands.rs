use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::process::Stdio;

use thiserror::Error;
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandTemplate {
    placeholders: Vec<Placeholder>,
    script: String,
    source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlainTemplate {
    placeholders: Vec<Placeholder>,
    source: String,
}

impl PlainTemplate {
    pub fn compile(
        source: impl Into<String>,
        allowed_placeholders: &HashSet<Placeholder>,
    ) -> Result<Self, TemplateError> {
        let source = source.into();
        if source.trim().is_empty() {
            return Err(TemplateError::Empty);
        }
        let placeholders = parse_plain_placeholders(&source, allowed_placeholders)?;
        Ok(Self {
            placeholders,
            source,
        })
    }

    pub fn contains(&self, placeholder: Placeholder) -> bool {
        self.placeholders.contains(&placeholder)
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn validate_https_url(&self) -> Result<(), TemplateError> {
        let mut resolved = self.source.clone();
        for placeholder in &self.placeholders {
            resolved = resolved.replacen(
                &format!("{{{placeholder}}}"),
                placeholder.validation_value(),
                1,
            );
        }
        let url = Url::parse(&resolved).map_err(|source| TemplateError::InvalidUrl { source })?;
        if url.scheme() != "https" {
            return Err(TemplateError::InvalidUrlScheme {
                scheme: url.scheme().to_owned(),
            });
        }
        Ok(())
    }
}

impl CommandTemplate {
    pub fn compile(
        source: impl Into<String>,
        allowed_placeholders: &HashSet<Placeholder>,
    ) -> Result<Self, TemplateError> {
        let source = source.into();
        if source.trim().is_empty() {
            return Err(TemplateError::Empty);
        }

        let (script, placeholders) = compile_template(&source, allowed_placeholders)?;
        Ok(Self {
            placeholders,
            script,
            source,
        })
    }

    pub fn contains(&self, placeholder: Placeholder) -> bool {
        self.placeholders.contains(&placeholder)
    }

    pub fn placeholders(&self) -> &[Placeholder] {
        &self.placeholders
    }

    pub fn script(&self) -> &str {
        &self.script
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn validate_bash_syntax(&self, shell: &Path) -> Result<(), TemplateError> {
        let output = std::process::Command::new(shell)
            .args(["-n", "-c", &self.script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|source| TemplateError::Shell {
                path: shell.display().to_string(),
                source,
            })?;

        if output.status.success() {
            return Ok(());
        }

        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(TemplateError::InvalidBash { message })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Placeholder {
    AutocompleteRequest,
    ItemNumber,
    ItemRepository,
    ItemUrl,
    Prompt,
    RepositoryPath,
}

impl Placeholder {
    pub fn autocomplete() -> HashSet<Self> {
        HashSet::from([Self::AutocompleteRequest])
    }

    pub fn button() -> HashSet<Self> {
        HashSet::from([
            Self::ItemNumber,
            Self::ItemRepository,
            Self::ItemUrl,
            Self::Prompt,
            Self::RepositoryPath,
        ])
    }

    pub fn item() -> HashSet<Self> {
        HashSet::from([
            Self::ItemNumber,
            Self::ItemRepository,
            Self::ItemUrl,
            Self::RepositoryPath,
        ])
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "autocomplete.request" => Some(Self::AutocompleteRequest),
            "item.number" => Some(Self::ItemNumber),
            "item.repository" => Some(Self::ItemRepository),
            "item.url" => Some(Self::ItemUrl),
            "prompt" => Some(Self::Prompt),
            "repository.path" => Some(Self::RepositoryPath),
            _ => None,
        }
    }

    fn validation_value(self) -> &'static str {
        match self {
            Self::AutocompleteRequest => "request",
            Self::ItemNumber => "1",
            Self::ItemRepository => "owner/repository",
            Self::ItemUrl => "https://github.com/owner/repository/issues/1",
            Self::Prompt => "prompt",
            Self::RepositoryPath => "/tmp/repository",
        }
    }
}

impl Display for Placeholder {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::AutocompleteRequest => "autocomplete.request",
            Self::ItemNumber => "item.number",
            Self::ItemRepository => "item.repository",
            Self::ItemUrl => "item.url",
            Self::Prompt => "prompt",
            Self::RepositoryPath => "repository.path",
        })
    }
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("command template cannot be blank")]
    Empty,
    #[error("Bash syntax is invalid: {message}")]
    InvalidBash { message: String },
    #[error("URL template does not resolve to a valid URL: {source}")]
    InvalidUrl {
        #[source]
        source: url::ParseError,
    },
    #[error("URL template must resolve to HTTPS, not {scheme}")]
    InvalidUrlScheme { scheme: String },
    #[error("template has an unterminated placeholder starting at byte {offset}")]
    MalformedPlaceholder { offset: usize },
    #[error("could not run configured shell at {path}: {source}")]
    Shell {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("placeholder {{{placeholder}}} is not available in this context")]
    UnavailablePlaceholder { placeholder: String },
    #[error("unknown placeholder {{{placeholder}}}")]
    UnknownPlaceholder { placeholder: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Quote {
    Double,
    Single,
    Unquoted,
}

fn compile_template(
    source: &str,
    allowed_placeholders: &HashSet<Placeholder>,
) -> Result<(String, Vec<Placeholder>), TemplateError> {
    let mut compiled = String::with_capacity(source.len());
    let mut characters = source.char_indices().peekable();
    let mut escaped = false;
    let mut parameter_expansion_depth = 0_u32;
    let mut placeholders = Vec::new();
    let mut quote = Quote::Unquoted;

    while let Some((offset, character)) = characters.next() {
        if escaped {
            compiled.push(character);
            escaped = false;
            continue;
        }

        if character == '\\' && quote != Quote::Single {
            compiled.push(character);
            escaped = true;
            continue;
        }

        match character {
            '\'' if quote != Quote::Double => {
                quote = if quote == Quote::Single {
                    Quote::Unquoted
                } else {
                    Quote::Single
                };
                compiled.push(character);
            }
            '"' if quote != Quote::Single => {
                quote = if quote == Quote::Double {
                    Quote::Unquoted
                } else {
                    Quote::Double
                };
                compiled.push(character);
            }
            '{' if compiled.ends_with('$') || parameter_expansion_depth > 0 => {
                parameter_expansion_depth += 1;
                compiled.push(character);
            }
            '}' if parameter_expansion_depth > 0 => {
                parameter_expansion_depth -= 1;
                compiled.push(character);
            }
            '{' => {
                let placeholder_end =
                    characters
                        .clone()
                        .find_map(|(next_offset, next_character)| {
                            (next_character == '}').then_some(next_offset)
                        });
                let Some(placeholder_end) = placeholder_end else {
                    return Err(TemplateError::MalformedPlaceholder { offset });
                };
                let placeholder_name = &source[offset + 1..placeholder_end];
                if !is_placeholder_name(placeholder_name) {
                    compiled.push(character);
                    continue;
                }

                while characters
                    .peek()
                    .is_some_and(|(next_offset, _)| *next_offset <= placeholder_end)
                {
                    characters.next();
                }

                let placeholder = Placeholder::parse(placeholder_name).ok_or_else(|| {
                    TemplateError::UnknownPlaceholder {
                        placeholder: placeholder_name.to_owned(),
                    }
                })?;
                if !allowed_placeholders.contains(&placeholder) {
                    return Err(TemplateError::UnavailablePlaceholder {
                        placeholder: placeholder_name.to_owned(),
                    });
                }

                placeholders.push(placeholder);
                let position = placeholders.len();
                match quote {
                    Quote::Double => compiled.push_str(&format!("${{{position}}}")),
                    Quote::Single => compiled.push_str(&format!("'\"${{{position}}}\"'")),
                    Quote::Unquoted => compiled.push_str(&format!("\"${{{position}}}\"")),
                }
            }
            _ => compiled.push(character),
        }
    }

    Ok((compiled, placeholders))
}

fn is_placeholder_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_'))
}

fn parse_plain_placeholders(
    source: &str,
    allowed_placeholders: &HashSet<Placeholder>,
) -> Result<Vec<Placeholder>, TemplateError> {
    let mut characters = source.char_indices().peekable();
    let mut placeholders = Vec::new();

    while let Some((offset, character)) = characters.next() {
        if character != '{' || source[..offset].ends_with('$') {
            continue;
        }
        let placeholder_end = characters
            .clone()
            .find_map(|(next_offset, next_character)| {
                (next_character == '}').then_some(next_offset)
            })
            .ok_or(TemplateError::MalformedPlaceholder { offset })?;
        let placeholder_name = &source[offset + 1..placeholder_end];
        if !is_placeholder_name(placeholder_name) {
            continue;
        }
        while characters
            .peek()
            .is_some_and(|(next_offset, _)| *next_offset <= placeholder_end)
        {
            characters.next();
        }

        let placeholder = Placeholder::parse(placeholder_name).ok_or_else(|| {
            TemplateError::UnknownPlaceholder {
                placeholder: placeholder_name.to_owned(),
            }
        })?;
        if !allowed_placeholders.contains(&placeholder) {
            return Err(TemplateError::UnavailablePlaceholder {
                placeholder: placeholder_name.to_owned(),
            });
        }
        placeholders.push(placeholder);
    }

    Ok(placeholders)
}

#[cfg(test)]
mod tests {
    use super::{CommandTemplate, Placeholder, TemplateError};
    use assert_matches::assert_matches;

    #[test]
    fn compiles_values_as_positional_parameters_in_every_quote_context() {
        let template = CommandTemplate::compile(
            "tool {item.url} \"prefix {item.repository}\" '/review {item.number} {prompt}'",
            &Placeholder::button(),
        )
        .unwrap();

        assert_eq!(
            template.script(),
            "tool \"${1}\" \"prefix ${2}\" '/review '\"${3}\"' '\"${4}\"''"
        );
        assert_eq!(
            template.placeholders(),
            &[
                Placeholder::ItemUrl,
                Placeholder::ItemRepository,
                Placeholder::ItemNumber,
                Placeholder::Prompt,
            ]
        );
    }

    #[test]
    fn preserves_bash_parameter_expansion() {
        let template =
            CommandTemplate::compile("printf '%s' \"${VALUE:-fallback}\"", &Placeholder::button())
                .unwrap();

        assert_eq!(template.script(), "printf '%s' \"${VALUE:-fallback}\"");
    }

    #[test]
    fn rejects_unknown_and_unavailable_placeholders() {
        let unknown = CommandTemplate::compile("tool {item.body}", &Placeholder::button());
        assert_matches!(unknown, Err(TemplateError::UnknownPlaceholder { .. }));

        let unavailable = CommandTemplate::compile("tool {prompt}", &Placeholder::autocomplete());
        assert_matches!(
            unavailable,
            Err(TemplateError::UnavailablePlaceholder { .. })
        );
    }
}
