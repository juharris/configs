import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type {
  ActiveConfiguration,
  DashboardSnapshot,
} from "../generated/transport";
import { DashboardPage } from "./DashboardPage";

const configuration: ActiveConfiguration = {
  revision: 7,
  setup: {
    configDirectories: ["/dashboard/configs"],
    features: ["dashboard"],
  },
  theme: "dark",
};

const dashboard: DashboardSnapshot = {
  configurationRevision: 7,
  sections: [
    {
      error: null,
      id: "reviews",
      items: [
        {
          advancedButtons: [],
          assignees: ["justin"],
          alwaysButtons: [],
          author: "octocat",
          isDraft: false,
          itemKind: "pull_request",
          labels: [{ color: "1d76db", name: "reviewed" }],
          number: 42,
          repository: "example/project",
          source: "github",
          state: "open",
          title: "Keep the dashboard dense",
          updatedAt: "2026-08-26T12:00:00Z",
          url: "https://app.graphite.com/github/pr/example/project/42",
        },
      ],
      itemsPerPage: 6,
      lastSuccessfulRefresh: 1_787_742_000_000,
      stale: false,
      status: "idle",
      title: "Reviews requested",
    },
  ],
};

describe("DashboardPage", () => {
  it("shows dense actionable section state and requests a typed refresh", async () => {
    const refreshSection = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(
      <DashboardPage
        activeConfiguration={configuration}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={dashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={refreshSection}
        run={null}
        runButton={vi.fn()}
      />,
    );

    expect(screen.getByText("Reviews requested")).toBeTruthy();
    expect(screen.getByText("example/project#42")).toBeTruthy();
    expect(screen.getByText("Keep the dashboard dense")).toBeTruthy();
    expect(screen.getByText("@octocat")).toBeTruthy();
    expect(screen.getByText("reviewed")).toBeTruthy();
    expect(screen.getByText(/^Updated /).textContent).not.toMatch(/[AP]M/i);
    expect(screen.getByRole("status", { name: "Connected" }).textContent).toBe(
      "✓",
    );
    expect(screen.queryByText("Connected")).toBeNull();
    expect(screen.getByRole("link", { name: "Options" }).textContent).toBe("⚙︎");
    expect(screen.queryByText("Options")).toBeNull();
    const reference = screen.getByRole("link", {
      name: "example/project#42",
    });
    expect(reference.getAttribute("href")).toBe(
      "https://app.graphite.com/github/pr/example/project/42",
    );
    expect(reference.closest("li")?.firstElementChild).toBe(
      screen.getByRole("img", { name: "Open" }),
    );
    expect(
      screen.getByRole("link", { name: "@octocat" }).getAttribute("href"),
    ).toBe("https://github.com/octocat");
    expect(
      screen.getByRole("link", { name: "@justin" }).getAttribute("href"),
    ).toBe("https://github.com/justin");
    expect(
      screen.getByRole("link", { name: "@justin" }).parentElement?.textContent,
    ).toBe("→ @justin");
    expect(screen.getByRole("img", { name: "Open" })).toBeTruthy();
    expect(screen.queryByText(/^open$/i)).toBeNull();
    expect(screen.queryByText("Personal Dashboard")).toBeNull();

    const refreshButton = screen.getByRole("button", { name: "Refresh" });
    expect(refreshButton.textContent).toBe("↻");
    expect(refreshButton.getAttribute("title")).toBe("Refresh");
    expect(screen.queryByText("Refresh")).toBeNull();

    await user.click(refreshButton);
    expect(refreshSection).toHaveBeenCalledWith("reviews");
  });

  it("shows a spinner while connecting", () => {
    render(
      <DashboardPage
        activeConfiguration={configuration}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connecting"
        dashboard={dashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={null}
        runButton={vi.fn()}
      />,
    );

    const indicator = screen.getByRole("status", { name: "Connecting…" });
    expect(indicator.querySelector(".connection-spinner")).toBeTruthy();
    expect(indicator.textContent).toBe("");
  });

  it("shows compact accessible icons for pull request states", () => {
    const statusesDashboard = structuredClone(dashboard);
    statusesDashboard.sections[0].items = [
      {
        ...dashboard.sections[0].items[0],
        isDraft: true,
        number: 1,
        state: "open",
      },
      { ...dashboard.sections[0].items[0], number: 2, state: "open" },
      { ...dashboard.sections[0].items[0], number: 3, state: "merged" },
      { ...dashboard.sections[0].items[0], number: 4, state: "closed" },
    ];

    render(
      <DashboardPage
        activeConfiguration={configuration}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={statusesDashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={null}
        runButton={vi.fn()}
      />,
    );

    expect(screen.getByRole("img", { name: "Draft" })).toBeTruthy();
    expect(screen.getByRole("img", { name: "Open" })).toBeTruthy();
    expect(screen.getByRole("img", { name: "Merged" })).toBeTruthy();
    expect(screen.getByRole("img", { name: "Closed" })).toBeTruthy();
  });

  it("shows every label alphabetically", () => {
    const labelsDashboard = structuredClone(dashboard);
    labelsDashboard.sections[0].items[0].labels = [
      { color: "1d76db", name: "complete label name" },
      { color: null, name: "another label" },
    ];

    render(
      <DashboardPage
        activeConfiguration={configuration}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={labelsDashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={null}
        runButton={vi.fn()}
      />,
    );

    const labels = screen.getByLabelText("2 labels");
    expect(Array.from(labels.children, (label) => label.textContent)).toEqual([
      "another label",
      "complete label name",
    ]);
    expect(screen.getByText("complete label name").getAttribute("title")).toBe(
      "complete label name",
    );
  });

  it("formats recent update times relatively and exposes exact local times", () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date("2026-08-26T12:00:00Z"));
      const timesDashboard = structuredClone(dashboard);
      timesDashboard.sections[0].items = [
        {
          ...dashboard.sections[0].items[0],
          number: 1,
          updatedAt: "2026-08-26T11:30:00Z",
        },
        {
          ...dashboard.sections[0].items[0],
          number: 2,
          updatedAt: "2026-08-26T08:30:00Z",
        },
        {
          ...dashboard.sections[0].items[0],
          number: 3,
          updatedAt: "2026-08-23T08:00:00Z",
        },
        {
          ...dashboard.sections[0].items[0],
          number: 4,
          updatedAt: "2026-08-10T12:00:00Z",
        },
      ];

      render(
        <DashboardPage
          activeConfiguration={configuration}
          cancelRun={vi.fn()}
          connectionError={null}
          connectionStatus="connected"
          dashboard={timesDashboard}
          dismissRun={vi.fn()}
          previewButton={vi.fn()}
          refreshSection={vi.fn()}
          run={null}
          runButton={vi.fn()}
        />,
      );

      expect(screen.getByText("30 minutes ago")).toBeTruthy();
      expect(screen.getByText("4 hours ago")).toBeTruthy();
      expect(screen.getByText("3 days ago")).toBeTruthy();
      const olderDate = new Date("2026-08-10T12:00:00Z");
      const olderTime = screen.getByTitle(olderDate.toString());
      expect(olderTime.textContent).toBe(
        new Intl.DateTimeFormat(undefined, {
          day: "numeric",
          month: "short",
        }).format(olderDate),
      );
      expect(olderTime.getAttribute("datetime")).toBe("2026-08-10T12:00:00Z");
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps stale data visible with its error and retry action", () => {
    const staleDashboard = structuredClone(dashboard);
    staleDashboard.sections[0].error = "The section command timed out.";
    staleDashboard.sections[0].stale = true;

    render(
      <DashboardPage
        activeConfiguration={configuration}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={staleDashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={null}
        runButton={vi.fn()}
      />,
    );

    expect(screen.getByText("Stale")).toBeTruthy();
    expect(screen.getByText("The section command timed out.")).toBeTruthy();
    expect(screen.getByText("Keep the dashboard dense")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Retry" })).toBeTruthy();
  });

  it("paginates each section using its configured item count", async () => {
    const paginatedDashboard = structuredClone(dashboard);
    paginatedDashboard.sections[0].items = Array.from(
      { length: 7 },
      (_, index) => ({
        ...dashboard.sections[0].items[0],
        number: index + 1,
        title: `Pull request ${String(index + 1)}`,
      }),
    );
    const user = userEvent.setup();

    render(
      <DashboardPage
        activeConfiguration={configuration}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={paginatedDashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={null}
        runButton={vi.fn()}
      />,
    );

    expect(screen.getByText("Pull request 1")).toBeTruthy();
    expect(screen.queryByText("Pull request 7")).toBeNull();

    await user.click(screen.getByRole("button", { name: "Next page" }));

    expect(screen.queryByText("Pull request 1")).toBeNull();
    expect(screen.getByText("Pull request 7")).toBeTruthy();
  });

  it("renders configured actions and sends prompt values by button position", async () => {
    const actionsDashboard = structuredClone(dashboard);
    actionsDashboard.sections[0].items[0].alwaysButtons = [
      {
        confirm: false,
        disabled: false,
        index: 0,
        label: "Review",
        prompt: {
          label: "Review focus",
          placeholder: "Add areas to inspect",
        },
        title: "codex exec '/review https://example.test/pull/42'",
        url: null,
      },
    ];
    actionsDashboard.sections[0].items[0].advancedButtons = [
      {
        confirm: false,
        disabled: false,
        index: 0,
        label: "Open",
        prompt: null,
        title: "https://example.test/pull/42",
        url: "https://example.test/pull/42",
      },
    ];
    const previewButton = vi
      .fn()
      .mockResolvedValue(
        "codex exec '/review https://example.test/pull/42 focus on tests'",
      );
    const runButton = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(
      <DashboardPage
        activeConfiguration={configuration}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={actionsDashboard}
        dismissRun={vi.fn()}
        previewButton={previewButton}
        refreshSection={vi.fn()}
        run={null}
        runButton={runButton}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Review" }));
    await user.type(screen.getByLabelText("Review focus"), "focus on tests");
    await screen.findByText(
      "codex exec '/review https://example.test/pull/42 focus on tests'",
    );
    await user.click(screen.getByRole("button", { name: "Run" }));

    expect(runButton).toHaveBeenCalledWith(
      "reviews",
      actionsDashboard.sections[0].items[0],
      "always",
      0,
      "focus on tests",
    );

    await user.click(
      screen.getByLabelText("More actions for example/project#42"),
    );
    expect(
      screen.getByRole("link", { name: "Open" }).getAttribute("href"),
    ).toBe("https://example.test/pull/42");
  });

  it("shows live run output and requests cancellation", async () => {
    const cancelRun = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(
      <DashboardPage
        activeConfiguration={configuration}
        cancelRun={cancelRun}
        connectionError={null}
        connectionStatus="connected"
        dashboard={dashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={{
          exitCode: null,
          id: "run-8",
          label: "Review",
          output: "Inspecting files…",
          preview: "codex exec review",
          status: "running",
        }}
        runButton={vi.fn()}
      />,
    );

    expect(screen.getByText("Inspecting files…")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(cancelRun).toHaveBeenCalledWith("run-8");
  });
});
