import { useEffect, useMemo, useRef, useState } from "react";

import type { ActiveConfiguration } from "./generated/transport";
import { OptionsPage } from "./options/OptionsPage";
import { OptifySetupStore } from "./options/OptifySetupStore";
import { applyTheme } from "./theme";
import { WebSocketClient, type ConnectionStatus } from "./WebSocketClient";

export function App() {
  const store = useMemo(() => new OptifySetupStore(window.localStorage), []);
  const [acceptedSetup, setAcceptedSetup] = useState(() => store.load());
  const [activeConfiguration, setActiveConfiguration] =
    useState<ActiveConfiguration | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [connectionStatus, setConnectionStatus] =
    useState<ConnectionStatus>("connecting");
  const acceptedSetupRef = useRef(acceptedSetup);
  const clientRef = useRef<WebSocketClient | null>(null);

  useEffect(() => {
    acceptedSetupRef.current = acceptedSetup;
  }, [acceptedSetup]);

  useEffect(() => {
    const client = new WebSocketClient({
      getSetup: () => acceptedSetupRef.current,
      onConfiguration: (configuration) => {
        store.save(configuration.setup);
        acceptedSetupRef.current = configuration.setup;
        setAcceptedSetup(configuration.setup);
        setActiveConfiguration(configuration);
        setConnectionError(null);
      },
      onError: setConnectionError,
      onStatus: setConnectionStatus,
    });
    clientRef.current = client;
    client.start();
    return () => {
      clientRef.current = null;
      client.stop();
    };
  }, [store]);

  useEffect(() => {
    if (acceptedSetup === null && window.location.pathname !== "/options") {
      window.history.replaceState(null, "", "/options");
    }
  }, [acceptedSetup]);

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
