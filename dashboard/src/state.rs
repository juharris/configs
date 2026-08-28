use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock, Weak};
use std::time::Duration;

use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};

use crate::config::{ConfigService, ConfigurationSnapshot};
use crate::items::{DiscoveryError, DiscoveryRequest, DiscoveryResult, ItemDiscoverer};
use crate::messages::{
    DashboardItem, DashboardSnapshot, ItemReference, SectionRefresh, SectionRefreshStatus,
    SectionSnapshot,
};

/// Exposes immutable dashboard snapshots and queues typed section refresh requests.
pub struct DashboardService {
    command_sender: mpsc::UnboundedSender<DashboardCommand>,
    snapshot: RwLock<Option<DashboardSnapshot>>,
    snapshot_sender: watch::Sender<Option<DashboardSnapshot>>,
}

impl DashboardService {
    pub fn new(config_service: Arc<ConfigService>) -> (Arc<Self>, DashboardRuntime) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        let (snapshot_sender, _) = watch::channel(None);
        let service = Arc::new(Self {
            command_sender: command_sender.clone(),
            snapshot: RwLock::new(None),
            snapshot_sender,
        });
        let runtime = DashboardRuntime {
            command_receiver,
            command_sender,
            configuration_receiver: config_service.subscribe(),
            service: Arc::downgrade(&service),
        };
        (service, runtime)
    }

    pub async fn refresh_section(
        &self,
        configuration_revision: u64,
        section_id: String,
    ) -> Result<SectionRefresh, DashboardServiceError> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.command_sender
            .send(DashboardCommand::ManualRefresh {
                configuration_revision,
                response_sender,
                section_id,
            })
            .map_err(|_| DashboardServiceError::Unavailable)?;
        response_receiver
            .await
            .map_err(|_| DashboardServiceError::Unavailable)?
    }

    pub fn item(
        &self,
        configuration_revision: u64,
        section_id: &str,
        reference: &ItemReference,
    ) -> Result<DashboardItem, DashboardServiceError> {
        let snapshot = self
            .snapshot
            .read()
            .map_err(|_| DashboardServiceError::Lock)?;
        let dashboard = snapshot
            .as_ref()
            .ok_or(DashboardServiceError::Unavailable)?;
        if dashboard.configuration_revision != configuration_revision {
            return Err(DashboardServiceError::ConfigurationChanged {
                active: dashboard.configuration_revision,
                requested: configuration_revision,
            });
        }
        let section = dashboard
            .sections
            .iter()
            .find(|section| section.id == section_id)
            .ok_or_else(|| DashboardServiceError::InvalidSection(section_id.to_owned()))?;
        section
            .items
            .iter()
            .find(|item| {
                item.number == reference.number
                    && item.repository == reference.repository
                    && item.source == reference.source
            })
            .cloned()
            .ok_or(DashboardServiceError::InvalidItem)
    }

    pub fn snapshot(&self) -> Result<Option<DashboardSnapshot>, DashboardServiceError> {
        self.snapshot
            .read()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| DashboardServiceError::Lock)
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<DashboardSnapshot>> {
        self.snapshot_sender.subscribe()
    }

    fn publish(&self, snapshot: DashboardSnapshot) -> Result<(), DashboardServiceError> {
        *self
            .snapshot
            .write()
            .map_err(|_| DashboardServiceError::Lock)? = Some(snapshot.clone());
        self.snapshot_sender.send_replace(Some(snapshot));
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DashboardServiceError {
    #[error(
        "configuration revision changed from requested revision {requested} to active revision {active}"
    )]
    ConfigurationChanged { active: u64, requested: u64 },
    #[error("section {0} is not configured")]
    InvalidSection(String),
    #[error("the requested dashboard item is not available")]
    InvalidItem,
    #[error("dashboard state lock was poisoned")]
    Lock,
    #[error("dashboard state service is not available")]
    Unavailable,
}

pub struct DashboardRuntime {
    command_receiver: mpsc::UnboundedReceiver<DashboardCommand>,
    command_sender: mpsc::UnboundedSender<DashboardCommand>,
    configuration_receiver: watch::Receiver<Option<ConfigurationSnapshot>>,
    service: Weak<DashboardService>,
}

