# GUI direct editing uses pending save

Arca's Archive Manager will treat Direct Editing actions as pending changes until the user
explicitly saves. This avoids surprising archive mutation during drag-and-drop or deletion, while
still allowing Replacement Prompts to resolve conflicts as they enter the pending change set. Bulk
replacement prompts keep conflict detection in the Rust planner and let the frontend consume those
planned conflicts with per-entry Skip/Replace plus Skip All/Replace All choices. Additional add
batches are allowed before Save, but the frontend passes existing pending add entries back to the
Rust planner so pending-vs-new conflicts are still rejected by core archive policy rather than
frontend string matching.
