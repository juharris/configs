import { type FormEvent, useState } from "react";

import type { ActiveConfiguration, OptifySetup } from "../generated/transport";
import type { ConnectionStatus } from "../WebSocketClient";
import { validateOptifySetup } from "./OptifySetupStore";

type OptionsPageProps = {
  acceptedSetup: OptifySetup | null;
  applySetup: (setup: OptifySetup) => Promise<ActiveConfiguration>;
  connectionError: string | null;
  connectionStatus: ConnectionStatus;
  onAccepted: (configuration: ActiveConfiguration) => void;
};

type OrderedListEditorProps = {
  addLabel: string;
  itemLabel: string;
  onChange: (values: string[]) => void;
  placeholder: string;
  values: string[];
};

export function OptionsPage({
  acceptedSetup,
  applySetup,
  connectionError,
  connectionStatus,
  onAccepted,
}: OptionsPageProps) {
  const [configDirectories, setConfigDirectories] = useState(
    acceptedSetup?.configDirectories ?? [""],
  );
  const [features, setFeatures] = useState(acceptedSetup?.features ?? [""]);
  const [isApplying, setIsApplying] = useState(false);
  const [messages, setMessages] = useState<string[]>([]);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const setup = { configDirectories, features };
    const validationMessages = validateOptifySetup(setup);
    if (validationMessages.length > 0) {
      setMessages(validationMessages);
      return;
    }

    setIsApplying(true);
    setMessages([]);
    try {
      const configuration = await applySetup(setup);
      onAccepted(configuration);
      setMessages([
        `Applied configuration revision ${configuration.revision.toString()}.`,
      ]);
    } catch (error) {
      setMessages([
        error instanceof Error
          ? error.message
          : "Could not apply Optify configuration.",
      ]);
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <main className="options-layout">
      <h1 className="visually-hidden">Dashboard options</h1>
      <form onSubmit={submit} noValidate>
        <div className="options-grid">
          <section
            className="option-card"
            aria-labelledby="directories-heading"
          >
            <div className="section-heading">
              <div>
                <h2 id="directories-heading">Optify directories</h2>
                <p>Absolute paths; earlier directories load first.</p>
              </div>
            </div>
            <OrderedListEditor
              addLabel="Add directory"
              itemLabel="Configuration directory"
              onChange={setConfigDirectories}
              placeholder="/Users/name/config"
              values={configDirectories}
            />
          </section>

          <section className="option-card" aria-labelledby="features-heading">
            <div className="section-heading">
              <div>
                <h2 id="features-heading">Optify features</h2>
                <p>Later root features override earlier values.</p>
              </div>
            </div>
            <OrderedListEditor
              addLabel="Add feature"
              itemLabel="Root feature"
              onChange={setFeatures}
              placeholder="dashboard"
              values={features}
            />
          </section>
        </div>

        <div className="apply-bar">
          <div className="status-group">
            <span className="connection-status" data-status={connectionStatus}>
              {connectionStatusLabel(connectionStatus)}
            </span>
            {acceptedSetup === null ? null : (
              <a className="utility-link" href="/">
                Dashboard
              </a>
            )}
            <div className="messages" aria-live="polite">
              {connectionError === null ? null : <p>{connectionError}</p>}
              {messages.map((message) => (
                <p key={message}>{message}</p>
              ))}
            </div>
          </div>
          <button
            aria-label="Apply Optify configuration"
            className="primary-button"
            disabled={isApplying || connectionStatus !== "connected"}
            type="submit"
          >
            {isApplying ? "Applying…" : "Apply"}
          </button>
        </div>
      </form>
    </main>
  );
}

function connectionStatusLabel(status: ConnectionStatus): string {
  switch (status) {
    case "connected":
      return "Connected";
    case "connecting":
      return "Connecting…";
    case "disconnected":
      return "Disconnected";
    case "reconnecting":
      return "Reconnecting…";
  }
}

function OrderedListEditor({
  addLabel,
  itemLabel,
  onChange,
  placeholder,
  values,
}: OrderedListEditorProps) {
  const add = () => onChange([...values, ""]);
  const move = (from: number, to: number) => {
    const reordered = [...values];
    const [value] = reordered.splice(from, 1);
    if (value === undefined) {
      return;
    }
    reordered.splice(to, 0, value);
    onChange(reordered);
  };
  const remove = (index: number) =>
    onChange(values.filter((_, valueIndex) => valueIndex !== index));
  const update = (index: number, value: string) =>
    onChange(
      values.map((current, valueIndex) =>
        valueIndex === index ? value : current,
      ),
    );

  return (
    <div className="ordered-list">
      {values.map((value, index) => (
        <div className="ordered-row" key={index}>
          <span className="order-index" aria-hidden="true">
            {index + 1}
          </span>
          <label>
            <span className="visually-hidden">
              {itemLabel} {index + 1}
            </span>
            <input
              autoComplete="off"
              onChange={(event) => update(index, event.target.value)}
              placeholder={placeholder}
              spellCheck={false}
              type="text"
              value={value}
            />
          </label>
          <div
            className="row-actions"
            aria-label={`${itemLabel} ${index + 1} ordering`}
          >
            <button
              aria-label={`Move ${itemLabel.toLowerCase()} ${index + 1} up`}
              disabled={index === 0}
              onClick={() => move(index, index - 1)}
              type="button"
            >
              ↑
            </button>
            <button
              aria-label={`Move ${itemLabel.toLowerCase()} ${index + 1} down`}
              disabled={index === values.length - 1}
              onClick={() => move(index, index + 1)}
              type="button"
            >
              ↓
            </button>
            <button
              aria-label={`Remove ${itemLabel.toLowerCase()} ${index + 1}`}
              disabled={values.length === 1}
              onClick={() => remove(index)}
              type="button"
            >
              ×
            </button>
          </div>
        </div>
      ))}
      <button className="add-button" onClick={add} type="button">
        <span aria-hidden="true">+</span> {addLabel}
      </button>
    </div>
  );
}
