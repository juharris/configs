import { Fragment, useEffect, useState } from "react";

import type {
  ActiveConfiguration,
  ButtonList,
  DashboardButton,
  DashboardItem,
  DashboardSnapshot,
  RunSnapshot,
  SectionSnapshot,
} from "../generated/transport";
import type { ConnectionStatus } from "../WebSocketClient";

type DashboardPageProps = {
  activeConfiguration: ActiveConfiguration | null;
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
    prompt: string,
  ) => Promise<string>;
  refreshSection: (sectionId: string) => Promise<void>;
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
  cancelRun,
  connectionError,
  connectionStatus,
  dashboard,
  dismissRun,
  previewButton,
  refreshSection,
  run,
  runButton,
}: DashboardPageProps) {
  const synchronized =
    activeConfiguration !== null &&
    dashboard?.configurationRevision === activeConfiguration.revision;

  return (
    <main className="dashboard-layout">
      <h1 className="visually-hidden">Personal dashboard</h1>
      <nav className="utility-bar" aria-label="Dashboard controls">
        {connectionError === null ? null : (
          <span className="utility-error">{connectionError}</span>
        )}
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
              connectionStatus={connectionStatus}
              key={section.id}
              previewButton={(item, buttonList, buttonIndex, prompt) =>
                previewButton(section.id, item, buttonList, buttonIndex, prompt)
              }
              refresh={() => refreshSection(section.id)}
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

function ConnectionIndicator({ status }: { status: ConnectionStatus }) {
  const label = connectionStatusLabel(status);
  const symbol =
    status === "connected" ? "✓" : status === "disconnected" ? "!" : null;

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
  prompt: string,
) => Promise<string>;

function DashboardSection({
  connectionStatus,
  previewButton,
  refresh,
  runButton,
  section,
}: {
  connectionStatus: ConnectionStatus;
  previewButton: PreviewButton;
  refresh: () => Promise<void>;
  runButton: RunButton;
  section: SectionSnapshot;
}) {
  const isRefreshing = section.status !== "idle";
  const pageCount = Math.max(
    1,
    Math.ceil(section.items.length / section.itemsPerPage),
  );
  const [requestedPage, setRequestedPage] = useState(0);
  const page = Math.min(requestedPage, pageCount - 1);
  const refreshDisabled = connectionStatus !== "connected" || isRefreshing;
  const visibleItems = section.items.slice(
    page * section.itemsPerPage,
    (page + 1) * section.itemsPerPage,
  );

  return (
    <section
      className="dashboard-section"
      aria-labelledby={`${section.id}-heading`}
    >
      <header className="dashboard-section-header">
        <div className="dashboard-section-identity">
          <h2 id={`${section.id}-heading`}>{section.title}</h2>
          <span className="section-count">{section.items.length}</span>
          {section.stale ? <span className="stale-badge">Stale</span> : null}
          <span className="section-refresh-state">
            {refreshStatusLabel(section.status)}
          </span>
          {section.lastSuccessfulRefresh === null ? null : (
            <span className="section-updated">
              Updated {formatTime(section.lastSuccessfulRefresh)}
            </span>
          )}
        </div>
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
      </header>

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
            connectionStatus={connectionStatus}
            item={item}
            key={`${item.source ?? "default"}:${item.repository}#${item.number.toString()}`}
            previewButton={previewButton}
            runButton={runButton}
          />
        ))}
      </ul>
    </section>
  );
}

type PendingAction = {
  button: DashboardButton;
  buttonList: ButtonList;
};

function DashboardItemRow({
  connectionStatus,
  item,
  previewButton,
  runButton,
}: {
  connectionStatus: ConnectionStatus;
  item: DashboardItem;
  previewButton: PreviewButton;
  runButton: RunButton;
}) {
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(
    null,
  );
  const [preview, setPreview] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [prompt, setPrompt] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const promptId = `${item.source ?? "default"}-${item.repository}-${item.number.toString()}-prompt`;
  const status = itemStatusPresentation(item);

  useEffect(() => {
    if (
      pendingAction === null ||
      pendingAction.button.prompt === null ||
      connectionStatus !== "connected"
    ) {
      return;
    }
    let active = true;
    const timeout = window.setTimeout(() => {
      void previewButton(
        item,
        pendingAction.buttonList,
        pendingAction.button.index,
        prompt,
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
    if (
      action.button.confirm &&
      !window.confirm(
        `Run ${action.button.label}?\n\n${preview ?? action.button.title}`,
      )
    ) {
      return;
    }
    setActionError(null);
    setSubmitting(true);
    try {
      await runButton(item, action.buttonList, action.button.index, value);
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
    if (button.prompt === null) {
      void execute(action, null);
      return;
    }
    setActionError(null);
    setPendingAction(action);
    setPreview(null);
    setPreviewing(true);
    setPrompt("");
  };

  return (
    <li className="item-row">
      <span
        aria-label={status.label}
        className="item-state"
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
      <time
        className="item-time"
        dateTime={item.updatedAt}
        title={new Date(item.updatedAt).toString()}
      >
        {formatItemUpdatedAt(item.updatedAt)}
      </time>
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
          className="prompt-editor"
          onSubmit={(event) => {
            event.preventDefault();
            void execute(pendingAction, prompt);
          }}
        >
          <label htmlFor={promptId}>{pendingAction.button.prompt?.label}</label>
          <input
            autoFocus
            disabled={submitting}
            id={promptId}
            onChange={(event) => {
              setPreview(null);
              setPreviewing(true);
              setPrompt(event.target.value);
            }}
            placeholder={pendingAction.button.prompt?.placeholder}
            value={prompt}
          />
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
            onClick={() => setPendingAction(null)}
            type="button"
          >
            Cancel
          </button>
          {preview === null ? null : (
            <pre className="prompt-preview">{preview}</pre>
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

function GitHubAlias({ login }: { login: string }) {
  return (
    <a
      className="item-person"
      href={`https://github.com/${encodeURIComponent(login)}`}
      rel="noreferrer noopener"
      target="_blank"
      title={`Open @${login} on GitHub`}
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
  const active = run.status === "queued" || run.status === "running";
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

function formatItemDate(date: Date): string {
  return new Intl.DateTimeFormat(undefined, {
    day: "numeric",
    month: "short",
  }).format(date);
}

function formatItemUpdatedAt(value: string): string {
  const date = new Date(value);
  const elapsedMilliseconds = Date.now() - date.getTime();
  const minuteMilliseconds = 60 * 1_000;
  const hourMilliseconds = 60 * minuteMilliseconds;
  const dayMilliseconds = 24 * hourMilliseconds;

  if (elapsedMilliseconds < 0) {
    return formatItemDate(date);
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
  return formatItemDate(date);
}

function formatTime(value: number): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    hourCycle: "h23",
    minute: "2-digit",
  }).format(new Date(value));
}

type ItemStatusPresentation = {
  icon: string;
  label: string;
  status: "closed" | "draft" | "merged" | "open" | "unknown";
};

function itemStatusPresentation(item: DashboardItem): ItemStatusPresentation {
  const state = item.state.toLowerCase();
  if (item.isDraft === true || state === "draft") {
    return { icon: "📝", label: "Draft", status: "draft" };
  }
  switch (state) {
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

function runStatusLabel(status: RunSnapshot["status"]): string {
  switch (status) {
    case "cancelled":
      return "Cancelled";
    case "completed":
      return "Completed";
    case "failed":
      return "Failed";
    case "queued":
      return "Queued";
    case "running":
      return "Running";
    case "timed_out":
      return "Timed out";
  }
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
