import { describe, expect, it } from "vitest";

import type { RunSnapshot } from "./generated/transport";
import { updateRunLogs } from "./runState";

function run(
  id: string,
  createdAt: number,
  status: RunSnapshot["status"],
): RunSnapshot {
  return {
    createdAt,
    exitCode: null,
    id,
    label: "Test",
    output: "",
    preview: id,
    status,
  };
}

describe("updateRunLogs", () => {
  it("updates snapshots without moving an older run ahead of a newer run", () => {
    const older = run("run-1", 1, "running");
    const newer = run("run-2", 2, "queued");

    const updated = updateRunLogs([newer, older], run("run-1", 1, "failed"));

    expect(updated.map(({ id }) => id)).toEqual(["run-2", "run-1"]);
    expect(updated[1].status).toBe("failed");
  });
});
