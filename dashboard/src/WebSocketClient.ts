import type {
  ActiveConfiguration,
  AutocompleteSnapshot,
  BootstrapResponse,
  ButtonList,
  ClientMessage,
  ClientRequest,
  DashboardSnapshot,
  ItemReference,
  OptifySetup,
  RunSnapshot,
  SectionRefresh,
  ServerMessage,
  ServerResponse,
} from "./generated/transport";

const PROTOCOL_VERSION = 7;
const RECONNECT_DELAYS = [250, 500, 1_000, 2_000, 5_000] as const;
const REQUEST_TIMEOUT = 10_000;

export type ConnectionStatus =
  "connected" | "connecting" | "disconnected" | "reconnecting";

export type AutocompleteRequestParameters = {
  autocompleteId: string;
  buttonIndex: number;
  buttonList: ButtonList;
  configurationRevision: number;
  draft: string;
  editorId: string;
  item: ItemReference;
  sectionId: string;
  selectionEnd: number;
  selectionStart: number;
};

type PendingRequest = {
  reject: (error: Error) => void;
  resolve: (response: ServerResponse) => void;
  timeout: number;
};

type WebSocketClientOptions = {
  getSetup: () => OptifySetup | null;
  onAutocomplete: (autocomplete: AutocompleteSnapshot) => void;
  onConfiguration: (configuration: ActiveConfiguration) => void;
  onDashboard: (dashboard: DashboardSnapshot) => void;
  onError: (message: string) => void;
  onRun: (run: RunSnapshot) => void;
  onStatus: (status: ConnectionStatus) => void;
};

/** Owns bootstrap authentication, request correlation, and reconnect behavior. */
export class WebSocketClient {
  readonly #getSetup: () => OptifySetup | null;
  readonly #onAutocomplete: (autocomplete: AutocompleteSnapshot) => void;
  readonly #onConfiguration: (configuration: ActiveConfiguration) => void;
  readonly #onDashboard: (dashboard: DashboardSnapshot) => void;
  readonly #onError: (message: string) => void;
  readonly #onRun: (run: RunSnapshot) => void;
  readonly #onStatus: (status: ConnectionStatus) => void;
  readonly #pending = new Map<string, PendingRequest>();
  #active = false;
  #attempt = 0;
  #generation = 0;
  #lastEventSequence: number | null = null;
  #ready = false;
  #reconnectTimer: number | null = null;
  #requestSequence = 0;
  #socket: WebSocket | null = null;

  constructor(options: WebSocketClientOptions) {
    this.#getSetup = options.getSetup;
    this.#onAutocomplete = options.onAutocomplete;
    this.#onConfiguration = options.onConfiguration;
    this.#onDashboard = options.onDashboard;
    this.#onError = options.onError;
    this.#onRun = options.onRun;
    this.#onStatus = options.onStatus;
  }

