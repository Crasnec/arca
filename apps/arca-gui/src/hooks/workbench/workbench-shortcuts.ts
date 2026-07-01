import React from "react";
import type { WorkbenchActionGroups } from "../../models/workbench-actions";
import {
  createWorkbenchShortcutHandler,
  type ShortcutCapabilities,
  type ShortcutPromptState
} from "./workbench-shortcut-handler";

type WorkbenchShortcutsInput = {
  prompts: ShortcutPromptState;
  loading: boolean;
  capabilities: ShortcutCapabilities;
  actions: WorkbenchActionGroups;
};

export function useWorkbenchShortcuts({
  prompts,
  loading,
  capabilities,
  actions
}: WorkbenchShortcutsInput) {
  React.useEffect(() => {
    const handleWorkbenchShortcut = createWorkbenchShortcutHandler({
      prompts,
      loading,
      capabilities,
      actions
    });

    window.addEventListener("keydown", handleWorkbenchShortcut);
    return () => {
      window.removeEventListener("keydown", handleWorkbenchShortcut);
    };
  }, [
    capabilities.canDeleteSelected,
    capabilities.canRedoPendingChanges,
    capabilities.canSaveDirectEdit,
    capabilities.canUndoPendingChanges,
    actions,
    loading,
    prompts
  ]);
}