impl DashboardRuntime {
    pub async fn run(mut self) {
        let mut state = RuntimeState::new(self.command_sender.clone());
        let initial_configuration = self.configuration_receiver.borrow_and_update().clone();
        if let Some(configuration) = initial_configuration {
            state.accept_configuration(configuration);
            if !self.publish(&state) {
                return;
            }
        }

        loop {
            tokio::select! {
                configuration = self.configuration_receiver.changed() => {
                    if configuration.is_err() {
                        return;
                    }
                    let configuration = self.configuration_receiver.borrow_and_update().clone();
                    if let Some(configuration) = configuration {
                        state.accept_configuration(configuration);
                        if !self.publish(&state) {
                            return;
                        }
                    }
                }
                command = self.command_receiver.recv() => {
                    let Some(command) = command else {
                        return;
                    };
                    if state.handle(command) && !self.publish(&state) {
                        return;
                    }
                }
            }
        }
    }

    fn publish(&self, state: &RuntimeState) -> bool {
        let Some(service) = self.service.upgrade() else {
            return false;
        };
        let Some(dashboard) = state.dashboard.clone() else {
            return true;
        };
        if let Err(error) = service.publish(dashboard) {
            tracing::error!(%error, "could not publish dashboard state");
            return false;
        }
        true
    }
}

enum DashboardCommand {
    Completed {
        configuration_revision: u64,
        result: Result<DiscoveryResult, DiscoveryError>,
        section_id: String,
    },
    ManualRefresh {
        configuration_revision: u64,
        response_sender: oneshot::Sender<Result<SectionRefresh, DashboardServiceError>>,
        section_id: String,
    },
}

struct RuntimeState {
    command_sender: mpsc::UnboundedSender<DashboardCommand>,
    configuration: Option<ConfigurationSnapshot>,
    dashboard: Option<DashboardSnapshot>,
    item_discoverer: ItemDiscoverer,
    pending: VecDeque<String>,
    pending_ids: HashSet<String>,
    running: HashMap<String, u64>,
}

impl RuntimeState {
    fn new(command_sender: mpsc::UnboundedSender<DashboardCommand>) -> Self {
        Self {
            command_sender,
            configuration: None,
            dashboard: None,
            item_discoverer: ItemDiscoverer::default(),
            pending: VecDeque::new(),
            pending_ids: HashSet::new(),
            running: HashMap::new(),
        }
    }

    fn accept_configuration(&mut self, configuration: ConfigurationSnapshot) {
        let previous = self.preserved_sections(&configuration);
        self.pending.clear();
        self.pending_ids.clear();

        let sections = configuration
            .configuration
            .root
            .sections
            .iter()
            .map(|section| {
                let preserved = previous.get(&section.id);
                SectionSnapshot {
                    collapsed: section.collapsed,
                    error: None,
                    id: section.id.clone(),
                    items: preserved.map_or_else(Vec::new, |snapshot| snapshot.items.clone()),
                    items_per_page: section.items_per_page,
                    last_successful_refresh: preserved
                        .and_then(|snapshot| snapshot.last_successful_refresh),
                    refresh_seconds: section.refresh_seconds.unwrap_or(
                        configuration
                            .configuration
                            .root
                            .application
                            .default_refresh_seconds,
                    ),
                    stale: preserved
                        .and_then(|snapshot| snapshot.last_successful_refresh)
                        .is_some(),
                    status: SectionRefreshStatus::Idle,
                    title: section.title.clone(),
                }
            })
            .collect();
        self.dashboard = Some(DashboardSnapshot {
            configuration_revision: configuration.revision,
            sections,
        });
        self.configuration = Some(configuration.clone());
    }

    fn complete(
        &mut self,
        configuration_revision: u64,
        result: Result<DiscoveryResult, DiscoveryError>,
        section_id: String,
    ) {
        self.running.remove(&section_id);
        let current_revision = self
            .configuration
            .as_ref()
            .map(|configuration| configuration.revision);
        let result = if current_revision == Some(configuration_revision) {
            self.decorate_items(result)
        } else {
            result
        };
        if current_revision == Some(configuration_revision)
            && let Some(section) = self.section_mut(&section_id)
        {
            section.status = SectionRefreshStatus::Idle;
            match result {
                Ok(discovery) => {
                    section.error = None;
                    section.items = discovery.items;
                    section.last_successful_refresh = Some(discovery.refreshed_at);
                    section.stale = false;
                }
                Err(error) => {
                    tracing::warn!(
                        category = error.category(),
                        %section_id,
                        "section discovery failed"
                    );
                    section.error = Some(error.safe_message());
                    section.stale = true;
                }
            }
        }
        self.start_available();
    }

