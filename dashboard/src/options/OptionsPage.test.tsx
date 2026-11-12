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
    const { rerender } = render(
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
    rerender(
      <OptionsPage
        acceptedSetup={acceptedConfiguration.setup}
        applySetup={applySetup}
        connectionError={null}
        connectionStatus="connected"
        onAccepted={onAccepted}
      />,
    );
    expect(
      await screen.findByText("Applied configuration revision 8."),
    ).toBeTruthy();
  });

  it.each([
    null,
    {
      configDirectories: ["/stale-browser/configs"],
      features: ["stale-browser-feature"],
    },
  ])(
    "adopts active server setup after mounting with %j and preserves edits on equivalent refreshes",
    async (initialSetup) => {
      const user = userEvent.setup();
      const props = {
        applySetup: vi.fn(),
        connectionError: null,
        connectionStatus: "connected" as const,
        onAccepted: vi.fn(),
      };
      const { rerender } = render(
        <OptionsPage {...props} acceptedSetup={initialSetup} />,
      );

      rerender(
        <OptionsPage {...props} acceptedSetup={acceptedConfiguration.setup} />,
      );
      expect(
        screen
          .getAllByRole("textbox")
          .map((input) => (input as HTMLInputElement).value),
      ).toEqual([
        ...acceptedConfiguration.setup.configDirectories,
        ...acceptedConfiguration.setup.features,
      ]);

      const directory = screen.getByRole("textbox", {
        name: "Configuration directory 1",
      });
      const feature = screen.getByRole("textbox", { name: "Root feature 1" });
      await user.clear(directory);
      await user.type(directory, "/edited-directory");
      await user.clear(feature);
      await user.type(feature, "edited-feature");
      rerender(
        <OptionsPage
          {...props}
          acceptedSetup={{
            features: [...acceptedConfiguration.setup.features],
            configDirectories: [
              ...acceptedConfiguration.setup.configDirectories,
            ],
          }}
        />,
      );

      expect((directory as HTMLInputElement).value).toBe("/edited-directory");
      expect((feature as HTMLInputElement).value).toBe("edited-feature");
      expect(props.applySetup).not.toHaveBeenCalled();
    },
  );

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
