import React from "react";
import {
  archiveFormatCapabilities,
  fileAssociationStatus,
  setNativeFileAssociation,
  setNativeFileAssociations,
  type ArchiveFormatCapability,
  type FileAssociationStatus
} from "../api/file-associations";
import { CREATE_ARCHIVE_EXTENSIONS } from "../shared/constants";

const STORAGE_KEY = "arca.settings";

export type ViewDensity = "comfortable" | "compact";
export type ArchiveExtension = string;
export type CreateArchiveExtension = (typeof CREATE_ARCHIVE_EXTENSIONS)[number];
export type FileAssociationSettings = Record<ArchiveExtension, boolean>;

export type AppSettings = {
  density: ViewDensity;
  showPackedSize: boolean;
  showEncryptedColumn: boolean;
  defaultArchiveExtension: CreateArchiveExtension;
};

type StoredSettings = {
  density?: ViewDensity;
  showPackedSize?: boolean;
  showEncryptedColumn?: boolean;
  defaultArchiveExtension?: CreateArchiveExtension;
};

type AppSettingsContextValue = {
  settings: AppSettings;
  archiveFormats: ArchiveFormatCapability[];
  density: ViewDensity;
  showPackedSize: boolean;
  showEncryptedColumn: boolean;
  defaultArchiveExtension: CreateArchiveExtension;
  fileAssociationStatus: FileAssociationStatus | null;
  fileAssociations: FileAssociationSettings;
  fileAssociationsLoading: boolean;
  fileAssociationsError: string;
  refreshFileAssociations: () => Promise<void>;
  setDensity: (density: ViewDensity) => void;
  setShowPackedSize: (value: boolean) => void;
  setShowEncryptedColumn: (value: boolean) => void;
  setDefaultArchiveExtension: (extension: CreateArchiveExtension) => void;
  setFileAssociation: (extension: ArchiveExtension, value: boolean) => Promise<void>;
  setAllFileAssociations: (value: boolean) => Promise<void>;
};

const AppSettingsContext = React.createContext<AppSettingsContextValue | null>(null);

export const DEFAULT_APP_SETTINGS: AppSettings = {
  density: "comfortable",
  showPackedSize: true,
  showEncryptedColumn: true,
  defaultArchiveExtension: "zip"
};

function isViewDensity(value: unknown): value is ViewDensity {
  return value === "comfortable" || value === "compact";
}

function isCreateArchiveExtension(value: unknown): value is CreateArchiveExtension {
  return CREATE_ARCHIVE_EXTENSIONS.includes(value as CreateArchiveExtension);
}

function normalizeSettings(value: StoredSettings): AppSettings {
  return {
    density: isViewDensity(value.density) ? value.density : DEFAULT_APP_SETTINGS.density,
    showPackedSize:
      typeof value.showPackedSize === "boolean"
        ? value.showPackedSize
        : DEFAULT_APP_SETTINGS.showPackedSize,
    showEncryptedColumn:
      typeof value.showEncryptedColumn === "boolean"
        ? value.showEncryptedColumn
        : DEFAULT_APP_SETTINGS.showEncryptedColumn,
    defaultArchiveExtension: isCreateArchiveExtension(value.defaultArchiveExtension)
      ? value.defaultArchiveExtension
      : DEFAULT_APP_SETTINGS.defaultArchiveExtension
  };
}

function readStoredSettings(): AppSettings {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return DEFAULT_APP_SETTINGS;
    }
    const value = JSON.parse(raw) as StoredSettings;
    return normalizeSettings(value);
  } catch {
    return DEFAULT_APP_SETTINGS;
  }
}

function writeStoredSettings(settings: AppSettings) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // The setting still applies for the current session if storage is unavailable.
  }
}

function associationMap(status: FileAssociationStatus | null): FileAssociationSettings {
  return Object.fromEntries(
    (status?.entries ?? []).map((entry) => [entry.extension, entry.enabled])
  ) as FileAssociationSettings;
}

function mergeAssociationEntry(
  current: FileAssociationStatus | null,
  entry: FileAssociationStatus["entries"][number]
): FileAssociationStatus {
  const entries = current?.entries ?? [];
  const nextEntries = entries.some((candidate) => candidate.extension === entry.extension)
    ? entries.map((candidate) => candidate.extension === entry.extension ? entry : candidate)
    : [...entries, entry];
  return {
    supported: current?.supported ?? true,
    message: current?.message ?? null,
    entries: nextEntries
  };
}

export function currentAppSettings(): AppSettings {
  return readStoredSettings();
}