    fn enqueue(&mut self, section_id: &str) -> bool {
        if self.running.contains_key(section_id) || self.pending_ids.contains(section_id) {
            return true;
        }
        self.pending.push_back(section_id.to_owned());
        self.pending_ids.insert(section_id.to_owned());
        if let Some(section) = self.section_mut(section_id) {
            section.status = SectionRefreshStatus::Queued;
        }
        false
    }

    fn handle(&mut self, command: DashboardCommand) -> bool {
        match command {
            DashboardCommand::Completed {
                configuration_revision,
                result,
                section_id,
            } => {
                self.complete(configuration_revision, result, section_id);
                true
            }
            DashboardCommand::ManualRefresh {
                configuration_revision,
                response_sender,
                section_id,
            } => {
                let result = self.request_refresh(configuration_revision, section_id);
                let changed = result.as_ref().is_ok_and(|refresh| !refresh.coalesced);
                let _ = response_sender.send(result);
                changed
            }
        }
    }

    fn has_section(&self, section_id: &str) -> bool {
        self.dashboard.as_ref().is_some_and(|dashboard| {
            dashboard
                .sections
                .iter()
                .any(|section| section.id == section_id)
        })
    }

    fn preserved_sections(
        &self,
        configuration: &ConfigurationSnapshot,
    ) -> HashMap<String, SectionSnapshot> {
        let Some(previous_configuration) = &self.configuration else {
            return HashMap::new();
        };
        let Some(previous_dashboard) = &self.dashboard else {
            return HashMap::new();
        };
        let previous_kinds = previous_configuration
            .configuration
            .root
            .sections
            .iter()
            .map(|section| (section.id.as_str(), section.item_kind))
            .collect::<HashMap<_, _>>();
        let current_kinds = configuration
            .configuration
            .root
            .sections
            .iter()
            .map(|section| (section.id.as_str(), section.item_kind))
            .collect::<HashMap<_, _>>();
        previous_dashboard
            .sections
            .iter()
            .filter(|section| {
                previous_kinds.get(section.id.as_str()) == current_kinds.get(section.id.as_str())
            })
            .map(|section| (section.id.clone(), section.clone()))
            .collect()
    }

    fn request_refresh(
        &mut self,
        configuration_revision: u64,
        section_id: String,
    ) -> Result<SectionRefresh, DashboardServiceError> {
        let active_revision = self
            .active_revision()
            .ok_or(DashboardServiceError::Unavailable)?;
        if active_revision != configuration_revision {
            return Err(DashboardServiceError::ConfigurationChanged {
                active: active_revision,
                requested: configuration_revision,
            });
        }
        if !self.has_section(&section_id) {
            return Err(DashboardServiceError::InvalidSection(section_id));
        }

        let coalesced = self.enqueue(&section_id);
        self.start_available();
        let status = self
            .section(&section_id)
            .expect("the requested section was validated")
            .status;
        Ok(SectionRefresh {
            coalesced,
            section_id,
            status,
        })
    }

    fn decorate_items(
        &self,
        result: Result<DiscoveryResult, DiscoveryError>,
    ) -> Result<DiscoveryResult, DiscoveryError> {
        let configuration = self
            .configuration
            .as_ref()
            .expect("an active revision has an active configuration");
        let mut discovery = result?;
        for item in &mut discovery.items {
            crate::buttons::decorate_item(configuration, item);
        }
        Ok(discovery)
    }

    fn section(&self, section_id: &str) -> Option<&SectionSnapshot> {
        self.dashboard
            .as_ref()?
            .sections
            .iter()
            .find(|section| section.id == section_id)
    }

    fn section_mut(&mut self, section_id: &str) -> Option<&mut SectionSnapshot> {
        self.dashboard
            .as_mut()?
            .sections
            .iter_mut()
            .find(|section| section.id == section_id)
    }

