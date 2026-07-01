import {
  ArrowRightLeft,
  CircleStop,
  Copy,
  Download,
  Settings,
  Info,
  RotateCcw,
  RotateCw,
  Save,
  ShieldCheck,
  Trash2,
  Upload
} from "lucide-react";
import { CommandButton } from "./command-button";
import type { CommandBarActions, CommandBarState } from "./command-bar";
import { useI18n } from "../../i18n";
import styles from "./command-bar.module.css";

export type CommandBarPrimaryActionsProps = {
  state: Pick<
    CommandBarState,
    "hasArchive" | "loading" | "selectedCount" | "canAddDirectEdit" | "canDeleteSelected"
  >;
  actions: Pick<
    CommandBarActions,
    "onAddFiles" | "onExtract" | "onTest" | "onCopy" | "onDelete" | "onInfo"
  >;
};

export function CommandBarPrimaryActions({ state, actions }: CommandBarPrimaryActionsProps) {
  const { t } = useI18n();
  return (
    <>
      <CommandButton
        icon={Upload}
        label={t("command.add")}
        title={t("command.addTitle")}
        disabled={!state.canAddDirectEdit}
        onClick={actions.onAddFiles}
      />
      <CommandButton
        icon={Download}
        label={t("command.extract")}
        title={
          state.selectedCount > 0 ? t("command.extractSelectedTitle") : t("command.extract")
        }
        disabled={!state.hasArchive || state.loading}
        onClick={actions.onExtract}
      />
      <CommandButton
        icon={ShieldCheck}
        label={t("command.test")}
        title={state.selectedCount > 0 ? t("command.testSelectedTitle") : t("command.test")}
        disabled={!state.hasArchive || state.loading}
        onClick={actions.onTest}
      />
      <CommandButton
        icon={Copy}
        label={t("command.copy")}
        title={t("command.copyTitle")}
        disabled={state.loading || state.selectedCount === 0}
        onClick={actions.onCopy}
      />
      <CommandButton
        icon={ArrowRightLeft}
        label={t("command.move")}
        title={t("command.moveUnavailableTitle")}
        disabled
      />
      <CommandButton
        icon={Trash2}
        label={t("command.delete")}
        title={t("command.deleteTitle")}
        disabled={!state.canDeleteSelected}
        onClick={actions.onDelete}
      />
      <CommandButton
        icon={Info}
        label={t("command.info")}
        title={t("command.infoTitle")}
        disabled={!state.hasArchive}
        onClick={actions.onInfo}
      />
    </>
  );
}

export type CommandBarPendingActionsProps = {
  state: Pick<
    CommandBarState,
    "hasPendingChanges" | "canSaveDirectEdit" | "canUndoPendingChanges" | "canRedoPendingChanges"
  >;
  actions: Pick<CommandBarActions, "onSave" | "onUndo" | "onRedo">;
};

export function CommandBarPendingActions({ state, actions }: CommandBarPendingActionsProps) {
  const { t } = useI18n();
  if (!state.hasPendingChanges && !state.canUndoPendingChanges && !state.canRedoPendingChanges) {
    return null;
  }

  return (
    <>
      <span className={styles.separator} />
      <CommandButton
        icon={Save}
        label={t("command.save")}
        title={t("command.save")}
        disabled={!state.canSaveDirectEdit}
        onClick={actions.onSave}
      />
      <CommandButton
        icon={RotateCcw}
        label={t("command.undo")}
        title={t("command.undo")}
        disabled={!state.canUndoPendingChanges}
        ariaLabel={t("command.undo")}
        onClick={actions.onUndo}
      />
      <CommandButton
        icon={RotateCw}
        label={t("command.redo")}
        title={t("command.redo")}
        disabled={!state.canRedoPendingChanges}
        ariaLabel={t("command.redo")}
        onClick={actions.onRedo}
      />
    </>
  );
}

export type CommandBarOperationActionProps = {
  state: Pick<CommandBarState, "activeOperation">;
  actions: Pick<CommandBarActions, "onCancelOperation">;
};

export function CommandBarOperationAction({ state, actions }: CommandBarOperationActionProps) {
  const { t } = useI18n();
  const { activeOperation } = state;
  if (!activeOperation) {
    return null;
  }

  return (
    <>
      <span className={styles.separator} />
      <CommandButton
        icon={CircleStop}
        label={activeOperation.cancelRequested ? t("command.canceling") : t("command.cancel")}
        title={t("command.cancelOperationTitle")}
        disabled={!activeOperation.cancellable || activeOperation.cancelRequested}
        onClick={actions.onCancelOperation}
      />
    </>
  );
}

export function CommandBarSettingsAction({
  onOpenSettings
}: {
  onOpenSettings?: () => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <span className={styles.separator} />
      <CommandButton
        icon={Settings}
        label={t("command.settings")}
        title={t("command.settingsTitle")}
        disabled={!onOpenSettings}
        onClick={onOpenSettings}
      />
    </>
  );
}
