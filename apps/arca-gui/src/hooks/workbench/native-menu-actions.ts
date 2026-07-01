import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { MENU_ACTION_EVENT } from "../../shared/constants";
import type { WorkbenchActionGroups } from "../../models/workbench-actions";
import type { ArchiveManifest } from "../../shared/types";
import { currentLocale } from "../../i18n/messages";
import {
  type NativeMenuCapabilities,
  runNativeMenuAction
} from "./native-menu-resolvers";

type NativeMenuActionsInput = {
  capabilities: NativeMenuCapabilities;
  selectedCount: number;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
  actions: WorkbenchActionGroups;
};

type ShowEntryContextMenuInput = {
  x: number;
  y: number;
  selectedPaths: string[];
  manifest: ArchiveManifest | null;
  canAddDirectEdit: boolean;
  loading: boolean;
};

export async function showNativeEntryContextMenu({
  x,
  y,
  selectedPaths,
  manifest,
  canAddDirectEdit,
  loading
}: ShowEntryContextMenuInput) {
  const entryPathSet = new Set(manifest?.entries.map((entry) => entry.path) ?? []);
  await invoke("show_entry_context_menu", {
    locale: currentLocale(),
    position: { x, y },
    state: {
      hasSelection: selectedPaths.length > 0,
      canAddDirectEdit,
      canDelete: Boolean(manifest?.directEdit.allowed && selectedPaths.length > 0 && !loading),
      canOpenProperties: selectedPaths.some((selected) => entryPathSet.has(selected)),
      loading
    }
  });
}

export function useNativeMenuActions({
  capabilities,
  selectedCount,
  setStatus,
  actions
}: NativeMenuActionsInput) {
  const handleNativeMenuAction = React.useCallback(
    (action: string) => {
      runNativeMenuAction({
        action,
        capabilities,
        selectedCount,
        actions
      });
    },
    [actions, capabilities, selectedCount]
  );

  React.useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | null = null;

    listen<string>(MENU_ACTION_EVENT, (event) => {
      if (!mounted) {
        return;
      }
      handleNativeMenuAction(event.payload);
    })
      .then((value) => {
        unlisten = value;
      })
      .catch(() => {
        if (mounted) {
          setStatus("Ready");
        }
      });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [handleNativeMenuAction, setStatus]);

  return handleNativeMenuAction;
}
