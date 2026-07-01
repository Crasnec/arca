import { ArchiveTree } from "./archive-tree";
import { EntryTable, type EntryTableActions, type EntryTableView } from "./entry-table";
import type {
  ArchiveManifest,
  DropState
} from "../../shared/types";
import styles from "./entry-workspace.module.css";

export type EntryWorkspaceProps = {
  view: {
    dropState: DropState;
    manifest: ArchiveManifest | null;
    treeRows: string[];
    table: EntryTableView;
  };
  actions: EntryTableActions;
};

export function EntryWorkspace({ view, actions }: EntryWorkspaceProps) {
  const { dropState, manifest, treeRows, table } = view;
  return (
    <section
      className={`${styles.workspace}${dropState === "hover" ? ` ${styles.dropTarget}` : ""}`}
    >
      <ArchiveTree manifest={manifest} treeRows={treeRows} />
      <EntryTable manifest={manifest} table={table} actions={actions} />
    </section>
  );
}
