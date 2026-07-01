import { createArchivePayloadArchiveActions } from "./archive-payload-archive-actions";
import { createArchivePayloadSelectionActions } from "./archive-payload-selection-actions";
import type { ArchivePayloadActionsInput } from "./archive-payload-action-types";

export function createArchivePayloadActions(input: ArchivePayloadActionsInput) {
  return {
    ...createArchivePayloadArchiveActions(input),
    ...createArchivePayloadSelectionActions(input)
  };
}
