import type { RunSnapshot } from "./generated/transport";

const RUN_LOG_CAPACITY = 100;

export function isRunActive(run: RunSnapshot): boolean {
  return run.status === "queued" || run.status === "running";
}

export function runProgress(run: RunSnapshot): number {
  switch (run.status) {
    case "queued":
      return 0;
    case "running":
      return 1;
    case "cancelled":
    case "completed":
    case "failed":
    case "started":
    case "timed_out":
      return 2;
  }
}

export function runStatusLabel(status: RunSnapshot["status"]): string {
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
    case "started":
      return "Started";
    case "timed_out":
      return "Timed out";
  }
}

export function updateRunLogs(
  runs: readonly RunSnapshot[],
  updatedRun: RunSnapshot,
): RunSnapshot[] {
  const updatedRuns = runs.some((run) => run.id === updatedRun.id)
    ? runs.map((run) => (run.id === updatedRun.id ? updatedRun : run))
    : [...runs, updatedRun];
  return updatedRuns
    .sort((left, right) => right.createdAt - left.createdAt)
    .slice(0, RUN_LOG_CAPACITY);
}
