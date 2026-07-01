import React from "react";
import { AddressBar, type AddressBarProps } from "../address-bar";
import { ArchiveModals, type ArchiveModalsProps } from "../modals";
import { CommandBar, type CommandBarProps } from "../command-bar";
import { EntryWorkspace, type EntryWorkspaceProps } from "../entry-workspace";
import { SettingsDialog } from "../settings";
import { StatusBar, type StatusBarProps } from "../status-bar";
import { OPEN_SETTINGS_EVENT } from "../../shared/constants";
import styles from "./workbench-layout.module.css";

export type WorkbenchLayoutProps = {
  commandBar: CommandBarProps;
  addressBar: AddressBarProps;
  entryWorkspace: EntryWorkspaceProps;
  statusBar: StatusBarProps;
  modals: ArchiveModalsProps;
};

export function WorkbenchLayout({
  commandBar,
  addressBar,
  entryWorkspace,
  statusBar,
  modals
}: WorkbenchLayoutProps) {
  const [settingsOpen, setSettingsOpen] = React.useState(false);

  React.useEffect(() => {
    function openSettings() {
      setSettingsOpen(true);
    }
    window.addEventListener(OPEN_SETTINGS_EVENT, openSettings);
    return () => window.removeEventListener(OPEN_SETTINGS_EVENT, openSettings);
  }, []);

  return (
    <main className={styles.shell}>
      <CommandBar {...commandBar} onOpenSettings={() => setSettingsOpen(true)} />
      <AddressBar {...addressBar} />
      <EntryWorkspace {...entryWorkspace} />
      <StatusBar {...statusBar} />
      <ArchiveModals {...modals} />
      <SettingsDialog open={settingsOpen} close={() => setSettingsOpen(false)} />
    </main>
  );
}
