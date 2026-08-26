use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;

use optify::provider::{OptionsRegistry, OptionsWatcher, OptionsWatcherListener, WatcherOptions};
use serde::Serialize;
use thiserror::Error;
use tokio::sync::{mpsc, watch};

use super::{RootConfig, RuntimeSchema, ValidatedRootConfig};
use crate::messages::{ActiveConfiguration, OptifySetup};

const WATCHER_DEBOUNCE: Duration = Duration::from_millis(250);
const RELOAD_COALESCE: Duration = Duration::from_millis(75);

#[derive(Clone, Debug)]
pub struct ConfigurationSnapshot {
    pub configuration: Arc<ValidatedRootConfig>,
    pub revision: u64,
    pub setup: OptifySetup,
}

impl ConfigurationSnapshot {
    pub fn transport(&self) -> ActiveConfiguration {
        ActiveConfiguration {
            revision: self.revision,
            setup: self.setup.clone(),
            theme: self.configuration.root.appearance.theme,
        }
    }
}

struct ActiveState {
    generation: u64,
    snapshot: ConfigurationSnapshot,
    watcher: Arc<OptionsWatcher>,
}

/// Owns the active Optify watcher and publishes only fully validated snapshots.
pub struct ConfigService {
    active: RwLock<Option<Arc<ActiveState>>>,
    apply_lock: Mutex<()>,
    generation: AtomicU64,
    reload_sender: mpsc::UnboundedSender<ReloadRequest>,
    runtime_schema: RuntimeSchema,
    snapshot_sender: watch::Sender<Option<ConfigurationSnapshot>>,
}

impl ConfigService {
    pub fn new(runtime_schema: RuntimeSchema) -> (Arc<Self>, ConfigReloadService) {
        let (reload_sender, reload_receiver) = mpsc::unbounded_channel();
        let (snapshot_sender, _) = watch::channel(None);
        let service = Arc::new(Self {
            active: RwLock::new(None),
            apply_lock: Mutex::new(()),
            generation: AtomicU64::new(0),
            reload_sender,
            runtime_schema,
            snapshot_sender,
        });
        let reload_service = ConfigReloadService {
            receiver: reload_receiver,
            service: Arc::downgrade(&service),
        };
        (service, reload_service)
    }

    pub fn apply_setup(
        &self,
        setup: OptifySetup,
    ) -> Result<ConfigurationSnapshot, ConfigServiceError> {
        let _apply_guard = self
            .apply_lock
            .lock()
            .map_err(|_| ConfigServiceError::Lock)?;
        if let Some(snapshot) = self.matching_snapshot(&setup)? {
            return Ok(snapshot);
        }

        let directories = validate_setup(&setup)?;
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let mut watcher = OptionsWatcher::build_from_directories_with_schema_and_options(
            &directories,
            self.runtime_schema.path(),
            WatcherOptions::new(WATCHER_DEBOUNCE),
        )
        .map_err(ConfigServiceError::Optify)?;

        watcher.add_listener(self.reload_listener(generation));

        let configuration = load_configuration(&watcher, &setup.features)?;
        let revision = self
            .active
            .read()
            .map_err(|_| ConfigServiceError::Lock)?
            .as_ref()
            .map_or(1, |active| active.snapshot.revision + 1);
        let snapshot = ConfigurationSnapshot {
            configuration: Arc::new(configuration),
            revision,
            setup,
        };
        let active = Arc::new(ActiveState {
            generation,
            snapshot: snapshot.clone(),
            watcher: Arc::new(watcher),
        });
        *self.active.write().map_err(|_| ConfigServiceError::Lock)? = Some(active);
        self.snapshot_sender.send_replace(Some(snapshot.clone()));
        Ok(snapshot)
    }

