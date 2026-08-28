use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use thiserror::Error;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use url::Url;

use crate::commands::CommandTemplate;
use crate::config::ItemKind;
use crate::messages::{DashboardActor, DashboardItem, DashboardLabel};

const MAX_ERROR_DETAIL_CHARACTERS: usize = 2_000;
const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug)]
pub struct DiscoveryRequest {
    pub cache_ttl: Duration,
    pub command: CommandTemplate,
    pub item_kind: ItemKind,
    pub max_output_bytes: usize,
    pub shell: PathBuf,
    pub timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct DiscoveryResult {
    pub items: Vec<DashboardItem>,
    pub refreshed_at: u64,
}

#[derive(Clone, Debug)]
struct CachedDiscovery {
    cached_at: Instant,
    result: DiscoveryResult,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct DiscoveryCacheKey {
    cache_ttl: Duration,
    command: String,
    item_kind: ItemKind,
    shell: PathBuf,
}

impl From<&DiscoveryRequest> for DiscoveryCacheKey {
    fn from(request: &DiscoveryRequest) -> Self {
        Self {
            cache_ttl: request.cache_ttl,
            command: request.command.source().to_owned(),
            item_kind: request.item_kind,
            shell: request.shell.clone(),
        }
    }
}

/// Runs complete configured discovery commands and validates their normalized output.
#[derive(Clone, Debug, Default)]
pub struct ItemDiscoverer {
    cache: Arc<RwLock<HashMap<DiscoveryCacheKey, CachedDiscovery>>>,
}

impl ItemDiscoverer {
    pub async fn discover(
        &self,
        request: DiscoveryRequest,
    ) -> Result<DiscoveryResult, DiscoveryError> {
        let cache_key = DiscoveryCacheKey::from(&request);
        if let Some(result) = self.cached_result(&cache_key).await {
            return Ok(result);
        }

        let output = execute(&request).await?;
        let items =
            normalize_items(&output, request.item_kind).map_err(DiscoveryError::InvalidItems)?;
        let result = DiscoveryResult {
            items,
            refreshed_at: timestamp_milliseconds(),
        };
        self.store(cache_key, result.clone()).await;
        Ok(result)
    }

    async fn cached_result(&self, key: &DiscoveryCacheKey) -> Option<DiscoveryResult> {
        self.cache
            .read()
            .await
            .get(key)
            .filter(|discovery| discovery.cached_at.elapsed() < key.cache_ttl)
            .map(|discovery| discovery.result.clone())
    }

    async fn store(&self, key: DiscoveryCacheKey, result: DiscoveryResult) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, discovery| discovery.cached_at.elapsed() < key.cache_ttl);
        cache.insert(
            key,
            CachedDiscovery {
                cached_at: Instant::now(),
                result,
            },
        );
    }
}

