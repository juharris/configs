import { Fragment, type MouseEvent, useEffect, useRef, useState } from "react";

import type {
  ActiveConfiguration,
  AutocompleteSettings,
  AutocompleteSnapshot,
  ButtonList,
  DashboardActor,
  DashboardButton,
  DashboardItem,
  DashboardSnapshot,
  RunSnapshot,
  SectionSnapshot,
} from "../generated/transport";
import { isRunActive, runStatusLabel } from "../runState";
import type {
  AutocompleteRequestParameters,
  ConnectionStatus,
} from "../WebSocketClient";

type DashboardPageProps = {
  activeConfiguration: ActiveConfiguration | null;
  autocompletes: Readonly<Record<string, AutocompleteSnapshot>>;
  cancelAutocomplete: (editorId: string) => Promise<void>;
  cancelRun: (runId: string) => Promise<void>;
  connectionError: string | null;
  connectionStatus: ConnectionStatus;
  dashboard: DashboardSnapshot | null;
  dismissRun: () => void;
  previewButton: (
    sectionId: string,
    item: DashboardItem,
    buttonList: ButtonList,
    buttonIndex: number,
    prompt: string | null,
  ) => Promise<string>;
  refreshSection: (sectionId: string) => Promise<void>;
  requestAutocomplete: (
    parameters: AutocompleteRequestParameters,
  ) => Promise<void>;
  run: RunSnapshot | null;
  runButton: (
    sectionId: string,
    item: DashboardItem,
    buttonList: ButtonList,
    buttonIndex: number,
    prompt: string | null,
  ) => Promise<void>;
};

export function DashboardPage({
  activeConfiguration,
  autocompletes,
  cancelAutocomplete,
  cancelRun,
  connectionError,
  connectionStatus,
  dashboard,
  dismissRun,
  previewButton,
  refreshSection,
  requestAutocomplete,
  run,
  runButton,
}: DashboardPageProps) {
  const synchronized =
    activeConfiguration !== null &&
    dashboard?.configurationRevision === activeConfiguration.revision;
  const { currentTime, pageVisible } = usePageClock();

  return (
    <main className="dashboard-layout">
      <h1 className="visually-hidden">Personal dashboard</h1>
      <nav className="utility-bar" aria-label="Dashboard controls">
        {connectionError === null ? null : (
          <span className="utility-error">{connectionError}</span>
        )}
        <a
          aria-label="Command logs"
          className="utility-icon-link"
          href="/logs"
          title="Command logs"
        >
          ▤
        </a>
        <ConnectionIndicator status={connectionStatus} />
        <a
          aria-label="Options"
          className="utility-icon-link"
          href="/options"
          title="Options"
        >
          ⚙︎
        </a>
      </nav>

      {activeConfiguration === null || dashboard === null ? (
        <p className="dashboard-notice">Loading configuration and sections…</p>
      ) : !synchronized ? (
        <p className="dashboard-notice">Synchronizing configuration…</p>
      ) : (
        <div className="dashboard-sections">
          {dashboard.sections.map((section) => (
            <DashboardSection
              autocompleteSettings={activeConfiguration.autocomplete}
              autocompletes={autocompletes}
              cancelAutocomplete={cancelAutocomplete}
              configurationRevision={dashboard.configurationRevision}
              connectionStatus={connectionStatus}
              currentTime={currentTime}
              key={`${dashboard.configurationRevision.toString()}:${section.id}`}
              pageVisible={pageVisible}
              previewButton={(item, buttonList, buttonIndex, prompt) =>
                previewButton(section.id, item, buttonList, buttonIndex, prompt)
              }
              refresh={() => refreshSection(section.id)}
              requestAutocomplete={requestAutocomplete}
              runButton={(item, buttonList, buttonIndex, prompt) =>
                runButton(section.id, item, buttonList, buttonIndex, prompt)
              }
              section={section}
            />
          ))}
        </div>
      )}

      {run === null ? null : (
        <RunDrawer
          cancelRun={cancelRun}
          dismiss={dismissRun}
          key={run.id}
          run={run}
        />
      )}
    </main>
  );
}

