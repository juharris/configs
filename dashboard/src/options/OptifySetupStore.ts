import type { OptifySetup } from "../generated/transport";

export const CONFIG_DIRECTORIES_KEY = "personal-dashboard.config-directories";
export const CONFIG_DIRECTORIES_PARAMETER = "config_dirs";
export const FEATURES_KEY = "personal-dashboard.features";
export const FEATURES_PARAMETER = "features";

type StorageAccess = Pick<Storage, "getItem" | "setItem">;

/** Owns all access to the dashboard's two permitted browser-storage values. */
export class OptifySetupStore {
  readonly #storage: StorageAccess;

  constructor(storage: StorageAccess) {
    this.#storage = storage;
  }

  load(parameters?: URLSearchParams): OptifySetup | null {
    const parameterSetup =
      parameters === undefined ? null : setupFromParameters(parameters);
    if (parameterSetup !== null) {
      return validateOptifySetup(parameterSetup).length === 0
        ? parameterSetup
        : null;
    }
    const configDirectories = parseStringArray(
      this.#storage.getItem(CONFIG_DIRECTORIES_KEY),
    );
    const features = parseStringArray(this.#storage.getItem(FEATURES_KEY));
    if (configDirectories === null || features === null) {
      return null;
    }
    const setup = { configDirectories, features };
    return validateOptifySetup(setup).length === 0 ? setup : null;
  }

  save(setup: OptifySetup): void {
    const errors = validateOptifySetup(setup);
    if (errors.length > 0) {
      throw new Error(errors[0]);
    }
    const configDirectories = JSON.stringify(setup.configDirectories);
    const features = JSON.stringify(setup.features);
    this.#storage.setItem(CONFIG_DIRECTORIES_KEY, configDirectories);
    this.#storage.setItem(FEATURES_KEY, features);
  }
}

function setupFromParameters(parameters: URLSearchParams): OptifySetup | null {
  if (
    !parameters.has(CONFIG_DIRECTORIES_PARAMETER) &&
    !parameters.has(FEATURES_PARAMETER)
  ) {
    return null;
  }
  return {
    configDirectories: parameters.getAll(CONFIG_DIRECTORIES_PARAMETER),
    features: parameters.getAll(FEATURES_PARAMETER),
  };
}

export function validateOptifySetup(setup: OptifySetup): string[] {
  const errors: string[] = [];
  if (setup.configDirectories.length === 0) {
    errors.push("Add at least one Optify directory.");
  }
  setup.configDirectories.forEach((directory, index) => {
    if (directory.trim().length === 0) {
      errors.push(`Optify directory ${index + 1} cannot be blank.`);
    } else if (!directory.startsWith("/")) {
      errors.push(`Optify directory ${index + 1} must be an absolute path.`);
    }
  });
  if (setup.features.length === 0) {
    errors.push("Add at least one Optify feature.");
  }
  setup.features.forEach((feature, index) => {
    if (feature.trim().length === 0) {
      errors.push(`Optify feature ${index + 1} cannot be blank.`);
    }
  });
  return errors;
}

function parseStringArray(serialized: string | null): string[] | null {
  if (serialized === null) {
    return null;
  }
  try {
    const value: unknown = JSON.parse(serialized);
    return Array.isArray(value) &&
      value.every((entry) => typeof entry === "string")
      ? value
      : null;
  } catch {
    return null;
  }
}
