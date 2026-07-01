import {
  CommandBarOperationAction,
  CommandBarPendingActions,
  CommandBarPrimaryActions,
  CommandBarSettingsAction
} from "./command-bar-sections";
import type { OperationProgress } from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./command-bar.module.css";

export type CommandBarState = {
  hasArchive: boolean;
  loading: boolean;
  selectedCount: number;
  canAddDirectEdit: boolean;
  canDeleteSelected: boolean;
  hasPendingChanges: boolean;
  canSaveDirectEdit: boolean;
  canUndoPendingChanges: boolean;
  canRedoPendingChanges: boolean;
  activeOperation: OperationProgress | null;
};

export type CommandBarActions = {
  onAddFiles: () => void | Promise<void>;
  onExtract: () => void | Promise<void>;
  onTest: () => void | Promise<void>;
  onCopy: () => void | Promise<void>;
  onDelete: () => void;
  onInfo: () => void;
  onSave: () => void | Promise<void>;
  onUndo: () => void;
  onRedo: () => void;
  onCancelOperation: () => void | Promise<void>;
};

export type CommandBarProps = {
  state: CommandBarState;
  actions: CommandBarActions;
  onOpenSettings?: () => void;
};

export function CommandBar({ state, actions, onOpenSettings }: CommandBarProps) {
  const { t } = useI18n();
  return (
    <div className={styles.bar} aria-label={t("commandBar.aria")}>
      <CommandBarPrimaryActions state={state} actions={actions} />
      <CommandBarPendingActions state={state} actions={actions} />
      <CommandBarOperationAction state={state} actions={actions} />
      <CommandBarSettingsAction onOpenSettings={onOpenSettings} />
    </div>
  );
}