    fn start_available(&mut self) {
        let Some(configuration) = self.configuration.clone() else {
            return;
        };
        let limit = configuration
            .configuration
            .root
            .application
            .max_concurrent_commands;
        while self.running.len() < limit {
            let Some(position) = self
                .pending
                .iter()
                .position(|section_id| !self.running.contains_key(section_id))
            else {
                return;
            };
            let section_id = self
                .pending
                .remove(position)
                .expect("the pending position exists");
            self.pending_ids.remove(&section_id);
            let Some(section_index) = configuration
                .configuration
                .root
                .sections
                .iter()
                .position(|section| section.id == section_id)
            else {
                continue;
            };
            if let Some(section) = self.section_mut(&section_id) {
                section.status = SectionRefreshStatus::Refreshing;
            }
            self.running
                .insert(section_id.clone(), configuration.revision);

            let request = DiscoveryRequest {
                cache_ttl: Duration::from_secs(
                    configuration.configuration.root.sections[section_index].cache_ttl_seconds,
                ),
                command: configuration.configuration.sections[section_index]
                    .command
                    .clone(),
                item_kind: configuration.configuration.root.sections[section_index].item_kind,
                max_output_bytes: configuration
                    .configuration
                    .root
                    .application
                    .max_output_bytes_per_run,
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
            };
            let command_sender = self.command_sender.clone();
            let configuration_revision = configuration.revision;
            let item_discoverer = self.item_discoverer.clone();
            tokio::spawn(async move {
                let result = item_discoverer.discover(request).await;
                let _ = command_sender.send(DashboardCommand::Completed {
                    configuration_revision,
                    result,
                    section_id,
                });
            });
        }
    }

    fn active_revision(&self) -> Option<u64> {
        self.configuration
            .as_ref()
            .map(|configuration| configuration.revision)
    }
}

trait DiscoveryErrorCategory {
    fn category(&self) -> &'static str;
}

