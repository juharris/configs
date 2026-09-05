import { beforeEach, describe, expect, it } from "vitest";

import {
  CONFIG_DIRECTORIES_KEY,
  CONFIG_DIRECTORIES_PARAMETER,
  FEATURES_KEY,
  FEATURES_PARAMETER,
  OptifySetupStore,
  validateOptifySetup,
} from "./OptifySetupStore";

describe("OptifySetupStore", () => {
  beforeEach(() => window.localStorage.clear());

  it("persists and loads exactly the two ordered string arrays", () => {
    const setup = {
      configDirectories: ["/personal", "/work"],
      features: ["dashboard", "work-dashboard"],
    };
    const store = new OptifySetupStore(window.localStorage);

    store.save(setup);

    expect(store.load()).toEqual(setup);
    expect(Object.keys(window.localStorage).sort()).toEqual(
      [CONFIG_DIRECTORIES_KEY, FEATURES_KEY].sort(),
    );
  });

  it("loads repeated URL parameters in order before stored values", () => {
    const store = new OptifySetupStore(window.localStorage);
    store.save({
      configDirectories: ["/stored"],
      features: ["stored-dashboard"],
    });
    const parameters = new URLSearchParams();
    parameters.append(CONFIG_DIRECTORIES_PARAMETER, "/personal");
    parameters.append(CONFIG_DIRECTORIES_PARAMETER, "/work");
    parameters.append(FEATURES_PARAMETER, "dashboard");
    parameters.append(FEATURES_PARAMETER, "work-dashboard");

    expect(store.load(parameters)).toEqual({
      configDirectories: ["/personal", "/work"],
      features: ["dashboard", "work-dashboard"],
    });
  });

  it("does not fall back to stored values for incomplete URL setup", () => {
    const store = new OptifySetupStore(window.localStorage);
    store.save({
      configDirectories: ["/stored"],
      features: ["stored-dashboard"],
    });
    const parameters = new URLSearchParams();
    parameters.append(CONFIG_DIRECTORIES_PARAMETER, "/personal");

    expect(store.load(parameters)).toBeNull();
  });

  it("treats missing, malformed, and invalid stored values as unavailable", () => {
    const store = new OptifySetupStore(window.localStorage);
    expect(store.load()).toBeNull();

    window.localStorage.setItem(
      CONFIG_DIRECTORIES_KEY,
      JSON.stringify(["/config"]),
    );
    window.localStorage.setItem(FEATURES_KEY, "not JSON");
    expect(store.load()).toBeNull();

    window.localStorage.setItem(FEATURES_KEY, JSON.stringify([]));
    expect(store.load()).toBeNull();
  });
});

describe("validateOptifySetup", () => {
  it("requires absolute nonblank directories and nonblank features", () => {
    expect(
      validateOptifySetup({
        configDirectories: ["relative", " "],
        features: [""],
      }),
    ).toEqual([
      "Optify directory 1 must be an absolute path.",
      "Optify directory 2 cannot be blank.",
      "Optify feature 1 cannot be blank.",
    ]);
  });
});
