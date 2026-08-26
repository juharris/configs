import { useState } from "react";

import { ConnectionIndicator } from "../dashboard/DashboardPage";
import type { RunSnapshot } from "../generated/transport";
import { runStatusLabel } from "../runState";
import type { ConnectionStatus } from "../WebSocketClient";

type LogsPageProps = {
  connectionError: string | null;
  connectionStatus: ConnectionStatus;
  runs: readonly RunSnapshot[] | null;
};

export function LogsPage({
  connectionError,
  connectionStatus,
  runs,
}: LogsPageProps) {
  const [copyError, setCopyError] = useState<string | null>(null);
  const [copiedRunId, setCopiedRunId] = useState<string | null>(null);

  const copyCommand = async (run: RunSnapshot) => {
    setCopiedRunId(null);
    setCopyError(null);
    try {
      await navigator.clipboard.writeText(run.preview);
      setCopiedRunId(run.id);
    } catch (error) {
      setCopyError(
        error instanceof Error ? error.message : "Could not copy the command.",
      );
    }
  };

  return (
    <main className="logs-layout">
      <nav className="utility-bar" aria-label="Log controls">
        {connectionError === null ? null : (
          <span className="utility-error">{connectionError}</span>
        )}
        <a
          aria-label="Dashboard"
          className="utility-icon-link"
          href="/"
          title="Dashboard"
        >
          ⌂
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
      <header className="logs-heading">
        <h1>Command logs</h1>
        {runs === null ? null : <span>{runs.length.toString()}</span>}
      </header>
      {copyError === null ? null : (
        <p className="action-error" role="alert">
          {copyError}
        </p>
      )}
      <div className="run-log-list">
        {(runs ?? []).map((run) => {
          const createdAt = new Date(run.createdAt);
          return (
            <article
              className="run-log-entry"
              data-status={run.status}
              key={run.id}
            >
              <header className="run-log-header">
                <div>
                  <strong>{run.label}</strong>
                  <span className="run-status" data-status={run.status}>
                    {runStatusLabel(run.status)}
                  </span>
                  {run.exitCode === null ? null : (
                    <span className="run-exit-code">Exit {run.exitCode}</span>
                  )}
                </div>
                <time
                  dateTime={createdAt.toISOString()}
                  title={createdAt.toString()}
                >
                  {createdAt.toLocaleString()}
                </time>
              </header>
              <div className="run-log-command-heading">
                <span>Attempted command</span>
                <button
                  className="compact-button"
                  onClick={() => void copyCommand(run)}
                  type="button"
                >
                  {copiedRunId === run.id ? "Copied" : "Copy"}
                </button>
              </div>
              <pre className="run-log-command">{run.preview}</pre>
              {run.output === "" ? null : (
                <>
                  <span className="run-log-output-heading">Output</span>
                  <pre className="run-log-output">{run.output}</pre>
                </>
              )}
            </article>
          );
        })}
      </div>
    </main>
  );
}