export function ConnectionIndicator({ status }: { status: ConnectionStatus }) {
  const label = connectionStatusLabel(status);
  const symbol =
    status === "connected" ? "↔" : status === "disconnected" ? "!" : null;

  return (
    <span
      aria-label={label}
      className="connection-indicator"
      data-status={status}
      role="status"
      title={label}
    >
      {symbol === null ? (
        <span aria-hidden="true" className="connection-spinner" />
      ) : (
        <span aria-hidden="true">{symbol}</span>
      )}
    </span>
  );
}

type RunButton = (
  item: DashboardItem,
  buttonList: ButtonList,
  buttonIndex: number,
  prompt: string | null,
) => Promise<void>;

type PreviewButton = (
  item: DashboardItem,
  buttonList: ButtonList,
  buttonIndex: number,
  prompt: string | null,
) => Promise<string>;

function DashboardSection({
  autocompleteSettings,
  autocompletes,
  cancelAutocomplete,
  configurationRevision,
  connectionStatus,
  currentTime,
  pageVisible,
  previewButton,
  refresh,
  requestAutocomplete,
  runButton,
  section,
}: {
  autocompleteSettings: AutocompleteSettings;
  autocompletes: Readonly<Record<string, AutocompleteSnapshot>>;
  cancelAutocomplete: (editorId: string) => Promise<void>;
  configurationRevision: number;
  connectionStatus: ConnectionStatus;
  currentTime: number;
  pageVisible: boolean;
  previewButton: PreviewButton;
  refresh: () => Promise<void>;
  requestAutocomplete: (
    parameters: AutocompleteRequestParameters,
  ) => Promise<void>;
  runButton: RunButton;
  section: SectionSnapshot;
}) {
  const [collapsed, setCollapsed] = useState(section.collapsed);
  const isRefreshing = section.status !== "idle";
  const pageCount = Math.max(
    1,
    Math.ceil(section.items.length / section.itemsPerPage),
  );
  const [requestedPage, setRequestedPage] = useState(0);
  const refreshRef = useRef(refresh);
  const page = Math.min(requestedPage, pageCount - 1);
  const refreshDisabled = connectionStatus !== "connected" || isRefreshing;
  const visibleItems = section.items.slice(
    page * section.itemsPerPage,
    (page + 1) * section.itemsPerPage,
  );

  useEffect(() => {
    refreshRef.current = refresh;
  }, [refresh]);

  useEffect(() => {
    if (collapsed || connectionStatus !== "connected" || !pageVisible) {
      return;
    }
    const requestRefresh = () => {
      void Promise.resolve(refreshRef.current()).catch(() => undefined);
    };
    requestRefresh();
    const interval = window.setInterval(
      requestRefresh,
      section.refreshSeconds * 1_000,
    );
    return () => window.clearInterval(interval);
  }, [
    collapsed,
    configurationRevision,
    connectionStatus,
    pageVisible,
    section.id,
    section.refreshSeconds,
  ]);

  return (
    <section
      className="dashboard-section"
      aria-labelledby={`${section.id}-heading`}
      data-collapsed={collapsed}
    >
      <header className="dashboard-section-header">
        <h2 className="dashboard-section-heading" id={`${section.id}-heading`}>
          <button
            aria-label={`${collapsed ? "Expand" : "Collapse"} ${section.title}`}
            aria-controls={`${section.id}-content`}
            aria-expanded={!collapsed}
            className="dashboard-section-toggle"
            onClick={() => setCollapsed((current) => !current)}
            title={`${collapsed ? "Expand" : "Collapse"} ${section.title}`}
            type="button"
          >
            <span aria-hidden="true" className="section-toggle-icon">
              {collapsed ? "▸" : "▾"}
            </span>
            <span className="section-title">{section.title}</span>
            {section.lastSuccessfulRefresh === null ? null : (
              <span className="section-count">{section.items.length}</span>
            )}
            {section.stale ? <span className="stale-badge">Stale</span> : null}
            <span className="section-refresh-state">
              {refreshStatusLabel(section.status)}
            </span>
            {section.lastSuccessfulRefresh === null ? null : (
              <span className="section-updated">
                Updated{" "}
                <UpdatedTime
                  currentTime={currentTime}
                  value={section.lastSuccessfulRefresh}
                />
              </span>
            )}
          </button>
        </h2>
        {collapsed ? null : (
          <div className="section-controls">
            {pageCount === 1 ? null : (
              <div className="section-pagination" aria-label="Section pages">
                <button
                  aria-label="Previous page"
                  className="compact-button"
                  disabled={page === 0}
                  onClick={() => setRequestedPage(page - 1)}
                  type="button"
                >
                  ‹
                </button>
                <span>
                  {page + 1}/{pageCount}
                </span>
                <button
                  aria-label="Next page"
                  className="compact-button"
                  disabled={page === pageCount - 1}
                  onClick={() => setRequestedPage(page + 1)}
                  type="button"
                >
                  ›
                </button>
              </div>
            )}
            <button
              aria-label="Refresh"
              className="compact-button icon-button"
              disabled={refreshDisabled}
              onClick={() => void refresh().catch(() => undefined)}
              title="Refresh"
              type="button"
            >
              <span aria-hidden="true">↻</span>
            </button>
          </div>
        )}
      </header>

      {collapsed ? null : (
        <div id={`${section.id}-content`}>
          {section.error === null ? null : (
            <div className="section-error" role="alert">
              <span className="section-error-message">{section.error}</span>
              <button
                className="text-button"
                disabled={refreshDisabled}
                onClick={() => void refresh().catch(() => undefined)}
                type="button"
              >
                Retry
              </button>
            </div>
          )}

          <ul className="item-list">
            {visibleItems.map((item) => (
              <DashboardItemRow
                autocompleteSettings={autocompleteSettings}
                autocompleteSnapshot={
                  autocompletes[itemEditorId(section.id, item)]
                }
                cancelAutocomplete={cancelAutocomplete}
                configurationRevision={configurationRevision}
                connectionStatus={connectionStatus}
                currentTime={currentTime}
                item={item}
                key={`${item.source ?? "default"}:${item.repository}#${item.number.toString()}`}
                previewButton={previewButton}
                requestAutocomplete={requestAutocomplete}
                runButton={runButton}
                sectionId={section.id}
              />
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

type PendingAction = {
  button: DashboardButton;
  buttonList: ButtonList;
};

function DashboardItemRow({
  autocompleteSettings,
  autocompleteSnapshot,
  cancelAutocomplete,
  configurationRevision,
  connectionStatus,
  currentTime,
  item,
  previewButton,
  requestAutocomplete,
  runButton,
  sectionId,
}: {
  autocompleteSettings: AutocompleteSettings;
  autocompleteSnapshot: AutocompleteSnapshot | undefined;
  cancelAutocomplete: (editorId: string) => Promise<void>;
  configurationRevision: number;
  connectionStatus: ConnectionStatus;
  currentTime: number;
  item: DashboardItem;
  previewButton: PreviewButton;
  requestAutocomplete: (
    parameters: AutocompleteRequestParameters,
  ) => Promise<void>;
  runButton: RunButton;
  sectionId: string;
}) {
  const [actionError, setActionError] = useState<string | null>(null);
  const [autocompleteError, setAutocompleteError] = useState<string | null>(
    null,
  );
  const [activeAutocompleteId, setActiveAutocompleteId] = useState<
    string | null
  >(null);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(
    null,
  );
  const [preview, setPreview] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [prompt, setPrompt] = useState("");
  const [selection, setSelection] = useState({ end: 0, start: 0 });
  const [submitting, setSubmitting] = useState(false);
  const autocompleteIdRef = useRef<string | null>(null);
  const autocompleteSequenceRef = useRef(0);
  const cancelAutocompleteRef = useRef(cancelAutocomplete);
  const requestAutocompleteRef = useRef(requestAutocomplete);
  const suppressAutocompleteRef = useRef(false);
  const currentEditorId = itemEditorId(sectionId, item);
  const promptId = `${item.source ?? "default"}-${item.repository}-${item.number.toString()}-prompt`;
  const status = itemStatusPresentation(item);

  useEffect(() => {
    cancelAutocompleteRef.current = cancelAutocomplete;
  }, [cancelAutocomplete]);

  useEffect(() => {
    requestAutocompleteRef.current = requestAutocomplete;
  }, [requestAutocomplete]);

  useEffect(() => {
    if (
      pendingAction === null ||
      pendingAction.button.prompt === null ||
      connectionStatus !== "connected" ||
      Array.from(prompt).length < autocompleteSettings.minimumCharacters
    ) {
      return;
    }
    if (suppressAutocompleteRef.current) {
      suppressAutocompleteRef.current = false;
      return;
    }

    const timeout = window.setTimeout(() => {
      const autocompleteId = `${currentEditorId}:${(++autocompleteSequenceRef.current).toString()}`;
      autocompleteIdRef.current = autocompleteId;
      setActiveAutocompleteId(autocompleteId);
      setAutocompleteError(null);
      void requestAutocompleteRef
        .current({
          autocompleteId,
          buttonIndex: pendingAction.button.index,
          buttonList: pendingAction.buttonList,
          configurationRevision,
          draft: prompt,
          editorId: currentEditorId,
          item: {
            number: item.number,
            repository: item.repository,
            source: item.source,
          },
          sectionId,
          selectionEnd: selection.end,
          selectionStart: selection.start,
        })
        .catch((error: unknown) => {
          if (autocompleteIdRef.current !== autocompleteId) {
            return;
          }
          autocompleteIdRef.current = null;
          setActiveAutocompleteId(null);
          setAutocompleteError(
            error instanceof Error
              ? error.message
              : "Could not request autocomplete.",
          );
        });
    }, autocompleteSettings.debounceMilliseconds);
    return () => window.clearTimeout(timeout);
  }, [
    autocompleteSettings.debounceMilliseconds,
    autocompleteSettings.minimumCharacters,
    configurationRevision,
    connectionStatus,
    currentEditorId,
    item.number,
    item.repository,
    item.source,
    pendingAction,
    prompt,
    sectionId,
    selection.end,
    selection.start,
  ]);

  useEffect(
    () => () => {
      if (autocompleteIdRef.current !== null) {
        void cancelAutocompleteRef
          .current(currentEditorId)
          .catch(() => undefined);
      }
    },
    [currentEditorId],
  );

  useEffect(() => {
    if (pendingAction === null || connectionStatus !== "connected") {
      return;
    }
    let active = true;
    const timeout = window.setTimeout(() => {
      void previewButton(
        item,
        pendingAction.buttonList,
        pendingAction.button.index,
        pendingAction.button.prompt === null ? null : prompt,
      )
        .then((resolvedPreview) => {
          if (active) {
            setActionError(null);
            setPreview(resolvedPreview);
          }
        })
        .catch((error: unknown) => {
          if (active) {
            setActionError(
              error instanceof Error
                ? error.message
                : "Could not preview the action.",
            );
          }
        })
        .finally(() => {
          if (active) {
            setPreviewing(false);
          }
        });
    }, 250);
    return () => {
      active = false;
      window.clearTimeout(timeout);
    };
  }, [connectionStatus, item, pendingAction, previewButton, prompt]);

  const execute = async (action: PendingAction, value: string | null) => {
    setActionError(null);
    setSubmitting(true);
    try {
      await runButton(item, action.buttonList, action.button.index, value);
      cancelCurrentAutocomplete();
      setPendingAction(null);
      setPrompt("");
    } catch (error) {
      setActionError(
        error instanceof Error ? error.message : "Could not run the action.",
      );
    } finally {
      setSubmitting(false);
    }
  };

  const selectAction = (button: DashboardButton, buttonList: ButtonList) => {
    const action = { button, buttonList };
    const defaultPrompt = button.prompt?.default ?? "";
    cancelCurrentAutocomplete();
    setActionError(null);
    setAutocompleteError(null);
    setPendingAction(action);
    setPreview(null);
    setPreviewing(true);
    setPrompt(defaultPrompt);
    setSelection({ end: defaultPrompt.length, start: defaultPrompt.length });
  };

  const cancelCurrentAutocomplete = () => {
    if (autocompleteIdRef.current === null) {
      return;
    }
    autocompleteIdRef.current = null;
    setActiveAutocompleteId(null);
    void cancelAutocompleteRef.current(currentEditorId).catch(() => undefined);
  };

  const currentAutocomplete =
    autocompleteSnapshot?.autocompleteId === activeAutocompleteId
      ? autocompleteSnapshot
      : undefined;
  const suggestion = currentAutocomplete?.suggestion ?? null;

  return (
    <li
      className="item-row"
      onClick={(event) => openItemFromRow(event, item.url)}
    >
      <span
        aria-label={status.label}
        className="item-state"
        data-item-kind={item.itemKind}
        data-status={status.status}
        role="img"
        title={status.label}
      >
        {status.icon}
      </span>
      <a
        className="item-reference"
        href={item.url}
        rel="noreferrer noopener"
        target="_blank"
        title={item.title}
      >
        {item.repository}#{item.number}
      </a>
      <a
        className="item-title"
        href={item.url}
        rel="noreferrer noopener"
        target="_blank"
        title={item.title}
      >
        {item.title}
      </a>
      {item.approvedBy.length === 0 ? null : (
        <span
          className="item-approvers"
          title={`Approved by ${item.approvedBy.map((approver) => approver.login).join(", ")}`}
        >
          {item.approvedBy.map((approver) => (
            <span className="item-approver" key={approver.login}>
              ✓ <DashboardActorAlias actor={approver} />
            </span>
          ))}
        </span>
      )}
      <span className="item-context">
        {item.author === null ? (
          <span>unknown author</span>
        ) : (
          <GitHubAlias login={item.author} />
        )}
        {item.assignees.length === 0 ? null : (
          <span title={`Assigned to ${item.assignees.join(", ")}`}>
            {item.assignees.map((assignee, index) => (
              <Fragment key={assignee}>
                {index === 0 ? "→ " : ", "}
                <GitHubAlias login={assignee} />
              </Fragment>
            ))}
          </span>
        )}
      </span>
      <ItemLabels labels={item.labels} />
      <UpdatedTime
        className="item-time"
        currentTime={currentTime}
        value={item.updatedAt}
      />
      <div className="item-actions">
        {item.alwaysButtons.map((button) => (
          <ActionButton
            button={button}
            connectionStatus={connectionStatus}
            key={button.index}
            onCommand={() => selectAction(button, "always")}
          />
        ))}
        {item.advancedButtons.length === 0 ? null : (
          <details className="advanced-actions">
            <summary
              aria-label={`More actions for ${item.repository}#${item.number.toString()}`}
              title="More actions"
            >
              •••
            </summary>
            <div className="advanced-actions-menu">
              {item.advancedButtons.map((button) => (
                <ActionButton
                  button={button}
                  connectionStatus={connectionStatus}
                  key={button.index}
                  onCommand={() => selectAction(button, "advanced")}
                />
              ))}
            </div>
          </details>
        )}
      </div>
      {pendingAction === null ? null : (
        <form
          className="command-editor"
          onSubmit={(event) => {
            event.preventDefault();
            void execute(
              pendingAction,
              pendingAction.button.prompt === null ? null : prompt,
            );
          }}
        >
          {pendingAction.button.prompt === null ? null : (
            <>
              <label htmlFor={promptId}>
                {pendingAction.button.prompt.label}
              </label>
              <input
                autoFocus
                disabled={submitting}
                id={promptId}
                onChange={(event) => {
                  const end = event.currentTarget.selectionEnd ?? 0;
                  const start = event.currentTarget.selectionStart ?? end;
                  cancelCurrentAutocomplete();
                  setAutocompleteError(null);
                  setPreview(null);
                  setPreviewing(true);
                  setPrompt(event.target.value);
                  setSelection({ end, start });
                }}
                onSelect={(event) => {
                  const end = event.currentTarget.selectionEnd ?? 0;
                  const start = event.currentTarget.selectionStart ?? end;
                  cancelCurrentAutocomplete();
                  setAutocompleteError(null);
                  setSelection({ end, start });
                }}
                placeholder={pendingAction.button.prompt.placeholder}
                value={prompt}
              />
            </>
          )}
          <button
            className="action-button"
            disabled={
              connectionStatus !== "connected" ||
              preview === null ||
              previewing ||
              submitting
            }
            title={preview ?? pendingAction.button.title}
            type="submit"
          >
            {submitting ? "Starting…" : previewing ? "Previewing…" : "Run"}
          </button>
          <button
            className="text-button"
            disabled={submitting}
            onClick={() => {
              cancelCurrentAutocomplete();
              setPendingAction(null);
            }}
            type="button"
          >
            Cancel
          </button>
          {preview === null ? null : (
            <pre className="command-preview">{preview}</pre>
          )}
          {pendingAction.button.prompt === null ? null : (
            <div className="autocomplete-feedback" aria-live="polite">
              {connectionStatus === "connected" ? null : (
                <span>Autocomplete unavailable while disconnected.</span>
              )}
              {activeAutocompleteId !== null &&
              currentAutocomplete === undefined ? (
                <span>Suggesting…</span>
              ) : null}
              {autocompleteError === null ? null : (
                <span className="autocomplete-error" role="alert">
                  {autocompleteError}
                </span>
              )}
              {currentAutocomplete?.error === null ||
              currentAutocomplete?.error === undefined ? null : (
                <span className="autocomplete-error" role="alert">
                  {currentAutocomplete.error}
                </span>
              )}
              {suggestion === null ? null : (
                <span className="autocomplete-suggestion">
                  <span>{suggestion}</span>
                  <button
                    className="text-button"
                    onClick={() => {
                      const end = suggestion.length;
                      autocompleteIdRef.current = null;
                      setActiveAutocompleteId(null);
                      suppressAutocompleteRef.current = true;
                      setPreview(null);
                      setPreviewing(true);
                      setPrompt(suggestion);
                      setSelection({ end, start: end });
                    }}
                    type="button"
                  >
                    Use suggestion
                  </button>
                </span>
              )}
            </div>
          )}
        </form>
      )}
      {actionError === null ? null : (
        <span className="action-error" role="alert">
          {actionError}
        </span>
      )}
    </li>
  );
}

function itemEditorId(sectionId: string, item: DashboardItem): string {
  return `${sectionId}:${item.source ?? "default"}:${item.repository}#${item.number.toString()}`;
}

function openItemFromRow(event: MouseEvent<HTMLLIElement>, url: string) {
  const target = event.target;
  if (
    event.defaultPrevented ||
    (target instanceof Element &&
      target.closest("a, button, details, form, input, summary, textarea") !==
        null) ||
    window.getSelection()?.isCollapsed === false
  ) {
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

function GitHubAlias({ login }: { login: string }) {
  return (
    <PersonAlias
      login={login}
      title={`Open @${login} on GitHub`}
      url={`https://github.com/${encodeURIComponent(login)}`}
    />
  );
}

function DashboardActorAlias({ actor }: { actor: DashboardActor }) {
  if (actor.url === null) {
    return <span>@{actor.login}</span>;
  }
  return (
    <PersonAlias
      login={actor.login}
      title={`Open @${actor.login}`}
      url={actor.url}
    />
  );
}

function PersonAlias({
  login,
  title,
  url,
}: {
  login: string;
  title: string;
  url: string;
}) {
  return (
    <a
      className="item-person"
      href={url}
      rel="noreferrer noopener"
      target="_blank"
      title={title}
    >
      @{login}
    </a>
  );
}

function ItemLabels({ labels }: { labels: DashboardItem["labels"] }) {
  const sortedLabels = labels
    .slice()
    .sort((left, right) => left.name.localeCompare(right.name));

  return (
    <span
      aria-label={
        labels.length === 1 ? "1 label" : `${labels.length.toString()} labels`
      }
      className="item-labels"
    >
      {sortedLabels.map((label) => (
        <span className="item-label" key={label.name} title={label.name}>
          {label.name}
        </span>
      ))}
    </span>
  );
}

function ActionButton({
  button,
  connectionStatus,
  onCommand,
}: {
  button: DashboardButton;
  connectionStatus: ConnectionStatus;
  onCommand: () => void;
}) {
  if (button.url !== null) {
    return (
      <a
        className="action-button"
        href={button.url}
        rel="noreferrer noopener"
        target="_blank"
        title={button.title}
      >
        {button.label}
      </a>
    );
  }
  return (
    <button
      className="action-button"
      disabled={button.disabled || connectionStatus !== "connected"}
      onClick={onCommand}
      title={button.title}
      type="button"
    >
      {button.label}
    </button>
  );
}

function RunDrawer({
  cancelRun,
  dismiss,
  run,
}: {
  cancelRun: (runId: string) => Promise<void>;
  dismiss: () => void;
  run: RunSnapshot;
}) {
  const [cancelError, setCancelError] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const active = isRunActive(run);
  const elapsedSeconds = useRunElapsed(active);

  return (
    <aside className="run-drawer" aria-label="Command run" aria-live="polite">
      <header className="run-drawer-header">
        <div>
          <strong>{run.label}</strong>
          <span className="run-status" data-status={run.status}>
            {runStatusLabel(run.status)} · {elapsedSeconds}s
          </span>
        </div>
        <div className="run-controls">
          {active ? (
            <button
              className="compact-button"
              disabled={cancelling}
              onClick={() => {
                setCancelling(true);
                setCancelError(null);
                void cancelRun(run.id)
                  .catch((error: unknown) => {
                    setCancelError(
                      error instanceof Error
                        ? error.message
                        : "Could not cancel the command.",
                    );
                  })
                  .finally(() => setCancelling(false));
              }}
              type="button"
            >
              {cancelling ? "Cancelling…" : "Cancel"}
            </button>
          ) : null}
          <button
            aria-label="Close command run"
            className="compact-button"
            onClick={dismiss}
            type="button"
          >
            ×
          </button>
        </div>
      </header>
      <pre className="run-preview">{run.preview}</pre>
      <pre className="run-output">
        {run.output === "" ? "Waiting for output…" : run.output}
      </pre>
      {run.exitCode === null ? null : (
        <span className="run-exit-code">Exit {run.exitCode}</span>
      )}
      {cancelError === null ? null : (
        <span className="action-error" role="alert">
          {cancelError}
        </span>
      )}
    </aside>
  );
}

function connectionStatusLabel(status: ConnectionStatus): string {
  switch (status) {
    case "connected":
      return "Connected";
    case "connecting":
      return "Connecting…";
    case "disconnected":
      return "Disconnected";
    case "reconnecting":
      return "Reconnecting…";
  }
}

function formatDate(date: Date): string {
  return new Intl.DateTimeFormat(undefined, {
    day: "numeric",
    month: "short",
  }).format(date);
}

function formatUpdatedAt(date: Date, currentTime: number): string {
  const elapsedMilliseconds = currentTime - date.getTime();
  const secondMilliseconds = 1_000;
  const minuteMilliseconds = 60 * secondMilliseconds;
  const hourMilliseconds = 60 * minuteMilliseconds;
  const dayMilliseconds = 24 * hourMilliseconds;

  if (elapsedMilliseconds < 0) {
    return formatDate(date);
  }
  if (elapsedMilliseconds < minuteMilliseconds) {
    return relativeTimeLabel(
      Math.floor(elapsedMilliseconds / secondMilliseconds),
      "second",
    );
  }
  if (elapsedMilliseconds < hourMilliseconds) {
    return relativeTimeLabel(
      Math.floor(elapsedMilliseconds / minuteMilliseconds),
      "minute",
    );
  }
  if (elapsedMilliseconds < dayMilliseconds) {
    return relativeTimeLabel(
      Math.round(elapsedMilliseconds / hourMilliseconds),
      "hour",
    );
  }
  if (elapsedMilliseconds < 7 * dayMilliseconds) {
    return relativeTimeLabel(
      Math.floor(elapsedMilliseconds / dayMilliseconds),
      "day",
    );
  }
  return formatDate(date);
}

type ItemStatusPresentation = {
  icon: string;
  label: string;
  status: "approved" | "closed" | "draft" | "merged" | "open" | "unknown";
};

function itemStatusPresentation(item: DashboardItem): ItemStatusPresentation {
  const state = item.state.toLowerCase();
  if (item.isDraft === true || state === "draft") {
    return { icon: "📝", label: "Draft", status: "draft" };
  }
  switch (state) {
    case "approved":
      return { icon: "", label: "Approved", status: "approved" };
    case "closed":
      return { icon: "●", label: "Closed", status: "closed" };
    case "merged":
      return { icon: "●", label: "Merged", status: "merged" };
    case "open":
      return { icon: "●", label: "Open", status: "open" };
    default:
      return { icon: "?", label: item.state, status: "unknown" };
  }
}

function refreshStatusLabel(status: SectionSnapshot["status"]): string {
  switch (status) {
    case "idle":
      return "";
    case "queued":
      return "Queued";
    case "refreshing":
      return "Refreshing…";
  }
}

function relativeTimeLabel(value: number, unit: string): string {
  return `${value.toString()} ${unit}${value === 1 ? "" : "s"} ago`;
}

function UpdatedTime({
  className,
  currentTime,
  value,
}: {
  className?: string;
  currentTime: number;
  value: number | string;
}) {
  const date = new Date(value);
  return (
    <time
      className={className}
      dateTime={date.toISOString()}
      title={date.toString()}
    >
      {formatUpdatedAt(date, currentTime)}
    </time>
  );
}

function usePageClock(): { currentTime: number; pageVisible: boolean } {
  const [clock, setClock] = useState(() => ({
    currentTime: Date.now(),
    pageVisible: document.visibilityState === "visible",
  }));

  useEffect(() => {
    let interval: number | undefined;
    const stopClock = () => {
      if (interval !== undefined) {
        window.clearInterval(interval);
        interval = undefined;
      }
    };
    const updateClock = () =>
      setClock((current) => ({ ...current, currentTime: Date.now() }));
    const startClock = () => {
      stopClock();
      interval = window.setInterval(updateClock, 30_000);
    };
    const updateVisibility = () => {
      const pageVisible = document.visibilityState === "visible";
      setClock((current) => ({
        currentTime: pageVisible ? Date.now() : current.currentTime,
        pageVisible,
      }));
      if (pageVisible) {
        startClock();
      } else {
        stopClock();
      }
    };
    if (document.visibilityState === "visible") {
      startClock();
    }
    document.addEventListener("visibilitychange", updateVisibility);
    return () => {
      stopClock();
      document.removeEventListener("visibilitychange", updateVisibility);
    };
  }, []);

  return clock;
}

function useRunElapsed(active: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  const [startedAt] = useState(now);

  useEffect(() => {
    if (!active) {
      return;
    }
    const interval = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(interval);
  }, [active]);

  return Math.floor((now - startedAt) / 1_000);
}