    pub fn snapshot(&self) -> Result<Option<ConfigurationSnapshot>, ConfigServiceError> {
        Ok(self
            .active
            .read()
            .map_err(|_| ConfigServiceError::Lock)?
            .as_ref()
            .map(|active| active.snapshot.clone()))
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<ConfigurationSnapshot>> {
        self.snapshot_sender.subscribe()
    }

    fn matching_snapshot(
        &self,
        setup: &OptifySetup,
    ) -> Result<Option<ConfigurationSnapshot>, ConfigServiceError> {
        Ok(self
            .active
            .read()
            .map_err(|_| ConfigServiceError::Lock)?
            .as_ref()
            .filter(|active| active.snapshot.setup == *setup)
            .map(|active| active.snapshot.clone()))
    }

    fn reload(&self, request: ReloadRequest) -> Result<(), ConfigServiceError> {
        let active = self
            .active
            .read()
            .map_err(|_| ConfigServiceError::Lock)?
            .clone();
        let Some(active) = active.filter(|active| active.generation == request.generation) else {
            return Ok(());
        };

        let configuration = load_configuration(&active.watcher, &active.snapshot.setup.features)?;
        let mut active_guard = self.active.write().map_err(|_| ConfigServiceError::Lock)?;
        let Some(current) = active_guard
            .as_ref()
            .filter(|current| current.generation == request.generation)
        else {
            return Ok(());
        };
        let snapshot = ConfigurationSnapshot {
            configuration: Arc::new(configuration),
            revision: current.snapshot.revision + 1,
            setup: current.snapshot.setup.clone(),
        };
        *active_guard = Some(Arc::new(ActiveState {
            generation: current.generation,
            snapshot: snapshot.clone(),
            watcher: current.watcher.clone(),
        }));
        self.snapshot_sender.send_replace(Some(snapshot));
        tracing::info!(
            changed_paths = ?request.paths,
            "accepted reloaded Optify configuration"
        );
        Ok(())
    }

    fn reload_listener(&self, generation: u64) -> OptionsWatcherListener {
        let reload_sender = self.reload_sender.clone();
        Arc::new(move |paths| {
            let _ = reload_sender.send(ReloadRequest {
                generation,
                paths: paths.clone(),
            });
        })
    }

    #[cfg(test)]
    fn simulate_successful_optify_rebuild(
        &self,
        changed_path: PathBuf,
    ) -> Result<(), ConfigServiceError> {
        let active = self
            .active
            .read()
            .map_err(|_| ConfigServiceError::Lock)?
            .clone()
            .expect("test setup is active");
        let directories = validate_setup(&active.snapshot.setup)?;
        let mut watcher = OptionsWatcher::build_from_directories_with_schema_and_options(
            &directories,
            self.runtime_schema.path(),
            WatcherOptions::new(WATCHER_DEBOUNCE),
        )
        .map_err(ConfigServiceError::Optify)?;
        let listener = self.reload_listener(active.generation);
        watcher.add_listener(listener.clone());
        *self.active.write().map_err(|_| ConfigServiceError::Lock)? = Some(Arc::new(ActiveState {
            generation: active.generation,
            snapshot: active.snapshot.clone(),
            watcher: Arc::new(watcher),
        }));
        listener(&HashSet::from([changed_path]));
        Ok(())
    }
}

pub struct ConfigReloadService {
    receiver: mpsc::UnboundedReceiver<ReloadRequest>,
    service: Weak<ConfigService>,
}

impl ConfigReloadService {
    pub async fn run(mut self) {
        while let Some(mut request) = self.receiver.recv().await {
            tokio::time::sleep(RELOAD_COALESCE).await;
            while let Ok(next) = self.receiver.try_recv() {
                if next.generation == request.generation {
                    request.paths.extend(next.paths);
                } else {
                    request = next;
                }
            }

            let Some(service) = self.service.upgrade() else {
                return;
            };
            if let Err(error) = service.reload(request) {
                tracing::error!(%error, "rejected reloaded Optify configuration");
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigServiceError {
    #[error("merged Optify options could not be deserialized: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("configuration service lock was poisoned")]
    Lock,
    #[error("Optify configuration could not be loaded: {0}")]
    Optify(String),
    #[error("invalid Optify setup at {field}: {message}")]
    Setup { field: String, message: String },
    #[error(transparent)]
    Validation(#[from] super::ConfigError),
}

#[derive(Clone, Debug)]
struct ReloadRequest {
    generation: u64,
    paths: HashSet<PathBuf>,
}

fn load_configuration(
    watcher: &OptionsWatcher,
    features: &[String],
) -> Result<ValidatedRootConfig, ConfigServiceError> {
    let options = watcher
        .get_all_options(features, None, None)
        .map_err(ConfigServiceError::Optify)?;
    let root = serde_json::from_value::<RootConfig>(options)?;
    ValidatedRootConfig::validate(root).map_err(ConfigServiceError::Validation)
}

fn validate_setup(setup: &OptifySetup) -> Result<Vec<PathBuf>, ConfigServiceError> {
    if setup.config_directories.is_empty() {
        return Err(setup_error(
            "configDirectories",
            "must contain at least one directory",
        ));
    }
    if setup.features.is_empty() {
        return Err(setup_error("features", "must contain at least one feature"));
    }

    let directories = setup
        .config_directories
        .iter()
        .enumerate()
        .map(|(index, directory)| {
            if directory.trim().is_empty() {
                return Err(setup_error(
                    format!("configDirectories[{index}]"),
                    "cannot be blank",
                ));
            }
            let path = Path::new(directory);
            if !path.is_absolute() {
                return Err(setup_error(
                    format!("configDirectories[{index}]"),
                    "must be an absolute path",
                ));
            }
            if !path.is_dir() {
                return Err(setup_error(
                    format!("configDirectories[{index}]"),
                    "must be a readable directory",
                ));
            }
            fs::read_dir(path).map_err(|error| {
                setup_error(
                    format!("configDirectories[{index}]"),
                    format!("cannot be read: {error}"),
                )
            })?;
            Ok(path.to_path_buf())
        })
        .collect::<Result<Vec<_>, _>>()?;

    for (index, feature) in setup.features.iter().enumerate() {
        if feature.trim().is_empty() {
            return Err(setup_error(format!("features[{index}]"), "cannot be blank"));
        }
    }
    Ok(directories)
}

fn setup_error(field: impl Into<String>, message: impl Into<String>) -> ConfigServiceError {
    ConfigServiceError::Setup {
        field: field.into(),
        message: message.into(),
    }
}

impl Serialize for ConfigurationSnapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.transport().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    use tempfile::TempDir;

    use super::{ConfigService, ConfigServiceError};
    use crate::config::RuntimeSchema;
    use crate::messages::{OptifySetup, Theme};

    #[test]
    fn applies_ordered_directories_and_features_idempotently_and_atomically() {
        let fixture = ConfigFixture::new();
        let (service, _reload_service) = ConfigService::new(RuntimeSchema::materialize().unwrap());
        let setup = fixture.setup();

        let first = service.apply_setup(setup.clone()).unwrap();
        let unchanged = service.apply_setup(setup).unwrap();

        assert_eq!(first.revision, 1);
        assert_eq!(unchanged.revision, first.revision);
        assert_eq!(unchanged.configuration.root.appearance.theme, Theme::Dark);
        assert_eq!(unchanged.configuration.root.sections.len(), 1);

        let invalid_setup = OptifySetup {
            config_directories: vec![fixture.base.path().display().to_string()],
            features: vec!["missing".to_owned()],
        };
        assert!(matches!(
            service.apply_setup(invalid_setup),
            Err(ConfigServiceError::Optify(_))
        ));
        assert_eq!(
            service.snapshot().unwrap().unwrap().revision,
            first.revision
        );

        fixture.write_invalid_template_override();
        let mut changed_setup = fixture.setup();
        changed_setup.config_directories.reverse();
        assert!(matches!(
            service.apply_setup(changed_setup),
            Err(ConfigServiceError::Validation(_))
        ));
        assert_eq!(
            service.snapshot().unwrap().unwrap().revision,
            first.revision
        );
    }

    #[test]
    fn loads_the_checked_in_root_with_recursive_focused_imports() {
        let (service, _reload_service) = ConfigService::new(RuntimeSchema::materialize().unwrap());
        let setup = OptifySetup {
            config_directories: vec![
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("configs")
                    .display()
                    .to_string(),
            ],
            features: vec!["dashboard".to_owned()],
        };

        let snapshot = service.apply_setup(setup).unwrap();

        assert_eq!(snapshot.configuration.root.appearance.theme, Theme::System);
        assert_eq!(snapshot.configuration.root.sections.len(), 3);
    }

    #[test]
    fn rejects_a_merged_root_with_missing_required_options() {
        let directory = workspace_tempdir();
        fs::write(
            directory.path().join("incomplete.yaml"),
            "options:\n  appearance:\n    theme: system\n",
        )
        .unwrap();
        let (service, _reload_service) = ConfigService::new(RuntimeSchema::materialize().unwrap());
        let setup = OptifySetup {
            config_directories: vec![directory.path().display().to_string()],
            features: vec!["incomplete".to_owned()],
        };

        assert!(matches!(
            service.apply_setup(setup),
            Err(ConfigServiceError::Deserialize(_))
        ));
        assert!(service.snapshot().unwrap().is_none());
    }

    #[tokio::test]
    async fn publishes_a_new_snapshot_after_a_valid_watched_change() {
        let fixture = ConfigFixture::new();
        let (service, reload_service) = ConfigService::new(RuntimeSchema::materialize().unwrap());
        tokio::spawn(reload_service.run());
        let first = service.apply_setup(fixture.setup()).unwrap();
        let mut snapshots = service.subscribe();

        fixture.write_override("light", "updated_items");
        service
            .simulate_successful_optify_rebuild(
                fixture.override_directory.path().join("override.yaml"),
            )
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), snapshots.changed())
            .await
            .unwrap()
            .unwrap();
        let reloaded = snapshots.borrow().clone().unwrap();

        assert_eq!(reloaded.revision, first.revision + 1);
        assert_eq!(reloaded.configuration.root.appearance.theme, Theme::Light);
        assert_eq!(reloaded.configuration.root.sections[0].id, "updated_items");
    }

    #[test]
    fn retains_the_previous_snapshot_when_a_changed_setup_fails_schema_rebuilding() {
        let fixture = ConfigFixture::new();
        let (service, _reload_service) = ConfigService::new(RuntimeSchema::materialize().unwrap());
        let first = service.apply_setup(fixture.setup()).unwrap();

        fs::write(
            fixture.override_directory.path().join("override.yaml"),
            "options:\n  unknown_field: true\n",
        )
        .unwrap();
        let mut changed_setup = fixture.setup();
        changed_setup.config_directories.reverse();

        assert!(matches!(
            service.apply_setup(changed_setup),
            Err(ConfigServiceError::Optify(_))
        ));
        assert_eq!(
            service.snapshot().unwrap().unwrap().revision,
            first.revision
        );
    }

    struct ConfigFixture {
        base: TempDir,
        override_directory: TempDir,
        repository: TempDir,
    }

    impl ConfigFixture {
        fn new() -> Self {
            let fixture = Self {
                base: workspace_tempdir(),
                override_directory: workspace_tempdir(),
                repository: workspace_tempdir(),
            };
            fs::write(
                fixture.base.path().join("base.yaml"),
                base_configuration(fixture.repository.path()),
            )
            .unwrap();
            fixture.write_override("dark", "overridden_items");
            fixture
        }

        fn setup(&self) -> OptifySetup {
            OptifySetup {
                config_directories: vec![
                    self.base.path().display().to_string(),
                    self.override_directory.path().display().to_string(),
                ],
                features: vec!["base".to_owned(), "override".to_owned()],
            }
        }

        fn write_override(&self, theme: &str, section_id: &str) {
            fs::write(
                self.override_directory.path().join("override.yaml"),
                format!(
                    "options:\n  appearance:\n    theme: {theme}\n  sections:\n    - command: printf '[]'\n      id: {section_id}\n      item_kind: issue\n      title: Items\n"
                ),
            )
            .unwrap();
        }

        fn write_invalid_template_override(&self) {
            fs::write(
                self.override_directory.path().join("override.yaml"),
                "options:\n  appearance:\n    theme: dark\n  sections:\n    - command: \"printf '\"\n      id: invalid_items\n      item_kind: issue\n      title: Items\n",
            )
            .unwrap();
        }
    }

    fn base_configuration(repository: &Path) -> String {
        format!(
            "options:\n  appearance:\n    theme: system\n  application:\n    command_timeout_seconds: 30\n    default_refresh_seconds: 60\n    max_concurrent_commands: 2\n    max_output_bytes_per_run: 4096\n    shell: /bin/bash\n  autocomplete:\n    command: printf '%s' '{{autocomplete.request}}'\n    debounce_milliseconds: 100\n    instruction: Suggest a useful edit.\n    minimum_characters: 2\n  buttons:\n    issues:\n      advanced: []\n      always: []\n    pull_requests:\n      advanced: []\n      always: []\n  repositories:\n    owner/repository:\n      path: {}\n  sections:\n    - command: printf '[]'\n      id: base_items\n      item_kind: issue\n      title: Items\n",
            repository.display()
        )
    }

    fn workspace_tempdir() -> TempDir {
        tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap()
    }
}