impl DiscoveryErrorCategory for DiscoveryError {
    fn category(&self) -> &'static str {
        match self {
            DiscoveryError::InvalidItems(_) => "invalid_items",
            DiscoveryError::NonZeroExit { .. } => "non_zero_exit",
            DiscoveryError::OutputLimit => "output_limit",
            DiscoveryError::OutputRead(_) => "output_read",
            DiscoveryError::Start(_) => "start",
            DiscoveryError::Timeout => "timeout",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    use serde_json::json;
    use tempfile::TempDir;
    use tokio::sync::watch;

    use super::DashboardService;
    use crate::config::{ConfigService, RuntimeSchema};
    use crate::messages::{DashboardSnapshot, OptifySetup, SectionRefreshStatus};

    #[tokio::test]
    async fn coalesces_refreshes_and_preserves_stale_items_after_failure() {
        let fixture = DiscoveryFixture::new();
        let (config_service, _config_runtime) =
            ConfigService::new(RuntimeSchema::materialize().unwrap());
        let (dashboard_service, dashboard_runtime) = DashboardService::new(config_service.clone());
        tokio::spawn(dashboard_runtime.run());
        let mut snapshots = dashboard_service.subscribe();

        let configuration = config_service.apply_setup(fixture.setup()).unwrap();
        let idle = wait_for_snapshot(&mut snapshots, |snapshot| {
            snapshot.sections[0].status == SectionRefreshStatus::Idle
        })
        .await;
        assert!(idle.sections[0].items.is_empty());
        assert!(!idle.sections[0].collapsed);
        assert_eq!(idle.sections[0].refresh_seconds, 300);

        let refresh = dashboard_service
            .refresh_section(configuration.revision, "reviews".to_owned())
            .await
            .unwrap();
        assert!(!refresh.coalesced);
        let refreshing = wait_for_snapshot(&mut snapshots, |snapshot| {
            snapshot.sections[0].status == SectionRefreshStatus::Refreshing
        })
        .await;
        assert!(refreshing.sections[0].items.is_empty());

        let coalesced = dashboard_service
            .refresh_section(configuration.revision, "reviews".to_owned())
            .await
            .unwrap();
        assert!(coalesced.coalesced);

        fixture.release();
        let current = wait_for_snapshot(&mut snapshots, |snapshot| {
            snapshot.sections[0].status == SectionRefreshStatus::Idle
                && snapshot.sections[0].last_successful_refresh.is_some()
        })
        .await;
        assert_eq!(current.sections[0].items.len(), 2);
        assert!(
            current.sections[0]
                .items
                .iter()
                .all(|item| !item.always_buttons[0].disabled)
        );
        assert!(!current.sections[0].stale);
        let last_successful_refresh = current.sections[0].last_successful_refresh;

        let refresh = dashboard_service
            .refresh_section(configuration.revision, "reviews".to_owned())
            .await
            .unwrap();
        assert!(!refresh.coalesced);
        snapshots.changed().await.unwrap();
        let cached = wait_for_snapshot(&mut snapshots, |snapshot| {
            snapshot.sections[0].status == SectionRefreshStatus::Idle
        })
        .await;
        assert_eq!(
            cached.sections[0].last_successful_refresh,
            last_successful_refresh
        );

        tokio::time::sleep(Duration::from_secs(1)).await;
        fixture.fail();
        let refresh = dashboard_service
            .refresh_section(configuration.revision, "reviews".to_owned())
            .await
            .unwrap();
        assert!(!refresh.coalesced);
        let stale = wait_for_snapshot(&mut snapshots, |snapshot| {
            snapshot.sections[0].status == SectionRefreshStatus::Idle && snapshot.sections[0].stale
        })
        .await;
        assert_eq!(stale.sections[0].items.len(), 2);
        assert_eq!(
            stale.sections[0].error.as_deref(),
            Some("The section command exited with status 7.\nFixture failure detail.")
        );
    }

    struct DiscoveryFixture {
        configuration: TempDir,
        process: TempDir,
    }

    impl DiscoveryFixture {
        fn new() -> Self {
            let configuration = workspace_tempdir();
            let process = workspace_tempdir();
            let fail_path = process.path().join("fail");
            let release_path = process.path().join("release");
            let shell_path = process.path().join("fake-shell");
            let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/fixtures/search-pull-requests.json");
            fs::write(
                &shell_path,
                format!(
                    "#!/bin/sh\nif [ \"$1\" = \"-n\" ]; then exit 0; fi\nwhile [ ! -e '{}' ]; do /bin/sleep 0.01; done\nif [ -e '{}' ]; then echo 'Fixture failure detail.' >&2; exit 7; fi\n/bin/cat '{}'\n",
                    release_path.display(),
                    fail_path.display(),
                    fixture_path.display(),
                ),
            )
            .unwrap();
            make_executable(&shell_path);
            fs::write(
                configuration.path().join("dashboard.json"),
                serde_json::to_vec_pretty(&json!({
                    "options": {
                        "appearance": { "theme": "dark" },
                        "application": {
                            "command_timeout_seconds": 5,
                            "default_refresh_seconds": 300,
                            "max_concurrent_commands": 1,
                            "max_output_bytes_per_run": 65536,
                            "shell": shell_path,
                        },
                        "autocomplete": {
                            "command": "ignored {autocomplete.request}",
                            "debounce_milliseconds": 100,
                            "instruction": "Complete the draft.",
                            "minimum_characters": 1,
                        },
                        "buttons": {
                            "issues": { "advanced": [], "always": [] },
                            "pull_requests": {
                                "advanced": [],
                                "always": [{
                                    "command": "cd; review {item.url}",
                                    "label": "Review",
                                }],
                            },
                        },
                        "sections": [{
                            "cache_ttl_seconds": 1,
                            "command": "ignored",
                            "id": "reviews",
                            "item_kind": "pull_request",
                            "items_per_page": 6,
                            "title": "Reviews",
                        }],
                    },
                }))
                .unwrap(),
            )
            .unwrap();
            Self {
                configuration,
                process,
            }
        }

        fn fail(&self) {
            fs::write(self.process.path().join("fail"), []).unwrap();
        }

        fn release(&self) {
            fs::write(self.process.path().join("release"), []).unwrap();
        }

        fn setup(&self) -> OptifySetup {
            OptifySetup {
                config_directories: vec![self.configuration.path().display().to_string()],
                features: vec!["dashboard".to_owned()],
            }
        }
    }

    async fn wait_for_snapshot(
        snapshots: &mut watch::Receiver<Option<DashboardSnapshot>>,
        predicate: impl Fn(&DashboardSnapshot) -> bool,
    ) -> DashboardSnapshot {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(snapshot) = snapshots.borrow_and_update().clone()
                    && predicate(&snapshot)
                {
                    return snapshot;
                }
                snapshots.changed().await.unwrap();
            }
        })
        .await
        .expect("dashboard state was not published")
    }

    fn workspace_tempdir() -> TempDir {
        tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap()
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
}