export function AppSettingsProvider({ children }: { children: React.ReactNode }) {
  const [settings, setSettings] = React.useState<AppSettings>(readStoredSettings);
  const [archiveFormats, setArchiveFormats] = React.useState<ArchiveFormatCapability[]>([]);
  const [associationStatus, setAssociationStatus] = React.useState<FileAssociationStatus | null>(null);
  const [fileAssociationsLoading, setFileAssociationsLoading] = React.useState(true);
  const [fileAssociationsError, setFileAssociationsError] = React.useState("");

  const refreshFileAssociations = React.useCallback(async () => {
    setFileAssociationsLoading(true);
    setFileAssociationsError("");
    try {
      const [formats, status] = await Promise.all([
        archiveFormatCapabilities(),
        fileAssociationStatus()
      ]);
      setArchiveFormats(formats);
      setAssociationStatus(status);
    } catch (error) {
      setFileAssociationsError(error instanceof Error ? error.message : String(error));
    } finally {
      setFileAssociationsLoading(false);
    }
  }, []);

  const updateSettings = React.useCallback((updater: (settings: AppSettings) => AppSettings) => {
    setSettings((current) => {
      const next = normalizeSettings(updater(current));
      writeStoredSettings(next);
      return next;
    });
  }, []);

  const setDensity = React.useCallback((nextDensity: ViewDensity) => {
    updateSettings((current) => ({ ...current, density: nextDensity }));
  }, [updateSettings]);

  const setShowPackedSize = React.useCallback((value: boolean) => {
    updateSettings((current) => ({ ...current, showPackedSize: value }));
  }, [updateSettings]);

  const setShowEncryptedColumn = React.useCallback((value: boolean) => {
    updateSettings((current) => ({ ...current, showEncryptedColumn: value }));
  }, [updateSettings]);

  const setDefaultArchiveExtension = React.useCallback((extension: CreateArchiveExtension) => {
    updateSettings((current) => ({ ...current, defaultArchiveExtension: extension }));
  }, [updateSettings]);

  const fileAssociations = React.useMemo(
    () => associationMap(associationStatus),
    [associationStatus]
  );

  const setFileAssociation = React.useCallback(
    async (extension: ArchiveExtension, value: boolean) => {
      setFileAssociationsLoading(true);
      setFileAssociationsError("");
      try {
        const entry = await setNativeFileAssociation(extension, value);
        setAssociationStatus((current) => mergeAssociationEntry(current, entry));
      } catch (error) {
        setFileAssociationsError(error instanceof Error ? error.message : String(error));
      } finally {
        setFileAssociationsLoading(false);
      }
    },
    []
  );

  const setAllFileAssociations = React.useCallback(async (value: boolean) => {
    setFileAssociationsLoading(true);
    setFileAssociationsError("");
    try {
      setAssociationStatus(await setNativeFileAssociations(value));
    } catch (error) {
      setFileAssociationsError(error instanceof Error ? error.message : String(error));
    } finally {
      setFileAssociationsLoading(false);
    }
  }, []);

  React.useEffect(() => {
    let mounted = true;
    setFileAssociationsLoading(true);
    Promise.all([archiveFormatCapabilities(), fileAssociationStatus()])
      .then(([formats, status]) => {
        if (!mounted) {
          return;
        }
        setArchiveFormats(formats);
        setAssociationStatus(status);
        setFileAssociationsError("");
      })
      .catch((error: unknown) => {
        if (mounted) {
          setFileAssociationsError(error instanceof Error ? error.message : String(error));
        }
      })
      .finally(() => {
        if (mounted) {
          setFileAssociationsLoading(false);
        }
      });
    return () => {
      mounted = false;
    };
  }, []);

  React.useEffect(() => {
    document.documentElement.dataset.density = settings.density;
    document.documentElement.dataset.showPackedSize = String(settings.showPackedSize);
    document.documentElement.dataset.showEncryptedColumn = String(settings.showEncryptedColumn);
  }, [settings]);

  const value = React.useMemo<AppSettingsContextValue>(
    () => ({
      settings,
      archiveFormats,
      density: settings.density,
      showPackedSize: settings.showPackedSize,
      showEncryptedColumn: settings.showEncryptedColumn,
      defaultArchiveExtension: settings.defaultArchiveExtension,
      fileAssociationStatus: associationStatus,
      fileAssociations,
      fileAssociationsLoading,
      fileAssociationsError,
      refreshFileAssociations,
      setDensity,
      setShowPackedSize,
      setShowEncryptedColumn,
      setDefaultArchiveExtension,
      setFileAssociation,
      setAllFileAssociations
    }),
    [
      settings,
      archiveFormats,
      associationStatus,
      fileAssociations,
      fileAssociationsLoading,
      fileAssociationsError,
      refreshFileAssociations,
      setDensity,
      setShowPackedSize,
      setShowEncryptedColumn,
      setDefaultArchiveExtension,
      setFileAssociation,
      setAllFileAssociations
    ]
  );

  return <AppSettingsContext.Provider value={value}>{children}</AppSettingsContext.Provider>;
}

export function useAppSettings() {
  const value = React.useContext(AppSettingsContext);
  if (!value) {
    throw new Error("useAppSettings must be used within AppSettingsProvider");
  }
  return value;
}
