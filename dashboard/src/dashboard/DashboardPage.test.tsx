import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type {
  ActiveConfiguration,
  AutocompleteSnapshot,
  DashboardSnapshot,
} from "../generated/transport";
import { DashboardPage } from "./DashboardPage";

const configuration: ActiveConfiguration = {
  autocomplete: {
    debounceMilliseconds: 300,
    minimumCharacters: 20,
  },
  revision: 7,
  setup: {
    configDirectories: ["/dashboard/configs"],
    features: ["dashboard"],
  },
  theme: "dark",
  workingDirectories: ["/workspace/first", "/workspace/second"],
};

const dashboard: DashboardSnapshot = {
  configurationRevision: 7,
  sections: [
    {
      collapsed: false,
      error: null,
      id: "reviews",
      items: [
        {
          advancedButtons: [],
          approvedBy: [
            {
              login: "hubot",
              url: "https://example.test/people/hubot",
            },
            {
              login: "monalisa",
              url: "https://example.test/people/monalisa",
            },
          ],
          assignees: ["justin"],
          alwaysButtons: [],
          author: "octocat",
          checksStatus: "passed",
          isDraft: false,
          itemKind: "pull_request",
          labels: [{ color: "1d76db", name: "reviewed" }],
          mergeStatus: "mergeable",
          number: 42,
          repository: "example/project",
          source: "github",
          state: "open",
          targetBranch: "main",
          title: "Keep the dashboard dense",
          updatedAt: "2026-08-26T12:00:00Z",
          url: "https://app.graphite.com/github/pr/example/project/42",
        },
      ],
      itemsPerPage: 6,
      lastSuccessfulRefresh: 1_787_742_000_000,
      refreshSeconds: 300,
      stale: false,
      status: "idle",
      title: "Reviews requested",
    },
  ],
};

const autocompleteProps = {
  autocompletes: {},
  cancelAutocomplete: vi.fn().mockResolvedValue(undefined),
  requestAutocomplete: vi.fn().mockResolvedValue(undefined),
};

function dashboardWithActions(): DashboardSnapshot {
  const actionsDashboard = structuredClone(dashboard);
  actionsDashboard.sections[0].items[0].alwaysButtons = [
    {
      disabled: false,
      index: 0,
      label: "Review",
      prompt: {
        default: "start in a new work tree",
        label: "Review focus",
        placeholder: "Add areas to inspect",
      },
      title:
        "codex exec '/review https://example.test/pull/42 start in a new work tree'",
      url: null,
    },
    {
      disabled: false,
      index: 1,
      label: "Check",
      prompt: null,
      title: "gh pr checks https://example.test/pull/42",
      url: null,
    },
  ];
  actionsDashboard.sections[0].items[0].advancedButtons = [
    {
      disabled: false,
      index: 0,
      label: "Open",
      prompt: null,
      title: "https://example.test/pull/42",
      url: "https://example.test/pull/42",
    },
  ];
  return actionsDashboard;
}

