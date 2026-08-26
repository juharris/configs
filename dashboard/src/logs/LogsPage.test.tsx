import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { RunSnapshot } from "../generated/transport";
import { LogsPage } from "./LogsPage";

const failedRun: RunSnapshot = {
  createdAt: 1_787_742_000_000,
  exitCode: 23,
  id: "run-2",
  label: "Review",
  output: "launcher failed\ntry the command manually",
  preview: "first line\nsecond line 'with values'",
  status: "failed",
};

describe("LogsPage", () => {
  it("does not show a count before history synchronization", () => {
    render(
      <LogsPage
        connectionError={null}
        connectionStatus="connecting"
        runs={null}
      />,
    );

    expect(screen.queryByText("0")).toBeNull();
  });

  it("shows and copies the exact attempted command for a failed run", async () => {
    const user = userEvent.setup();
    const writeText = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockResolvedValue(undefined);

    render(
      <LogsPage
        connectionError={null}
        connectionStatus="connected"
        runs={[failedRun]}
      />,
    );

    expect(screen.getByText("Failed")).toBeTruthy();
    expect(screen.getByText("Exit 23")).toBeTruthy();
    const command = document.querySelector<HTMLElement>(".run-log-command");
    const output = document.querySelector<HTMLElement>(".run-log-output");
    expect(command?.textContent).toBe(failedRun.preview);
    expect(output?.textContent).toBe(failedRun.output);
    expect(command?.closest("article")?.dataset.status).toBe("failed");

    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(writeText).toHaveBeenCalledWith(failedRun.preview);
    expect(screen.getByRole("button", { name: "Copied" })).toBeTruthy();
  });
});
