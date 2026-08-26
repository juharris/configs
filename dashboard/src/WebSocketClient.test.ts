import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  ActiveConfiguration,
  DashboardSnapshot,
  OptifySetup,
} from "./generated/transport";
import { WebSocketClient } from "./WebSocketClient";

const setup: OptifySetup = {
  configDirectories: ["/dashboard/configs"],
  features: ["dashboard"],
};

const configuration: ActiveConfiguration = {
  autocomplete: {
    debounceMilliseconds: 300,
    minimumCharacters: 20,
  },
  revision: 3,
  setup,
  theme: "dark",
};

const dashboard: DashboardSnapshot = {
  configurationRevision: 3,
  sections: [],
};

describe("WebSocketClient", () => {
  afterEach(() => {
    FakeWebSocket.instances = [];
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("authenticates and correlates an apply request", async () => {
    stubBootstrap("token-1");
    vi.stubGlobal("WebSocket", FakeWebSocket);
    const onConfiguration = vi.fn();
    const onStatus = vi.fn();
    const client = new WebSocketClient({
      getSetup: () => null,
      onAutocomplete: vi.fn(),
      onConfiguration,
      onDashboard: vi.fn(),
      onError: vi.fn(),
      onRun: vi.fn(),
      onStatus,
    });

    client.start();
    await vi.waitFor(() => expect(FakeWebSocket.instances).toHaveLength(1));
    const socket = FakeWebSocket.instances[0];
    socket.open();
    expect(JSON.parse(socket.sent[0])).toEqual({
      lastEventSequence: null,
      protocolVersion: 7,
      token: "token-1",
      type: "authenticate",
    });
    socket.receive({
      activeConfiguration: null,
      connectionId: "connection-1",
      dashboard: null,
      eventSequence: 0,
      protocolVersion: 7,
      setupStatus: "required",
      type: "connection_ready",
    });

    const applied = client.applySetup(setup);
    const request = JSON.parse(socket.sent[1]) as { requestId: string };
    socket.receive({
      requestId: request.requestId,
      response: {
        configuration,
        type: "optify_setup_applied",
      },
      type: "response",
    });

    await expect(applied).resolves.toEqual(configuration);

    const refreshed = client.refreshSection(3, "requested_reviews");
    const refreshRequest = JSON.parse(socket.sent[2]) as { requestId: string };
    socket.receive({
      requestId: refreshRequest.requestId,
      response: {
        refresh: {
          coalesced: false,
          sectionId: "requested_reviews",
          status: "refreshing",
        },
        type: "section_refresh_accepted",
      },
      type: "response",
    });
    await expect(refreshed).resolves.toEqual({
      coalesced: false,
      sectionId: "requested_reviews",
      status: "refreshing",
    });

    const run = {
      exitCode: null,
      id: "run-1",
      label: "Review",
      output: "",
      preview: "codex exec '/review https://example.test/pull/42'",
      status: "queued" as const,
    };
    const previewed = client.previewButton(
      0,
      "always",
      3,
      { number: 42, repository: "shop/world", source: "github" },
      "focus on tests",
      "requested_reviews",
    );
    const previewRequest = JSON.parse(socket.sent[3]) as {
      requestId: string;
    };
    socket.receive({
      requestId: previewRequest.requestId,
      response: { preview: run.preview, type: "button_previewed" },
      type: "response",
    });
    await expect(previewed).resolves.toBe(run.preview);

    const started = client.runButton(
      0,
      "always",
      3,
      { number: 42, repository: "shop/world", source: "github" },
      "focus on tests",
      "requested_reviews",
    );
    const runRequest = JSON.parse(socket.sent[4]) as { requestId: string };
    socket.receive({
      requestId: runRequest.requestId,
      response: { run, type: "button_run_accepted" },
      type: "response",
    });
    await expect(started).resolves.toEqual(run);

    const cancelled = client.cancelRun("run-1");
    const cancelRequest = JSON.parse(socket.sent[5]) as { requestId: string };
    socket.receive({
      requestId: cancelRequest.requestId,
      response: { runId: "run-1", type: "run_cancellation_accepted" },
      type: "response",
    });
    await expect(cancelled).resolves.toBeUndefined();

    const autocompleteRequested = client.requestAutocomplete({
      autocompleteId: "autocomplete-1",
      buttonIndex: 0,
      buttonList: "always",
      configurationRevision: 3,
      draft: "focus on tests",
      editorId: "editor-1",
      item: { number: 42, repository: "shop/world", source: "github" },
      sectionId: "requested_reviews",
      selectionEnd: 14,
      selectionStart: 14,
    });
    const autocompleteRequest = JSON.parse(socket.sent[6]) as {
      request: { type: string };
      requestId: string;
    };
    expect(autocompleteRequest.request.type).toBe("request_autocomplete");
    socket.receive({
      requestId: autocompleteRequest.requestId,
      response: {
        autocompleteId: "autocomplete-1",
        editorId: "editor-1",
        type: "autocomplete_request_accepted",
      },
      type: "response",
    });
    await expect(autocompleteRequested).resolves.toBeUndefined();

    const autocompleteCancelled = client.cancelAutocomplete("editor-1");
    const autocompleteCancellation = JSON.parse(socket.sent[7]) as {
      requestId: string;
    };
    socket.receive({
      requestId: autocompleteCancellation.requestId,
      response: {
        editorId: "editor-1",
        type: "autocomplete_cancellation_accepted",
      },
      type: "response",
    });
    await expect(autocompleteCancelled).resolves.toBeUndefined();
    expect(onConfiguration).not.toHaveBeenCalled();
    expect(onStatus).toHaveBeenCalledWith("connected");
    client.stop();
  });

  it("synchronizes saved setup and consumes configuration events", async () => {
    stubBootstrap("token-2");
    vi.stubGlobal("WebSocket", FakeWebSocket);
    const onConfiguration = vi.fn();
    const onDashboard = vi.fn();
    const onAutocomplete = vi.fn();
    const onRun = vi.fn();
    const client = new WebSocketClient({
      getSetup: () => setup,
      onAutocomplete,
      onConfiguration,
      onDashboard,
      onError: vi.fn(),
      onRun,
      onStatus: vi.fn(),
    });

    client.start();
    await vi.waitFor(() => expect(FakeWebSocket.instances).toHaveLength(1));
    const socket = FakeWebSocket.instances[0];
    socket.open();
    socket.receive({
      activeConfiguration: null,
      connectionId: "connection-1",
      dashboard,
      eventSequence: 0,
      protocolVersion: 7,
      setupStatus: "required",
      type: "connection_ready",
    });
    const request = JSON.parse(socket.sent[1]) as { requestId: string };
    socket.receive({
      requestId: request.requestId,
      response: {
        configuration,
        type: "optify_setup_applied",
      },
      type: "response",
    });
    await vi.waitFor(() =>
      expect(onConfiguration).toHaveBeenCalledWith(configuration),
    );
    expect(onDashboard).toHaveBeenCalledWith(dashboard);

    const reloaded = { ...configuration, revision: 4 };
    socket.receive({
      event: { configuration: reloaded, type: "configuration_reloaded" },
      eventId: "event-1",
      sequence: 1,
      type: "event",
    });
    expect(onConfiguration).toHaveBeenLastCalledWith(reloaded);

    const autocomplete = {
      autocompleteId: "autocomplete-1",
      editorId: "editor-1",
      error: null,
      status: "completed" as const,
      suggestion: "Focus on the boundary case.",
    };
    socket.receive({
      event: { autocomplete, type: "autocomplete_updated" },
      eventId: "event-2",
      sequence: 2,
      type: "event",
    });
    expect(onAutocomplete).toHaveBeenCalledWith(autocomplete);

    const updatedDashboard = { ...dashboard, configurationRevision: 4 };
    socket.receive({
      event: { dashboard: updatedDashboard, type: "dashboard_updated" },
      eventId: "event-3",
      sequence: 3,
      type: "event",
    });
    expect(onDashboard).toHaveBeenLastCalledWith(updatedDashboard);

    const run = {
      exitCode: 0,
      id: "run-1",
      label: "Review",
      output: "Done",
      preview: "codex exec review",
      status: "completed" as const,
    };
    socket.receive({
      event: { run, type: "run_updated" },
      eventId: "event-4",
      sequence: 4,
      type: "event",
    });
    expect(onRun).toHaveBeenCalledWith(run);
    client.stop();
  });
});

class FakeWebSocket {
  static readonly OPEN = 1;
  static instances: FakeWebSocket[] = [];

  readonly listeners = new Map<string, Array<(event: MessageEvent) => void>>();
  readonly sent: string[] = [];
  readyState = 0;

  constructor(readonly url: string) {
    FakeWebSocket.instances.push(this);
  }

  addEventListener(type: string, listener: (event: MessageEvent) => void) {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  close() {
    if (this.readyState === 3) {
      return;
    }
    this.readyState = 3;
    this.emit("close", new MessageEvent("close"));
  }

  open() {
    this.readyState = FakeWebSocket.OPEN;
    this.emit("open", new MessageEvent("open"));
  }

  receive(message: object) {
    this.emit(
      "message",
      new MessageEvent("message", { data: JSON.stringify(message) }),
    );
  }

  send(message: string) {
    this.sent.push(message);
  }

  private emit(type: string, event: MessageEvent) {
    this.listeners.get(type)?.forEach((listener) => listener(event));
  }
}

function stubBootstrap(token: string) {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      json: vi.fn().mockResolvedValue({ protocolVersion: 7, token }),
      ok: true,
      status: 200,
    }),
  );
}
