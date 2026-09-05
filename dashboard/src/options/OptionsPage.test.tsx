import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ActiveConfiguration } from "../generated/transport";
import { OptionsPage } from "./OptionsPage";

const acceptedConfiguration: ActiveConfiguration = {
  autocomplete: {
    debounceMilliseconds: 300,
    minimumCharacters: 20,
  },
  revision: 8,
  setup: {
    configDirectories: ["/work", "/personal"],
    features: ["work-dashboard", "dashboard"],
  },
  theme: "system",
};

describe("OptionsPage", () => {
  it("edits and applies directories before features while preserving their order", async () => {
    const applySetup = vi.fn().mockResolvedValue(acceptedConfiguration);
    const onAccepted = vi.fn();
    const user = userEvent.setup();
    render(
      <OptionsPage
        acceptedSetup={{
          configDirectories: ["/personal", "/work"],
          features: ["dashboard", "work-dashboard"],
        }}
        applySetup={applySetup}
        connectionError={null}
        connectionStatus="connected"
        onAccepted={onAccepted}
      />,
    );

    expect(screen.queryByText("Personal Dashboard")).toBeNull();
    expect(
      screen
        .getAllByRole("heading", { level: 2 })
        .map((heading) => heading.textContent),
    ).toEqual(["Optify directories", "Optify features"]);
    await user.click(
      screen.getByRole("button", { name: "Move configuration directory 2 up" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Move root feature 2 up" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Apply Optify configuration" }),
    );

    expect(applySetup).toHaveBeenCalledWith(acceptedConfiguration.setup);
    expect(onAccepted).toHaveBeenCalledWith(acceptedConfiguration);
    expect(
      await screen.findByText("Applied configuration revision 8."),
    ).toBeTruthy();
  });

  it("keeps an invalid draft editable without calling the service", async () => {
    const applySetup = vi.fn();
    const user = userEvent.setup();
    render(
      <OptionsPage
        acceptedSetup={null}
        applySetup={applySetup}
        connectionError={null}
        connectionStatus="connected"
        onAccepted={vi.fn()}
      />,
    );

    await user.click(
      screen.getByRole("button", { name: "Apply Optify configuration" }),
    );

    expect(applySetup).not.toHaveBeenCalled();
    expect(
      screen.getByText("Optify directory 1 cannot be blank."),
    ).toBeTruthy();
    expect(screen.getByText("Optify feature 1 cannot be blank.")).toBeTruthy();
  });
});