fn timestamp_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("discovery command returned invalid items: {0}")]
    InvalidItems(#[from] ItemValidationError),
    #[error("discovery command exited with status {status}")]
    NonZeroExit {
        detail: Option<String>,
        status: String,
    },
    #[error("discovery command exceeded the configured output limit")]
    OutputLimit,
    #[error("could not read discovery command output: {0}")]
    OutputRead(#[source] std::io::Error),
    #[error("could not start the configured discovery command: {0}")]
    Start(#[source] std::io::Error),
    #[error("discovery command timed out")]
    Timeout,
}

impl DiscoveryError {
    pub fn safe_message(&self) -> String {
        match self {
            Self::InvalidItems(error) => error.to_string(),
            Self::NonZeroExit { detail, status } => detail.as_ref().map_or_else(
                || format!("The section command exited with status {status}."),
                |detail| format!("The section command exited with status {status}.\n{detail}"),
            ),
            Self::OutputLimit => {
                "The section command exceeded the configured output limit.".to_owned()
            }
            Self::OutputRead(_) => "The section command output could not be read.".to_owned(),
            Self::Start(_) => "The section command could not be started.".to_owned(),
            Self::Timeout => "The section command timed out.".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("The section command returned invalid item {item}: {message}.")]
pub struct ItemValidationError {
    item: usize,
    message: String,
}

struct BoundedOutput {
    bytes: Vec<u8>,
    exceeded: bool,
}

async fn execute(request: &DiscoveryRequest) -> Result<Vec<u8>, DiscoveryError> {
    let mut command = Command::new(&request.shell);
    command
        .args(["-c", request.command.script()])
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);

    let mut child = command.spawn().map_err(DiscoveryError::Start)?;
    let stderr = child
        .stderr
        .take()
        .expect("piped discovery stderr is available");
    let stdout = child
        .stdout
        .take()
        .expect("piped discovery stdout is available");
    let remaining = Arc::new(AtomicUsize::new(request.max_output_bytes));
    let mut stderr_reader = tokio::spawn(read_bounded(stderr, remaining.clone()));
    let mut stdout_reader = tokio::spawn(read_bounded(stdout, remaining));

    let completed = async {
        let status = child.wait().await.map_err(DiscoveryError::Start)?;
        let stderr = (&mut stderr_reader)
            .await
            .map_err(|error| DiscoveryError::OutputRead(std::io::Error::other(error)))?
            .map_err(DiscoveryError::OutputRead)?;
        let stdout = (&mut stdout_reader)
            .await
            .map_err(|error| DiscoveryError::OutputRead(std::io::Error::other(error)))?
            .map_err(DiscoveryError::OutputRead)?;
        Ok::<_, DiscoveryError>((status, stderr, stdout))
    };
    let (status, stderr, stdout) = match tokio::time::timeout(request.timeout, completed).await {
        Ok(completed) => completed?,
        Err(_) => {
            terminate(&mut child).await;
            stderr_reader.abort();
            stdout_reader.abort();
            return Err(DiscoveryError::Timeout);
        }
    };

    if stderr.exceeded || stdout.exceeded {
        return Err(DiscoveryError::OutputLimit);
    }
    if !status.success() {
        return Err(DiscoveryError::NonZeroExit {
            detail: bounded_stderr_detail(&stderr.bytes),
            status: status
                .code()
                .map_or_else(|| "unknown".to_owned(), |code| code.to_string()),
        });
    }
    Ok(stdout.bytes)
}

fn bounded_stderr_detail(stderr: &[u8]) -> Option<String> {
    let sanitized = String::from_utf8_lossy(stderr)
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .collect::<String>();
    let sanitized = sanitized.trim();
    if sanitized.is_empty() {
        return None;
    }

    let character_count = sanitized.chars().count();
    if character_count <= MAX_ERROR_DETAIL_CHARACTERS {
        return Some(sanitized.to_owned());
    }

    let retained = sanitized
        .chars()
        .skip(character_count - MAX_ERROR_DETAIL_CHARACTERS)
        .collect::<String>();
    Some(format!("…\n{}", retained.trim_start()))
}

fn dashboard_actors(
    value: &Value,
    item: usize,
) -> Result<Vec<DashboardActor>, ItemValidationError> {
    value
        .as_array()
        .ok_or_else(|| item_error(item, "approvedBy must be an array"))?
        .iter()
        .map(|actor| {
            let login = actor_login(actor, item)?
                .ok_or_else(|| item_error(item, "an approver cannot be null"))?
                .to_owned();
            let url = match actor {
                Value::Object(actor) => match actor.get("url") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(url)) => {
                        let url = nonblank(url, item, "approver URL")?;
                        validate_https_url(url, item, "approver URL")?;
                        Some(url.to_owned())
                    }
                    Some(_) => {
                        return Err(item_error(item, "approver URL must be a string"));
                    }
                },
                _ => None,
            };
            Ok(DashboardActor { login, url })
        })
        .collect()
}

fn item_error(item: usize, message: impl Into<String>) -> ItemValidationError {
    ItemValidationError {
        item: item + 1,
        message: message.into(),
    }
}

fn labels(value: &Value, item: usize) -> Result<Vec<DashboardLabel>, ItemValidationError> {
    value
        .as_array()
        .ok_or_else(|| item_error(item, "labels must be an array"))?
        .iter()
        .map(|label| {
            if let Some(name) = label.as_str() {
                return Ok(DashboardLabel {
                    color: None,
                    name: nonblank(name, item, "label name")?.to_owned(),
                });
            }
            let label = label
                .as_object()
                .ok_or_else(|| item_error(item, "each label must be an object or string"))?;
            let name = required_string(label, "name", item)?;
            let color = match label.get("color") {
                None | Some(Value::Null) => None,
                Some(Value::String(color)) => {
                    Some(nonblank(color, item, "label color")?.to_owned())
                }
                Some(_) => return Err(item_error(item, "label color must be a string")),
            };
            Ok(DashboardLabel {
                color,
                name: name.to_owned(),
            })
        })
        .collect()
}

fn logins(value: &Value, item: usize) -> Result<Vec<String>, ItemValidationError> {
    value
        .as_array()
        .ok_or_else(|| item_error(item, "assignees must be an array"))?
        .iter()
        .map(|actor| {
            actor_login(actor, item)?
                .map(str::to_owned)
                .ok_or_else(|| item_error(item, "an assignee cannot be null"))
        })
        .collect()
}

pub fn normalize_items(
    output: &[u8],
    item_kind: ItemKind,
) -> Result<Vec<DashboardItem>, ItemValidationError> {
    let values = serde_json::from_slice::<Value>(output)
        .map_err(|_| item_error(0, "output must be a JSON array"))?;
    values
        .as_array()
        .ok_or_else(|| item_error(0, "output must be a JSON array"))?
        .iter()
        .enumerate()
        .map(|(index, value)| normalize_item(value, index, item_kind))
        .collect()
}

fn normalize_item(
    value: &Value,
    index: usize,
    item_kind: ItemKind,
) -> Result<DashboardItem, ItemValidationError> {
    let item = value
        .as_object()
        .ok_or_else(|| item_error(index, "each item must be an object"))?;
    let approved_by = item
        .get("approvedBy")
        .map_or_else(|| Ok(Vec::new()), |value| dashboard_actors(value, index))?;
    let assignees = logins(required(item, "assignees", index)?, index)?;
    let author = actor_login(required(item, "author", index)?, index)?.map(str::to_owned);
    let is_draft = match item_kind {
        ItemKind::Issue => None,
        ItemKind::PullRequest => Some(
            required(item, "isDraft", index)?
                .as_bool()
                .ok_or_else(|| item_error(index, "isDraft must be a boolean"))?,
        ),
    };
    let labels = labels(required(item, "labels", index)?, index)?;
    let number = required(item, "number", index)?
        .as_u64()
        .filter(|number| *number > 0)
        .ok_or_else(|| item_error(index, "number must be a positive integer"))?;
    let repository = repository(required(item, "repository", index)?, index)?.to_owned();
    let source = match item.get("source") {
        None | Some(Value::Null) => None,
        Some(Value::String(source)) => Some(nonblank(source, index, "source")?.to_owned()),
        Some(_) => return Err(item_error(index, "source must be a string")),
    };
    let state = required_string(item, "state", index)?.to_owned();
    let title = required_string(item, "title", index)?.to_owned();
    let updated_at = required_string(item, "updatedAt", index)?;
    OffsetDateTime::parse(updated_at, &Rfc3339)
        .map_err(|_| item_error(index, "updatedAt must be an RFC 3339 timestamp"))?;
    let url = required_string(item, "url", index)?;
    validate_https_url(url, index, "url")?;

    Ok(DashboardItem {
        advanced_buttons: Vec::new(),
        approved_by,
        assignees,
        always_buttons: Vec::new(),
        author,
        is_draft,
        item_kind,
        labels,
        number,
        repository,
        source,
        state,
        title,
        updated_at: updated_at.to_owned(),
        url: url.to_owned(),
    })
}

fn nonblank<'a>(value: &'a str, item: usize, field: &str) -> Result<&'a str, ItemValidationError> {
    if value.trim().is_empty() {
        return Err(item_error(item, format!("{field} cannot be blank")));
    }
    Ok(value)
}