  applySetup(setup: OptifySetup): Promise<ActiveConfiguration> {
    return this.#sendRequest({ setup, type: "apply_optify_setup" }).then(
      (response) => {
        if (response.type !== "optify_setup_applied") {
          throw new Error(
            "The dashboard service returned an unexpected response.",
          );
        }
        return response.configuration;
      },
    );
  }

  cancelRun(runId: string): Promise<void> {
    return this.#sendRequest({ runId, type: "cancel_run" }).then((response) => {
      if (response.type !== "run_cancellation_accepted") {
        throw new Error(
          "The dashboard service returned an unexpected response.",
        );
      }
    });
  }

  cancelAutocomplete(editorId: string): Promise<void> {
    return this.#sendRequest({ editorId, type: "cancel_autocomplete" }).then(
      (response) => {
        if (
          response.type !== "autocomplete_cancellation_accepted" ||
          response.editorId !== editorId
        ) {
          throw new Error(
            "The dashboard service returned an unexpected response.",
          );
        }
      },
    );
  }

  previewButton(
    buttonIndex: number,
    buttonList: ButtonList,
    configurationRevision: number,
    item: ItemReference,
    prompt: string | null,
    sectionId: string,
  ): Promise<string> {
    return this.#sendRequest({
      buttonIndex,
      buttonList,
      configurationRevision,
      item,
      prompt,
      sectionId,
      type: "preview_button",
    }).then((response) => {
      if (response.type !== "button_previewed") {
        throw new Error(
          "The dashboard service returned an unexpected response.",
        );
      }
      return response.preview;
    });
  }

  refreshSection(
    configurationRevision: number,
    sectionId: string,
  ): Promise<SectionRefresh> {
    return this.#sendRequest({
      configurationRevision,
      sectionId,
      type: "refresh_section",
    }).then((response) => {
      if (response.type !== "section_refresh_accepted") {
        throw new Error(
          "The dashboard service returned an unexpected response.",
        );
      }
      return response.refresh;
    });
  }

  requestAutocomplete(
    parameters: AutocompleteRequestParameters,
  ): Promise<void> {
    return this.#sendRequest({
      ...parameters,
      type: "request_autocomplete",
    }).then((response) => {
      if (
        response.type !== "autocomplete_request_accepted" ||
        response.autocompleteId !== parameters.autocompleteId ||
        response.editorId !== parameters.editorId
      ) {
        throw new Error(
          "The dashboard service returned an unexpected response.",
        );
      }
    });
  }

  runButton(
    buttonIndex: number,
    buttonList: ButtonList,
    configurationRevision: number,
    item: ItemReference,
    prompt: string | null,
    sectionId: string,
  ): Promise<RunSnapshot> {
    return this.#sendRequest({
      buttonIndex,
      buttonList,
      configurationRevision,
      item,
      prompt,
      sectionId,
      type: "run_button",
    }).then((response) => {
      if (response.type !== "button_run_accepted") {
        throw new Error(
          "The dashboard service returned an unexpected response.",
        );
      }
      return response.run;
    });
  }

  #sendRequest(request: ClientRequest): Promise<ServerResponse> {
    if (
      !this.#ready ||
      this.#socket === null ||
      this.#socket.readyState !== WebSocket.OPEN
    ) {
      return Promise.reject(
        new Error("The dashboard service is reconnecting. Try again shortly."),
      );
    }
    const requestId = `request-${(++this.#requestSequence).toString()}`;
    const message: ClientMessage = {
      request,
      requestId,
      type: "request",
    };
    return new Promise((resolve, reject) => {
      const timeout = window.setTimeout(() => {
        this.#pending.delete(requestId);
        reject(new Error("The dashboard service did not respond in time."));
      }, REQUEST_TIMEOUT);
      this.#pending.set(requestId, { reject, resolve, timeout });
      try {
        this.#socket?.send(JSON.stringify(message));
      } catch (error) {
        window.clearTimeout(timeout);
        this.#pending.delete(requestId);
        reject(
          error instanceof Error
            ? error
            : new Error("Could not send the dashboard request."),
        );
      }
    });
  }

  start(): void {
    if (this.#active) {
      return;
    }
    this.#active = true;
    this.#attempt = 0;
    this.#generation += 1;
    this.#connect(this.#generation);
  }

  stop(): void {
    this.#active = false;
    this.#generation += 1;
    this.#ready = false;
    if (this.#reconnectTimer !== null) {
      window.clearTimeout(this.#reconnectTimer);
      this.#reconnectTimer = null;
    }
    this.#rejectPending("The dashboard connection closed.");
    this.#socket?.close();
    this.#socket = null;
    this.#onStatus("disconnected");
  }

  async #connect(generation: number): Promise<void> {
    this.#onStatus(this.#attempt === 0 ? "connecting" : "reconnecting");
    try {
      const response = await fetch("/bootstrap", {
        cache: "no-store",
        headers: { Accept: "application/json" },
      });
      if (!response.ok) {
        throw new Error(
          `Dashboard bootstrap failed with HTTP ${response.status.toString()}.`,
        );
      }
      const bootstrap = (await response.json()) as BootstrapResponse;
      if (bootstrap.protocolVersion !== PROTOCOL_VERSION) {
        throw new Error(
          "The browser and dashboard service protocol versions do not match.",
        );
      }
      if (!this.#active || generation !== this.#generation) {
        return;
      }
      const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
      const socket = new WebSocket(`${protocol}//${window.location.host}/ws`);
      this.#socket = socket;
      socket.addEventListener("close", () => this.#handleDisconnect(socket));
      socket.addEventListener("message", (event) =>
        this.#handleMessage(event.data),
      );
      socket.addEventListener("open", () => {
        const message: ClientMessage = {
          lastEventSequence: this.#lastEventSequence,
          protocolVersion: PROTOCOL_VERSION,
          token: bootstrap.token,
          type: "authenticate",
        };
        socket.send(JSON.stringify(message));
      });
    } catch (error) {
      if (!this.#active || generation !== this.#generation) {
        return;
      }
      this.#onError(
        error instanceof Error
          ? error.message
          : "Could not connect to the dashboard service.",
      );
      this.#scheduleReconnect();
    }
  }

  #handleDisconnect(socket: WebSocket): void {
    if (socket !== this.#socket) {
      return;
    }
    this.#ready = false;
    this.#socket = null;
    this.#rejectPending("The dashboard connection was interrupted.");
    this.#scheduleReconnect();
  }

  #handleMessage(data: unknown): void {
    if (typeof data !== "string") {
      this.#onError("The dashboard service sent an unsupported message.");
      return;
    }
    let message: ServerMessage;
    try {
      message = JSON.parse(data) as ServerMessage;
    } catch {
      this.#onError("The dashboard service sent invalid JSON.");
      return;
    }

    switch (message.type) {
      case "connection_ready":
        if (message.protocolVersion !== PROTOCOL_VERSION) {
          this.#onError(
            "The browser and dashboard service protocol versions do not match.",
          );
          this.#socket?.close();
          return;
        }
        this.#attempt = 0;
        this.#lastEventSequence = Math.max(
          this.#lastEventSequence ?? 0,
          message.eventSequence,
        );
        this.#ready = true;
        this.#onStatus("connected");
        if (message.dashboard !== null) {
          this.#onDashboard(message.dashboard);
        }
        this.#synchronizeSetup(message.activeConfiguration);
        return;
      case "error": {
        const error = new Error(
          message.field === null
            ? message.message
            : `${message.field}: ${message.message}`,
        );
        if (message.requestId === null) {
          this.#onError(error.message);
          return;
        }
        const pending = this.#pending.get(message.requestId);
        if (pending !== undefined) {
          window.clearTimeout(pending.timeout);
          this.#pending.delete(message.requestId);
          pending.reject(error);
        }
        return;
      }
      case "event":
        if (
          this.#lastEventSequence !== null &&
          message.sequence <= this.#lastEventSequence
        ) {
          return;
        }
        this.#lastEventSequence = message.sequence;
        switch (message.event.type) {
          case "autocomplete_updated":
            this.#onAutocomplete(message.event.autocomplete);
            break;
          case "configuration_reloaded":
            this.#onConfiguration(message.event.configuration);
            break;
          case "dashboard_updated":
            this.#onDashboard(message.event.dashboard);
            break;
          case "run_updated":
            this.#onRun(message.event.run);
            break;
        }
        return;
      case "response": {
        const pending = this.#pending.get(message.requestId);
        if (pending === undefined) {
          return;
        }
        window.clearTimeout(pending.timeout);
        this.#pending.delete(message.requestId);
        pending.resolve(message.response);
      }
    }
  }

  #rejectPending(message: string): void {
    for (const request of this.#pending.values()) {
      window.clearTimeout(request.timeout);
      request.reject(new Error(message));
    }
    this.#pending.clear();
  }

  #scheduleReconnect(): void {
    if (!this.#active || this.#reconnectTimer !== null) {
      return;
    }
    this.#onStatus("reconnecting");
    const delay =
      RECONNECT_DELAYS[Math.min(this.#attempt, RECONNECT_DELAYS.length - 1)];
    this.#attempt += 1;
    const generation = this.#generation;
    this.#reconnectTimer = window.setTimeout(() => {
      this.#reconnectTimer = null;
      if (generation === this.#generation) {
        this.#connect(generation);
      }
    }, delay);
  }

  #synchronizeSetup(activeConfiguration: ActiveConfiguration | null): void {
    const setup = this.#getSetup();
    if (setup === null) {
      return;
    }
    if (
      activeConfiguration !== null &&
      JSON.stringify(activeConfiguration.setup) === JSON.stringify(setup)
    ) {
      this.#onConfiguration(activeConfiguration);
      return;
    }
    void this.applySetup(setup)
      .then((configuration) => this.#onConfiguration(configuration))
      .catch((error: unknown) =>
        this.#onError(
          error instanceof Error
            ? error.message
            : "Could not synchronize the saved configuration.",
        ),
      );
  }
}
