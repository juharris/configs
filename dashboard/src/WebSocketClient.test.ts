import { afterEach, describe, expect, it, vi } from "vitest";

import type { ActiveConfiguration, OptifySetup } from "./generated/transport";
import { WebSocketClient } from "./WebSocketClient";

const setup: OptifySetup = {
  configDirectories: ["/dashboard/configs"],
  features: ["dashboard"],
};

const configuration: ActiveConfiguration = {
  revision: 3,
  setup,
  theme: "dark",
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
      onConfiguration,
      onError: vi.fn(),
      onStatus,
    });

    client.start();
    await vi.waitFor(() => expect(FakeWebSocket.instances).toHaveLength(1));
    const socket = FakeWebSocket.instances[0];
    socket.open();
    expect(JSON.parse(socket.sent[0])).toEqual({
      lastEventSequence: null,
      protocolVersion: 1,
      token: "token-1",
      type: "authenticate",
    });
    socket.receive({
      activeConfiguration: null,
      connectionId: "connection-1",
      eventSequence: 0,
      protocolVersion: 1,
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
    expect(onConfiguration).not.toHaveBeenCalled();
    expect(onStatus).toHaveBeenCalledWith("connected");
    client.stop();
  });

  it("synchronizes saved setup and consumes configuration events", async () => {
    stubBootstrap("token-2");
    vi.stubGlobal("WebSocket", FakeWebSocket);
    const onConfiguration = vi.fn();
    const client = new WebSocketClient({
      getSetup: () => setup,
      onConfiguration,
      onError: vi.fn(),
      onStatus: vi.fn(),
    });

    client.start();
    await vi.waitFor(() => expect(FakeWebSocket.instances).toHaveLength(1));
    const socket = FakeWebSocket.instances[0];
    socket.open();
    socket.receive({
      activeConfiguration: null,
      connectionId: "connection-1",
      eventSequence: 0,
      protocolVersion: 1,
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

    const reloaded = { ...configuration, revision: 4 };
    socket.receive({
      event: { configuration: reloaded, type: "configuration_reloaded" },
      eventId: "event-1",
      sequence: 1,
      type: "event",
    });
    expect(onConfiguration).toHaveBeenLastCalledWith(reloaded);
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
      json: vi.fn().mockResolvedValue({ protocolVersion: 1, token }),
      ok: true,
      status: 200,
    }),
  );
}