async fn read_bounded(
    mut reader: impl AsyncRead + Unpin,
    remaining: Arc<AtomicUsize>,
) -> Result<BoundedOutput, std::io::Error> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; OUTPUT_CHUNK_BYTES];
    let mut exceeded = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let reserved = reserve_bytes(&remaining, read);
        bytes.extend_from_slice(&buffer[..reserved]);
        exceeded |= reserved < read;
    }
    Ok(BoundedOutput { bytes, exceeded })
}

fn repository(value: &Value, item: usize) -> Result<&str, ItemValidationError> {
    let repository = match value {
        Value::String(repository) => repository.as_str(),
        Value::Object(repository) => required_string(repository, "nameWithOwner", item)?,
        _ => return Err(item_error(item, "repository must identify an owner/name")),
    };
    let mut parts = repository.split('/');
    if parts.next().is_none_or(str::is_empty)
        || parts.next().is_none_or(str::is_empty)
        || parts.next().is_some()
    {
        return Err(item_error(item, "repository must identify an owner/name"));
    }
    Ok(repository)
}

fn required<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    item: usize,
) -> Result<&'a Value, ItemValidationError> {
    object
        .get(field)
        .ok_or_else(|| item_error(item, format!("{field} is required")))
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    item: usize,
) -> Result<&'a str, ItemValidationError> {
    let value = required(object, field, item)?
        .as_str()
        .ok_or_else(|| item_error(item, format!("{field} must be a string")))?;
    nonblank(value, item, field)
}

