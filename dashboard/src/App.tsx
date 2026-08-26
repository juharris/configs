import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { DashboardPage } from "./dashboard/DashboardPage";
import type {
  ActiveConfiguration,
  AutocompleteSnapshot,
  ButtonList,
  DashboardItem,
  DashboardSnapshot,
  RunSnapshot,
} from "./generated/transport";
import { OptionsPage } from "./options/OptionsPage";
import { OptifySetupStore } from "./options/OptifySetupStore";
import { applyTheme } from "./theme";
import {
  type AutocompleteRequestParameters,
  type ConnectionStatus,
  WebSocketClient,
} from "./WebSocketClient";

export function App() {
  const store = useMemo(() => new OptifySetupStore(window.localStorage), []);
  const [acceptedSetup, setAcceptedSetup] = useState(() => store.load());
  const [activeConfiguration, setActiveConfiguration] =
    useState<ActiveConfiguration | null>(null);
  const [autocompletes, setAutocompletes] = useState<
    Record<string, AutocompleteSnapshot>
  >({});
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [connectionStatus, setConnectionStatus] =
    useState<ConnectionStatus>("connecting");
  const [dashboard, setDashboard] = useState<DashboardSnapshot | null>(null);
  const [run, setRun] = useState<RunSnapshot | null>(null);
  const [path, setPath] = useState(() => {
    if (acceptedSetup === null && window.location.pathname !== "/options") {
      window.history.replaceState(null, "", "/options");
      return "/options";
    }
    return window.location.pathname;
  });
  const acceptedSetupRef = useRef(acceptedSetup);
  const clientRef = useRef<WebSocketClient | null>(null);

  const acceptRun = useCallback((nextRun: RunSnapshot) => {
    setRun((currentRun) => {
      if (currentRun?.id !== nextRun.id) {
        return nextRun;
      }
      return runProgress(nextRun) >= runProgress(currentRun)
        ? nextRun
        : currentRun;
    });
  }, []);

  useEffect(() => {
    acceptedSetupRef.current = acceptedSetup;
  }, [acceptedSetup]);

  useEffect(() => {
    const client = new WebSocketClient({
      getSetup: () => acceptedSetupRef.current,
      onAutocomplete: (autocomplete) =>
        setAutocompletes((current) => ({
          ...current,
          [autocomplete.editorId]: autocomplete,
        })),
      onConfiguration: (configuration) => {
        store.save(configuration.setup);
        acceptedSetupRef.current = configuration.setup;
        setAcceptedSetup(configuration.setup);
        setActiveConfiguration(configuration);
        setConnectionError(null);
      },
      onDashboard: setDashboard,
      onError: setConnectionError,
      onRun: acceptRun,
      onStatus: setConnectionStatus,
    });
    clientRef.current = client;
    client.start();
    return () => {
      clientRef.current = null;
      client.stop();
    };
  }, [acceptRun, store]);

  useEffect(() => {
    const updatePath = () => setPath(window.location.pathname);
    window.addEventListener("popstate", updatePath);
    return () => window.removeEventListener("popstate", updatePath);
  }, []);

  useEffect(() => {
    if (activeConfiguration === null) {
      return;
    }
    return applyTheme(activeConfiguration.theme);
  }, [activeConfiguration]);

  const acceptSetup = (configuration: ActiveConfiguration) => {
    store.save(configuration.setup);
    acceptedSetupRef.current = configuration.setup;
    setActiveConfiguration(configuration);
    setAcceptedSetup(configuration.setup);
    setConnectionError(null);
  };

  if (path === "/options" || acceptedSetup === null) {
    return (
      <OptionsPage
        acceptedSetup={acceptedSetup}
        applySetup={(setup) => {
          const client = clientRef.current;
          return client === null
            ? Promise.reject(new Error("The dashboard service is connecting."))
            : client.applySetup(setup);
        }}
        connectionError={connectionError}
        connectionStatus={connectionStatus}
        onAccepted={acceptSetup}
      />
    );
  }

  return (
    <DashboardPage
      activeConfiguration={activeConfiguration}
      autocompletes={autocompletes}
      cancelAutocomplete={async (editorId) => {
        const client = clientRef.current;
        if (client === null) {
          throw new Error("The dashboard service is connecting.");
        }
        await client.cancelAutocomplete(editorId);
      }}
      cancelRun={async (runId) => {
        const client = clientRef.current;
        if (client === null) {
          throw new Error("The dashboard service is connecting.");
        }
        await client.cancelRun(runId);
      }}
      connectionError={connectionError}
      connectionStatus={connectionStatus}
      dashboard={dashboard}
      dismissRun={() => setRun(null)}
      previewButton={async (
        sectionId: string,
        item: DashboardItem,
        buttonList: ButtonList,
        buttonIndex: number,
        prompt: string,
      ) => {
        const client = clientRef.current;
        if (client === null || activeConfiguration === null) {
          throw new Error("The dashboard service is connecting.");
        }
        return client.previewButton(
          buttonIndex,
          buttonList,
          activeConfiguration.revision,
          {
            number: item.number,
            repository: item.repository,
            source: item.source,
          },
          prompt,
          sectionId,
        );
      }}
      refreshSection={async (sectionId) => {
        const client = clientRef.current;
        if (client === null || activeConfiguration === null) {
          throw new Error("The dashboard service is connecting.");
        }
        try {
          await client.refreshSection(activeConfiguration.revision, sectionId);
          setConnectionError(null);
        } catch (error) {
          setConnectionError(
            error instanceof Error
              ? error.message
              : "Could not refresh the dashboard section.",
          );
          throw error;
        }
      }}
      requestAutocomplete={async (
        parameters: AutocompleteRequestParameters,
      ) => {
        const client = clientRef.current;
        if (client === null) {
          throw new Error("The dashboard service is connecting.");
        }
        await client.requestAutocomplete(parameters);
      }}
      run={run}
      runButton={async (
        sectionId: string,
        item: DashboardItem,
        buttonList: ButtonList,
        buttonIndex: number,
        prompt: string | null,
      ) => {
        const client = clientRef.current;
        if (client === null || activeConfiguration === null) {
          throw new Error("The dashboard service is connecting.");
        }
        const acceptedRun = await client.runButton(
          buttonIndex,
          buttonList,
          activeConfiguration.revision,
          {
            number: item.number,
            repository: item.repository,
            source: item.source,
          },
          prompt,
          sectionId,
        );
        acceptRun(acceptedRun);
      }}
    />
  );
}

function runProgress(run: RunSnapshot): number {
  switch (run.status) {
    case "queued":
      return 0;
    case "running":
      return 1;
    case "cancelled":
    case "completed":
    case "failed":
    case "timed_out":
      return 2;
  }
}