describe("DashboardPage", () => {
  it("shows dense actionable section state and requests a typed refresh", async () => {
    const refreshSection = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
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
    expect(screen.getByText("main").getAttribute("title")).toBe(
      "Target branch: main",
    );
    expect(
      screen.getByRole("img", { name: "CI checks passed" }).textContent,
    ).toBe("✓");
    expect(screen.queryByRole("img", { name: "Merge conflicts" })).toBeNull();
    expect(document.querySelector(".section-updated")?.textContent).toMatch(
      /^Updated /,
    );
    expect(screen.getByRole("status", { name: "Connected" }).textContent).toBe(
      "↔",
    );
    expect(screen.queryByText("Connected")).toBeNull();
    expect(screen.getByRole("link", { name: "Options" }).textContent).toBe("⚙︎");
    expect(screen.queryByText("Options")).toBeNull();
    expect(screen.getByRole("link", { name: "Command logs" }).textContent).toBe(
      "▤",
    );
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
    expect(
      screen.getByRole("link", { name: "@hubot" }).getAttribute("href"),
    ).toBe("https://example.test/people/hubot");
    expect(
      screen.getByRole("link", { name: "@hubot" }).parentElement?.textContent,
    ).toBe("✓ @hubot");
    const approvers = screen
      .getByRole("link", { name: "@hubot" })
      .closest(".item-approvers");
    expect(
      Array.from(approvers?.children ?? [], (approver) => approver.textContent),
    ).toEqual(["✓ @hubot", "✓ @monalisa"]);
    expect(approvers?.parentElement).toBe(reference.closest("li"));
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
        {...autocompleteProps}
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

  it("refreshes only while the tab is visible and the section is expanded", async () => {
    let visibilityState: DocumentVisibilityState = "hidden";
    const visibility = vi
      .spyOn(document, "visibilityState", "get")
      .mockImplementation(() => visibilityState);
    const refreshSection = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();

    try {
      render(
        <DashboardPage
          activeConfiguration={configuration}
          {...autocompleteProps}
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

      expect(refreshSection).not.toHaveBeenCalled();
      visibilityState = "visible";
      fireEvent(document, new Event("visibilitychange"));
      await waitFor(() => expect(refreshSection).toHaveBeenCalledTimes(1));

      await user.click(
        screen.getByRole("button", { name: "Collapse Reviews requested" }),
      );
      expect(screen.queryByText("Keep the dashboard dense")).toBeNull();
      expect(screen.queryByRole("button", { name: "Refresh" })).toBeNull();
      refreshSection.mockClear();

      visibilityState = "hidden";
      fireEvent(document, new Event("visibilitychange"));
      visibilityState = "visible";
      fireEvent(document, new Event("visibilitychange"));
      expect(refreshSection).not.toHaveBeenCalled();

      await user.click(
        screen.getByRole("button", { name: "Expand Reviews requested" }),
      );
      await waitFor(() => expect(refreshSection).toHaveBeenCalledTimes(1));
    } finally {
      visibility.mockRestore();
    }
  });

  it("starts a configured section collapsed and refreshes after expansion", async () => {
    const collapsedDashboard = structuredClone(dashboard);
    collapsedDashboard.sections[0].collapsed = true;
    collapsedDashboard.sections[0].items = [];
    collapsedDashboard.sections[0].lastSuccessfulRefresh = null;
    const refreshSection = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={collapsedDashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={refreshSection}
        run={null}
        runButton={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Expand Reviews requested" }),
    ).toBeTruthy();
    expect(document.querySelector(".section-count")).toBeNull();
    expect(screen.queryByText("Keep the dashboard dense")).toBeNull();
    expect(refreshSection).not.toHaveBeenCalled();

    await user.click(
      screen.getByRole("button", { name: "Expand Reviews requested" }),
    );
    await waitFor(() => expect(refreshSection).toHaveBeenCalledTimes(1));
  });

  it("opens an item by clicking otherwise inactive row space", async () => {
    const open = vi.spyOn(window, "open").mockImplementation(() => null);
    const user = userEvent.setup();
    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={dashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn().mockResolvedValue(undefined)}
        run={null}
        runButton={vi.fn()}
      />,
    );

    const row = screen.getByText("Keep the dashboard dense").closest("li");
    expect(row).not.toBeNull();
    await user.click(row!);

    expect(open).toHaveBeenCalledWith(
      "https://app.graphite.com/github/pr/example/project/42",
      "_blank",
      "noopener,noreferrer",
    );
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
      { ...dashboard.sections[0].items[0], number: 5, state: "approved" },
    ];

    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
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
    expect(screen.getByRole("img", { name: "Approved" }).textContent).toBe("");
  });

  it("shows CI and merge status icons", () => {
    const checksDashboard = structuredClone(dashboard);
    checksDashboard.sections[0].items = [
      {
        ...dashboard.sections[0].items[0],
        checksStatus: "failed",
        mergeStatus: "conflicting",
        number: 1,
      },
      {
        ...dashboard.sections[0].items[0],
        checksStatus: "passed",
        mergeStatus: "mergeable",
        number: 2,
      },
      {
        ...dashboard.sections[0].items[0],
        checksStatus: "pending",
        mergeStatus: "unknown",
        number: 3,
      },
    ];

    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={checksDashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={null}
        runButton={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("img", { name: "CI checks failed" }).textContent,
    ).toBe("✕");
    expect(
      screen.getByRole("img", { name: "CI checks passed" }).textContent,
    ).toBe("✓");
    expect(
      screen.getByRole("img", { name: "CI checks pending" }).textContent,
    ).toBe("◷");
    expect(
      screen.getByRole("img", { name: "Merge conflicts" }).textContent,
    ).toBe("✕");
  });

  it("identifies open and closed issue circles separately", () => {
    const issuesDashboard = structuredClone(dashboard);
    issuesDashboard.sections[0].items = [
      {
        ...issuesDashboard.sections[0].items[0],
        isDraft: null,
        itemKind: "issue",
      },
      {
        ...issuesDashboard.sections[0].items[0],
        isDraft: null,
        itemKind: "issue",
        number: 43,
        state: "closed",
      },
    ];

    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={issuesDashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={null}
        runButton={vi.fn()}
      />,
    );

    expect(screen.getByRole("img", { name: "Open" }).dataset.itemKind).toBe(
      "issue",
    );
    expect(screen.getByRole("img", { name: "Closed" }).dataset.itemKind).toBe(
      "issue",
    );
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
        {...autocompleteProps}
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
    let visibilityState: DocumentVisibilityState = "visible";
    const visibility = vi
      .spyOn(document, "visibilityState", "get")
      .mockImplementation(() => visibilityState);
    try {
      vi.setSystemTime(new Date("2026-08-26T12:00:00Z"));
      const timesDashboard = structuredClone(dashboard);
      const sectionRefreshDate = new Date("2026-08-26T11:50:00Z");
      timesDashboard.sections[0].lastSuccessfulRefresh =
        sectionRefreshDate.getTime();
      timesDashboard.sections[0].items = [
        {
          ...dashboard.sections[0].items[0],
          number: 1,
          updatedAt: "2026-08-26T11:59:45Z",
        },
        {
          ...dashboard.sections[0].items[0],
          number: 2,
          updatedAt: "2026-08-26T11:30:00Z",
        },
        {
          ...dashboard.sections[0].items[0],
          number: 3,
          updatedAt: "2026-08-26T08:30:00Z",
        },
        {
          ...dashboard.sections[0].items[0],
          number: 4,
          updatedAt: "2026-08-23T08:00:00Z",
        },
        {
          ...dashboard.sections[0].items[0],
          number: 5,
          updatedAt: "2026-08-10T12:00:00Z",
        },
      ];

      render(
        <DashboardPage
          activeConfiguration={configuration}
          {...autocompleteProps}
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

      expect(screen.getByText("15 seconds ago")).toBeTruthy();
      expect(screen.getByText("30 minutes ago")).toBeTruthy();
      expect(screen.getByText("4 hours ago")).toBeTruthy();
      expect(screen.getByText("3 days ago")).toBeTruthy();
      const sectionTime = screen.getByTitle(sectionRefreshDate.toString());
      expect(sectionTime.textContent).toBe("10 minutes ago");
      expect(sectionTime.parentElement?.textContent).toBe(
        "Updated 10 minutes ago",
      );
      expect(sectionTime.getAttribute("datetime")).toBe(
        sectionRefreshDate.toISOString(),
      );

      act(() => vi.advanceTimersByTime(1_000));

      expect(screen.getByText("15 seconds ago")).toBeTruthy();

      act(() => vi.advanceTimersByTime(29_000));

      expect(screen.getByText("45 seconds ago")).toBeTruthy();

      act(() => vi.advanceTimersByTime(30_000));

      expect(screen.getByText("1 minute ago")).toBeTruthy();
      expect(screen.getByText("31 minutes ago")).toBeTruthy();
      expect(sectionTime.textContent).toBe("11 minutes ago");

      visibilityState = "hidden";
      fireEvent(document, new Event("visibilitychange"));
      act(() => vi.advanceTimersByTime(60_000));
      expect(screen.getByText("31 minutes ago")).toBeTruthy();

      visibilityState = "visible";
      fireEvent(document, new Event("visibilitychange"));
      expect(screen.getByText("32 minutes ago")).toBeTruthy();
      expect(sectionTime.textContent).toBe("12 minutes ago");
      const olderDate = new Date("2026-08-10T12:00:00Z");
      const olderTime = screen.getByTitle(olderDate.toString());
      expect(olderTime.textContent).toBe(
        new Intl.DateTimeFormat(undefined, {
          day: "numeric",
          month: "short",
        }).format(olderDate),
      );
      expect(olderTime.getAttribute("datetime")).toBe(
        new Date("2026-08-10T12:00:00Z").toISOString(),
      );
    } finally {
      visibility.mockRestore();
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
        {...autocompleteProps}
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
        {...autocompleteProps}
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
    const actionsDashboard = dashboardWithActions();
    const previewButton = vi
      .fn()
      .mockResolvedValue(
        "codex exec '/review https://example.test/pull/42 start in a new work tree'",
      );
    const runButton = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
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
    await screen.findByText(
      "codex exec '/review https://example.test/pull/42 start in a new work tree'",
    );
    expect(runButton).not.toHaveBeenCalled();
    const prompt = screen.getByLabelText("Review focus") as HTMLInputElement;
    expect(prompt.placeholder).toBe("Add areas to inspect");
    expect(prompt.value).toBe("start in a new work tree");
    const workingDirectory = screen.getByLabelText(
      "Working directory",
    ) as HTMLSelectElement;
    expect(workingDirectory.value).toBe("/workspace/first");
    expect(previewButton).toHaveBeenLastCalledWith(
      "reviews",
      actionsDashboard.sections[0].items[0],
      "always",
      0,
      "start in a new work tree",
      "/workspace/first",
    );

    previewButton.mockResolvedValue(
      "codex exec '/review https://example.test/pull/42 focus on tests'",
    );
    await user.selectOptions(workingDirectory, "/workspace/second");
    await user.clear(prompt);
    await user.type(prompt, "focus on tests");
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
      "/workspace/second",
    );

    previewButton.mockResolvedValue(
      "gh pr checks https://example.test/pull/42",
    );
    await user.click(screen.getByRole("button", { name: "Check" }));
    await screen.findByText("gh pr checks https://example.test/pull/42");
    expect(screen.queryByLabelText("Review focus")).toBeNull();
    expect(previewButton).toHaveBeenLastCalledWith(
      "reviews",
      actionsDashboard.sections[0].items[0],
      "always",
      1,
      null,
      "/workspace/first",
    );
    expect(
      (screen.getByLabelText("Working directory") as HTMLSelectElement).value,
    ).toBe("/workspace/first");
    expect(runButton).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "Run" }));
    expect(runButton).toHaveBeenLastCalledWith(
      "reviews",
      actionsDashboard.sections[0].items[0],
      "always",
      1,
      null,
      "/workspace/first",
    );

    await user.click(
      screen.getByLabelText("More actions for example/project#42"),
    );
    expect(
      screen.getByRole("link", { name: "Open" }).getAttribute("href"),
    ).toBe("https://example.test/pull/42");
  });

  it("debounces configured autocomplete and applies only the current suggestion", async () => {
    const actionsDashboard = dashboardWithActions();
    const autocompleteConfiguration = {
      ...configuration,
      autocomplete: {
        debounceMilliseconds: 300,
        minimumCharacters: 3,
      },
    };
    const cancelAutocomplete = vi.fn().mockResolvedValue(undefined);
    const previewButton = vi.fn().mockResolvedValue("review focus on tests");
    const refreshSection = vi.fn().mockResolvedValue(undefined);
    const requestAutocomplete = vi.fn().mockResolvedValue(undefined);
    const runButton = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    const view = (
      autocompletes: Readonly<Record<string, AutocompleteSnapshot>>,
    ) => (
      <DashboardPage
        activeConfiguration={autocompleteConfiguration}
        autocompletes={autocompletes}
        cancelAutocomplete={cancelAutocomplete}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={actionsDashboard}
        dismissRun={vi.fn()}
        previewButton={previewButton}
        refreshSection={refreshSection}
        requestAutocomplete={requestAutocomplete}
        run={null}
        runButton={runButton}
      />
    );
    const { rerender } = render(view({}));

    await user.click(screen.getByRole("button", { name: "Review" }));
    const prompt = screen.getByLabelText("Review focus");
    await user.clear(prompt);
    await user.type(prompt, "focus on tests");
    await waitFor(() => expect(requestAutocomplete).toHaveBeenCalledTimes(1));
    const request = requestAutocomplete.mock.calls[0][0];
    expect(request).toMatchObject({
      buttonIndex: 0,
      buttonList: "always",
      configurationRevision: 7,
      draft: "focus on tests",
      sectionId: "reviews",
      selectionEnd: 14,
      selectionStart: 14,
    });

    rerender(
      view({
        [request.editorId]: {
          autocompleteId: "stale-autocomplete",
          editorId: request.editorId,
          error: null,
          status: "completed",
          suggestion: "replace the current draft with a stale suggestion",
        },
      }),
    );
    expect(
      screen.queryByText("replace the current draft with a stale suggestion"),
    ).toBeNull();

    rerender(
      view({
        [request.editorId]: {
          autocompleteId: request.autocompleteId,
          editorId: request.editorId,
          error: null,
          status: "completed",
          suggestion: "focus on tests and boundaries",
        },
      }),
    );
    expect(screen.getByText("focus on tests and boundaries")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Use suggestion" }));

    expect(
      (screen.getByLabelText("Review focus") as HTMLInputElement).value,
    ).toBe("focus on tests and boundaries");
    expect(requestAutocomplete).toHaveBeenCalledTimes(1);
  });

  it("shows live run output and requests cancellation", async () => {
    const cancelRun = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
        cancelRun={cancelRun}
        connectionError={null}
        connectionStatus="connected"
        dashboard={dashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={{
          createdAt: 1_787_742_000_000,
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

  it("shows detached commands as started without retaining cancellation", () => {
    const output =
      "Opened draft.\nStarted Codex thread 01a0597a-ab15-7a61-a2f5-e031e9fc2a20.\nReady.";
    render(
      <DashboardPage
        activeConfiguration={configuration}
        {...autocompleteProps}
        cancelRun={vi.fn()}
        connectionError={null}
        connectionStatus="connected"
        dashboard={dashboard}
        dismissRun={vi.fn()}
        previewButton={vi.fn()}
        refreshSection={vi.fn()}
        run={{
          createdAt: 1_787_742_000_000,
          exitCode: null,
          id: "run-9",
          label: "Review",
          output,
          preview: "open a configured application",
          status: "started",
        }}
        runButton={vi.fn()}
      />,
    );

    expect(document.querySelector(".run-status")?.textContent).toMatch(
      /^Started/,
    );
    const threadLink = screen.getByRole("link", {
      name: "Started Codex thread 01a0597a-ab15-7a61-a2f5-e031e9fc2a20.",
    });
    expect(threadLink.getAttribute("href")).toBe(
      "codex://threads/01a0597a-ab15-7a61-a2f5-e031e9fc2a20",
    );
    expect(document.querySelector(".run-output")?.textContent).toBe(output);
    expect(screen.queryByRole("link", { name: "Opened draft." })).toBeNull();
    expect(screen.queryByRole("link", { name: "Ready." })).toBeNull();
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  });
});