fn validate_https_url(value: &str, item: usize, field: &str) -> Result<(), ItemValidationError> {
    let parsed =
        Url::parse(value).map_err(|_| item_error(item, format!("{field} must be valid HTTPS")))?;
    if parsed.scheme() != "https" {
        return Err(item_error(item, format!("{field} must be valid HTTPS")));
    }
    Ok(())
}

fn reserve_bytes(remaining: &AtomicUsize, requested: usize) -> usize {
    let mut available = remaining.load(Ordering::Acquire);
    loop {
        let reserved = available.min(requested);
        match remaining.compare_exchange_weak(
            available,
            available - reserved,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return reserved,
            Err(current) => available = current,
        }
    }
}

async fn terminate(child: &mut Child) {
    #[cfg(unix)]
    if let Some(process_id) = child.id() {
        // Bash pipelines can outlive the shell unless the complete process group is terminated.
        unsafe {
            libc::kill(-(process_id as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

fn actor_login(value: &Value, item: usize) -> Result<Option<&str>, ItemValidationError> {
    match value {
        Value::Null => Ok(None),
        Value::String(login) => nonblank(login, item, "actor login").map(Some),
        Value::Object(actor) => required_string(actor, "login", item).map(Some),
        _ => Err(item_error(item, "actor must contain a login")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    use tempfile::TempDir;

    use super::{
        DiscoveryRequest, ItemDiscoverer, MAX_ERROR_DETAIL_CHARACTERS, bounded_stderr_detail,
        normalize_items,
    };
    use crate::commands::CommandTemplate;
    use crate::config::ItemKind;

    const PULL_REQUESTS: &[u8] = include_bytes!("fixtures/search-pull-requests.json");

    #[test]
    fn bounds_and_filters_stderr_details() {
        let stderr = format!(
            "discarded\0{}\u{7}",
            "x".repeat(MAX_ERROR_DETAIL_CHARACTERS + 1)
        );

        let detail = bounded_stderr_detail(stderr.as_bytes()).unwrap();

        assert!(detail.starts_with("…\n"));
        assert_eq!(detail.chars().count(), MAX_ERROR_DETAIL_CHARACTERS + 2);
        assert!(
            detail
                .chars()
                .all(|character| { !character.is_control() || matches!(character, '\n' | '\t') })
        );
    }

    #[tokio::test]
    async fn caches_successful_discoveries_by_command_until_the_ttl_expires() {
        let directory = workspace_tempdir();
        let count_path = directory.path().join("count");
        let fixture_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/search-pull-requests.json");
        let shell_path = directory.path().join("fake-shell");
        fs::write(
            &shell_path,
            format!(
                "#!/bin/sh\nprintf x >> '{}'\n/bin/cat '{}'\n",
                count_path.display(),
                fixture_path.display(),
            ),
        )
        .unwrap();
        make_executable(&shell_path);

        let discoverer = ItemDiscoverer::default();
        let request = DiscoveryRequest {
            cache_ttl: Duration::from_millis(40),
            command: CommandTemplate::compile("first query", &HashSet::new()).unwrap(),
            item_kind: ItemKind::PullRequest,
            max_output_bytes: 65_536,
            shell: shell_path,
            timeout: Duration::from_secs(2),
        };

        let first = discoverer.discover(request.clone()).await.unwrap();
        let cached = discoverer.discover(request.clone()).await.unwrap();
        assert_eq!(fs::read(&count_path).unwrap(), b"x");
        assert_eq!(cached.refreshed_at, first.refreshed_at);

        tokio::time::sleep(Duration::from_millis(80)).await;
        let refreshed = discoverer.discover(request.clone()).await.unwrap();
        assert_eq!(fs::read(&count_path).unwrap(), b"xx");
        assert!(refreshed.refreshed_at > first.refreshed_at);

        let mut changed_request = request;
        changed_request.command =
            CommandTemplate::compile("changed query", &HashSet::new()).unwrap();
        discoverer.discover(changed_request).await.unwrap();
        assert_eq!(fs::read(&count_path).unwrap(), b"xxx");
    }

    #[test]
    fn normalizes_checked_in_gh_search_output() {
        let items = normalize_items(PULL_REQUESTS, ItemKind::PullRequest).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].repository, "example/project");
        assert_eq!(items[0].number, 42);
        assert_eq!(items[0].author.as_deref(), Some("octocat"));
        assert_eq!(items[0].approved_by[0].login, "approver");
        assert_eq!(
            items[0].approved_by[0].url.as_deref(),
            Some("https://github.com/approver")
        );
        assert_eq!(items[0].assignees, ["reviewer"]);
        assert_eq!(items[0].labels[0].name, "reviewed");
        assert_eq!(items[0].is_draft, Some(false));
        assert_eq!(items[0].source, None);
        assert_eq!(items[1].author, None);
        assert!(items[1].approved_by.is_empty());
    }

    #[test]
    fn rejects_output_that_does_not_match_the_selected_item_kind() {
        let issue_without_draft = br#"[{"assignees":[],"author":{"login":"octocat"},"labels":[],"number":1,"repository":{"nameWithOwner":"example/project"},"state":"open","title":"Issue","updatedAt":"2026-08-25T12:00:00Z","url":"https://github.com/example/project/issues/1"}]"#;

        let error = normalize_items(issue_without_draft, ItemKind::PullRequest).unwrap_err();

        assert_eq!(
            error.to_string(),
            "The section command returned invalid item 1: isDraft is required."
        );
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) {
        panic!("Personal Dashboard requires Bash and a Unix process model");
    }

    fn workspace_tempdir() -> TempDir {
        tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap()
    }
}
