#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const required = [
  "apps/arca-gui/package.json",
  "apps/arca-gui/src/api/archive-commands.ts",
  "apps/arca-gui/src/api/file-dialogs.ts",
  "apps/arca-gui/src/components/address-bar.module.css",
  "apps/arca-gui/src/components/address-bar.tsx",
  "apps/arca-gui/src/components/entry-workspace/entry-workspace.module.css",
  "apps/arca-gui/src/components/entry-workspace/archive-tree.tsx",
  "apps/arca-gui/src/components/modals/archive-modals.tsx",
  "apps/arca-gui/src/components/modals/modals.module.css",
  "apps/arca-gui/src/components/command-bar/command-bar.module.css",
  "apps/arca-gui/src/components/command-bar/command-button.tsx",
  "apps/arca-gui/src/components/command-bar/command-bar-sections.tsx",
  "apps/arca-gui/src/components/command-bar/command-bar.tsx",
  "apps/arca-gui/src/components/command-bar/index.ts",
  "apps/arca-gui/src/components/entry-workspace/entry-table-body.tsx",
  "apps/arca-gui/src/components/entry-workspace/entry-table-header.tsx",
  "apps/arca-gui/src/components/entry-workspace/entry-table.tsx",
  "apps/arca-gui/src/components/entry-workspace/entry-workspace.tsx",
  "apps/arca-gui/src/components/entry-workspace/index.ts",
  "apps/arca-gui/src/components/modals/close-blocked-modal.tsx",
  "apps/arca-gui/src/components/modals/create-archive-fields.tsx",
  "apps/arca-gui/src/components/modals/create-archive-modal.tsx",
  "apps/arca-gui/src/components/modals/direct-edit-replace-modal.tsx",
  "apps/arca-gui/src/components/modals/info-modals.tsx",
  "apps/arca-gui/src/components/modals/overwrite-prompt-modal.tsx",
  "apps/arca-gui/src/components/modals/password-prompt-modal.tsx",
  "apps/arca-gui/src/components/modals/prompt-modals.tsx",
  "apps/arca-gui/src/components/modals/unsaved-changes-modal.tsx",
  "apps/arca-gui/src/components/modals/index.ts",
  "apps/arca-gui/src/components/settings/index.ts",
  "apps/arca-gui/src/components/settings/settings-dialog.module.css",
  "apps/arca-gui/src/components/settings/settings-dialog.tsx",
  "apps/arca-gui/src/components/status-bar/status-bar.module.css",
  "apps/arca-gui/src/components/status-bar/status-bar-sections.tsx",
  "apps/arca-gui/src/components/status-bar/status-bar.tsx",
  "apps/arca-gui/src/components/status-bar/index.ts",
  "apps/arca-gui/src/components/workbench/workbench-layout.module.css",
  "apps/arca-gui/src/components/workbench/workbench-layout.tsx",
  "apps/arca-gui/src/components/workbench/workbench.tsx",
  "apps/arca-gui/src/components/workbench/index.ts",
  "apps/arca-gui/src-tauri/src/lib.rs",
  "apps/arca-gui/src-tauri/src/file_associations.rs",
  "apps/arca-gui/src-tauri/src/menus.rs",
  "apps/arca-gui/src-tauri/src/operations.rs",
  "apps/arca-gui/src-tauri/src/startup.rs",
  "apps/arca-gui/src/hooks/archive/archive-extract-destination.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-action-types.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-actions.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-archive-actions.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-extract-requests.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-requests.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-runner.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-selection-actions.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-test-requests.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-workflow.ts",
  "apps/arca-gui/src/hooks/archive/archive-session-actions.ts",
  "apps/arca-gui/src/hooks/archive/archive-session-runner.ts",
  "apps/arca-gui/src/hooks/archive/archive-session-workflow.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-action-types.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-actions.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-input-actions.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-modal-actions.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-runner.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-submit-action.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-workflow.ts",
  "apps/arca-gui/src/hooks/archive/archive-view-state.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-state.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-delete-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-feedback-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-add-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-pending-action-types.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-pending-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-pending-history-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-runner.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-save-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-replace-prompt.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-workflow-model.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-workflow.ts",
  "apps/arca-gui/src/hooks/direct-edit/pending-change-state-utils.ts",
  "apps/arca-gui/src/hooks/workbench/entry-browser-state.ts",
  "apps/arca-gui/src/hooks/workbench/entry-selection-state.ts",
  "apps/arca-gui/src/hooks/workbench/native-menu-actions.ts",
  "apps/arca-gui/src/hooks/workbench/native-menu-resolvers.ts",
  "apps/arca-gui/src/hooks/workbench/operation-tracker.ts",
  "apps/arca-gui/src/hooks/workbench/prompt-dialog-resolvers.ts",
  "apps/arca-gui/src/hooks/direct-edit/pending-changes.ts",
  "apps/arca-gui/src/hooks/workbench/tauri-app-events.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-bindings.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-archive-access.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-archive-workflow-model.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-archive-workflows.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-dialogs.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-drag-drop.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-events.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-info-dialogs.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-interactions.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-model.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-ports.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-prompt-close-actions.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-prompt-dialog-model.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-prompt-dialogs.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-shortcut-handler.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-shortcuts.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-state.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-workflow-model.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-workflows.ts",
  "apps/arca-gui/src/hooks/workflow-ports.ts",
  "apps/arca-gui/src/api/file-associations.ts",
  "apps/arca-gui/src/i18n/i18n-context.tsx",
  "apps/arca-gui/src/i18n/index.ts",
  "apps/arca-gui/src/i18n/messages.ts",
  "apps/arca-gui/src/main.tsx",
  "apps/arca-gui/src/settings/index.ts",
  "apps/arca-gui/src/settings/settings-context.tsx",
  "apps/arca-gui/src/models/workbench-actions.ts",
  "apps/arca-gui/src/models/workbench-layout/address-bar-model.ts",
  "apps/arca-gui/src/models/workbench-layout/command-bar-model.ts",
  "apps/arca-gui/src/models/workbench-layout/entry-workspace-model.ts",
  "apps/arca-gui/src/models/workbench-layout/index.ts",
  "apps/arca-gui/src/models/workbench-layout/status-bar-model.ts",
  "apps/arca-gui/src/models/workbench-layout/types.ts",
  "apps/arca-gui/src/models/workbench-layout/workbench-layout-model.ts",
  "apps/arca-gui/src/models/workbench-modal-model.ts",
  "apps/arca-gui/src/styles/base.css",
  "apps/arca-gui/src/styles/index.css",
  "apps/arca-gui/src/styles/tokens.css",
  "apps/arca-gui/src/vite-env.d.ts",
  "apps/arca-gui/src/shared/archive-validation.ts",
  "apps/arca-gui/src/shared/command-errors.ts",
  "apps/arca-gui/src/shared/constants.ts",
  "apps/arca-gui/src/shared/dom-utils.ts",
  "apps/arca-gui/src/shared/drop-intents.ts",
  "apps/arca-gui/src/shared/entry-list-utils.ts",
  "apps/arca-gui/src/shared/format.ts",
  "apps/arca-gui/src/shared/path-utils.ts",
  "apps/arca-gui/src/shared/pending-change-utils.ts",
  "apps/arca-gui/src/shared/prompt-utils.ts",
  "apps/arca-gui/src/shared/types.ts",
  "apps/arca-gui/src/shared/utils.ts",
  "apps/arca-gui/src-tauri/Cargo.toml",
  "apps/arca-gui/src-tauri/tauri.conf.json",
  "apps/arca-gui/src-tauri/capabilities/main.json",
  "apps/arca-gui/src-tauri/icons/arca-icon-source.png",
  "apps/arca-gui/src-tauri/icons/32x32.png",
  "apps/arca-gui/src-tauri/icons/128x128.png",
  "apps/arca-gui/src-tauri/icons/128x128@2x.png",
  "apps/arca-gui/src-tauri/icons/icon.icns",
  "apps/arca-gui/src-tauri/icons/icon.ico",
  "apps/arca-gui/src-tauri/windows/nsis-shell-context.nsh"
];

for (const path of required) {
  if (!existsSync(resolve(root, path))) {
    fail(`missing GUI foundation file: ${path}`);
  }
}

const tauriConfig = JSON.parse(
  readFileSync(resolve(root, "apps/arca-gui/src-tauri/tauri.conf.json"), "utf8")
);
const coreFormat = readFileSync(resolve(root, "crates/arca-core/src/format.rs"), "utf8");
const coreFormatDescriptors = parseCoreFormatDescriptors(coreFormat);
const expectedAssociationExts = coreFormatDescriptors.flatMap((descriptor) => descriptor.extensions);
const expectedMimeExtensions = new Map();
for (const descriptor of coreFormatDescriptors) {
  const current = expectedMimeExtensions.get(descriptor.mimeType) ?? [];
  expectedMimeExtensions.set(descriptor.mimeType, [...current, ...descriptor.extensions]);
}
if (tauriConfig.mainBinaryName !== "arca-gui") {
  fail("Tauri main binary name must stay stable for Windows shell context commands");
}
const expectedBundleIcons = [
  "icons/32x32.png",
  "icons/128x128.png",
  "icons/128x128@2x.png",
  "icons/icon.icns",
  "icons/icon.ico"
];
const bundleIcons = tauriConfig.bundle?.icon ?? [];
if (!Array.isArray(bundleIcons)) {
  fail("Tauri bundle icon list must be explicit");
}
for (const icon of expectedBundleIcons) {
  if (!bundleIcons.includes(icon)) {
    fail(`Tauri bundle icon missing: ${icon}`);
  }
  if (!existsSync(resolve(root, "apps/arca-gui/src-tauri", icon))) {
    fail(`Tauri bundle icon file missing: ${icon}`);
  }
}
if (bundleIcons.some((icon) => String(icon).endsWith(".svg"))) {
  fail("Tauri bundle icons must reference generated desktop icon assets, not only the source SVG");
}
const fileAssociations = tauriConfig.bundle?.fileAssociations ?? [];
if (!Array.isArray(fileAssociations) || fileAssociations.length === 0) {
  fail("Tauri bundle must register archive file associations");
}
const unsupportedAssociationExts = ["z01", "z02", "001", "zipx", "rar", "7z"];
const associatedExts = [];
for (const association of fileAssociations) {
  if (association.role !== "Viewer") {
    fail("Tauri archive file associations must stay Viewer-only");
  }
  if (!Array.isArray(association.ext) || association.ext.length === 0) {
    fail("Tauri archive file association missing extensions");
  }
  if (typeof association.mimeType !== "string" || association.mimeType.length === 0) {
    fail("Tauri archive file associations must declare a Linux MIME type");
  }
  associatedExts.push(...association.ext);
}
if (!sameItems(associatedExts, expectedAssociationExts)) {
  fail(`Tauri archive file associations must match supported extensions: ${expectedAssociationExts.join(", ")}`);
}
for (const unsupportedExt of unsupportedAssociationExts) {
  if (associatedExts.includes(unsupportedExt)) {
    fail(`Tauri file associations must not advertise unsupported split/archive extension: ${unsupportedExt}`);
  }
}
for (const [mimeType, extensions] of expectedMimeExtensions) {
  const association = fileAssociations.find((candidate) => candidate.mimeType === mimeType);
  if (!association || !sameItems(association.ext, extensions)) {
    fail(`Tauri file association MIME mapping is missing or incorrect: ${mimeType}`);
  }
}
const nsis = tauriConfig.bundle?.windows?.nsis ?? {};
if (nsis.installMode !== "currentUser") {
  fail("Windows NSIS install mode must match HKCU shell context registration");
}
if (nsis.installerIcon !== "icons/icon.ico" || nsis.uninstallerIcon !== "icons/icon.ico") {
  fail("Windows NSIS installer/uninstaller icons must use the generated ICO asset");
}
if (nsis.installerHooks !== "windows/nsis-shell-context.nsh") {
  fail("Windows NSIS shell context hook must be wired in tauri.conf.json");
}
if (tauriConfig.build?.devUrl !== "http://127.0.0.1:1420") {
  fail("Tauri dev server must stay bound to 127.0.0.1");
}
if (tauriConfig.app?.windows?.some((window) => window.label !== "main")) {
  fail("GUI M0 must only define the main workbench window");
}
const csp = tauriConfig.app?.security?.csp ?? "";
if (!csp.includes("default-src 'self'") || !csp.includes("script-src 'self'")) {
  fail("Tauri CSP must keep default/script sources restricted to self");
}
if (String(csp).includes("'unsafe-eval'")) {
  fail("Tauri CSP must not allow unsafe-eval");
}
if (String(csp).includes("http:") && !String(csp).includes("http://ipc.localhost")) {
  fail("Tauri CSP must not allow broad remote HTTP content");
}
if (tauriConfig.app?.windows?.some((window) => window.devtools !== false)) {
  fail("production window devtools must be disabled in M0 config");
}

const capabilities = JSON.parse(
  readFileSync(resolve(root, "apps/arca-gui/src-tauri/capabilities/main.json"), "utf8")
);
const permissions = capabilities.permissions ?? [];
for (const deniedPrefix of ["fs:", "shell:", "process:", "http:", "core:window:"]) {
  if (permissions.some((permission) => String(permission).startsWith(deniedPrefix))) {
    fail(`GUI M0 capability must not grant ${deniedPrefix} permissions`);
  }
}
for (const requiredPermission of [
  "core:event:allow-listen",
  "core:event:allow-unlisten",
  "dialog:allow-open",
  "dialog:allow-save"
]) {
  if (!permissions.includes(requiredPermission)) {
    fail(`GUI capability missing narrowed permission: ${requiredPermission}`);
  }
}
for (const deniedPermission of [
  "core:event:allow-emit",
  "core:event:allow-emit-to",
  "dialog:default",
  "dialog:allow-message"
]) {
  if (permissions.includes(deniedPermission)) {
    fail(`GUI capability must not grant broad permission: ${deniedPermission}`);
  }
}
if (capabilities.windows?.length !== 1 || capabilities.windows[0] !== "main") {
  fail("GUI M0 capability must be scoped to the main window");
}

const workspaceCargo = readFileSync(resolve(root, "Cargo.toml"), "utf8");
const guiCargo = readFileSync(resolve(root, "apps/arca-gui/src-tauri/Cargo.toml"), "utf8");
if (!workspaceCargo.includes("tauri-plugin-single-instance = \"2.4.2\"")) {
  fail("workspace Cargo manifest must pin the single-instance plugin version");
}
if (!guiCargo.includes("tauri-plugin-single-instance.workspace = true")) {
  fail("GUI Tauri crate must depend on the workspace single-instance plugin");
}

const frontend = [
  "apps/arca-gui/src/api/archive-commands.ts",
  "apps/arca-gui/src/api/file-dialogs.ts",
  "apps/arca-gui/src/components/address-bar.module.css",
  "apps/arca-gui/src/components/address-bar.tsx",
  "apps/arca-gui/src/components/entry-workspace/entry-workspace.module.css",
  "apps/arca-gui/src/components/entry-workspace/archive-tree.tsx",
  "apps/arca-gui/src/components/modals/archive-modals.tsx",
  "apps/arca-gui/src/components/modals/modals.module.css",
  "apps/arca-gui/src/components/command-bar/command-bar.module.css",
  "apps/arca-gui/src/components/command-bar/command-button.tsx",
  "apps/arca-gui/src/components/command-bar/command-bar-sections.tsx",
  "apps/arca-gui/src/components/command-bar/command-bar.tsx",
  "apps/arca-gui/src/components/command-bar/index.ts",
  "apps/arca-gui/src/components/entry-workspace/entry-table-body.tsx",
  "apps/arca-gui/src/components/entry-workspace/entry-table-header.tsx",
  "apps/arca-gui/src/components/entry-workspace/entry-table.tsx",
  "apps/arca-gui/src/components/entry-workspace/entry-workspace.tsx",
  "apps/arca-gui/src/components/entry-workspace/index.ts",
  "apps/arca-gui/src/components/modals/close-blocked-modal.tsx",
  "apps/arca-gui/src/components/modals/create-archive-fields.tsx",
  "apps/arca-gui/src/components/modals/create-archive-modal.tsx",
  "apps/arca-gui/src/components/modals/direct-edit-replace-modal.tsx",
  "apps/arca-gui/src/components/modals/info-modals.tsx",
  "apps/arca-gui/src/components/modals/overwrite-prompt-modal.tsx",
  "apps/arca-gui/src/components/modals/password-prompt-modal.tsx",
  "apps/arca-gui/src/components/modals/prompt-modals.tsx",
  "apps/arca-gui/src/components/modals/unsaved-changes-modal.tsx",
  "apps/arca-gui/src/components/modals/index.ts",
  "apps/arca-gui/src/components/settings/index.ts",
  "apps/arca-gui/src/components/settings/settings-dialog.module.css",
  "apps/arca-gui/src/components/settings/settings-dialog.tsx",
  "apps/arca-gui/src/components/status-bar/status-bar.module.css",
  "apps/arca-gui/src/components/status-bar/status-bar-sections.tsx",
  "apps/arca-gui/src/components/status-bar/status-bar.tsx",
  "apps/arca-gui/src/components/status-bar/index.ts",
  "apps/arca-gui/src/components/workbench/workbench-layout.module.css",
  "apps/arca-gui/src/components/workbench/workbench-layout.tsx",
  "apps/arca-gui/src/components/workbench/workbench.tsx",
  "apps/arca-gui/src/components/workbench/index.ts",
  "apps/arca-gui/src/hooks/archive/archive-extract-destination.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-action-types.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-actions.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-archive-actions.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-extract-requests.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-requests.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-runner.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-selection-actions.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-test-requests.ts",
  "apps/arca-gui/src/hooks/archive/archive-payload-workflow.ts",
  "apps/arca-gui/src/hooks/archive/archive-session-actions.ts",
  "apps/arca-gui/src/hooks/archive/archive-session-runner.ts",
  "apps/arca-gui/src/hooks/archive/archive-session-workflow.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-action-types.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-actions.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-input-actions.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-modal-actions.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-runner.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-submit-action.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-workflow.ts",
  "apps/arca-gui/src/hooks/archive/archive-view-state.ts",
  "apps/arca-gui/src/hooks/create-archive/create-archive-state.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-delete-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-feedback-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-add-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-pending-action-types.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-pending-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-pending-history-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-runner.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-save-actions.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-replace-prompt.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-workflow-model.ts",
  "apps/arca-gui/src/hooks/direct-edit/direct-edit-workflow.ts",
  "apps/arca-gui/src/hooks/direct-edit/pending-change-state-utils.ts",
  "apps/arca-gui/src/hooks/workbench/entry-browser-state.ts",
  "apps/arca-gui/src/hooks/workbench/entry-selection-state.ts",
  "apps/arca-gui/src/hooks/workbench/native-menu-actions.ts",
  "apps/arca-gui/src/hooks/workbench/native-menu-resolvers.ts",
  "apps/arca-gui/src/hooks/workbench/operation-tracker.ts",
  "apps/arca-gui/src/hooks/workbench/prompt-dialog-resolvers.ts",
  "apps/arca-gui/src/hooks/direct-edit/pending-changes.ts",
  "apps/arca-gui/src/hooks/workbench/tauri-app-events.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-bindings.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-archive-access.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-archive-workflow-model.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-archive-workflows.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-dialogs.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-drag-drop.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-events.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-info-dialogs.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-interactions.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-model.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-ports.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-prompt-close-actions.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-prompt-dialog-model.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-prompt-dialogs.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-shortcut-handler.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-shortcuts.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-state.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-workflow-model.ts",
  "apps/arca-gui/src/hooks/workbench/workbench-workflows.ts",
  "apps/arca-gui/src/hooks/workflow-ports.ts",
  "apps/arca-gui/src/i18n/i18n-context.tsx",
  "apps/arca-gui/src/i18n/index.ts",
  "apps/arca-gui/src/i18n/messages.ts",
  "apps/arca-gui/src/main.tsx",
  "apps/arca-gui/src/settings/index.ts",
  "apps/arca-gui/src/settings/settings-context.tsx",
  "apps/arca-gui/src/models/workbench-actions.ts",
  "apps/arca-gui/src/models/workbench-layout/address-bar-model.ts",
  "apps/arca-gui/src/models/workbench-layout/command-bar-model.ts",
  "apps/arca-gui/src/models/workbench-layout/entry-workspace-model.ts",
  "apps/arca-gui/src/models/workbench-layout/index.ts",
  "apps/arca-gui/src/models/workbench-layout/status-bar-model.ts",
  "apps/arca-gui/src/models/workbench-layout/types.ts",
  "apps/arca-gui/src/models/workbench-layout/workbench-layout-model.ts",
  "apps/arca-gui/src/models/workbench-modal-model.ts",
  "apps/arca-gui/src/styles/base.css",
  "apps/arca-gui/src/styles/index.css",
  "apps/arca-gui/src/styles/tokens.css",
  "apps/arca-gui/src/shared/archive-validation.ts",
  "apps/arca-gui/src/shared/command-errors.ts",
  "apps/arca-gui/src/shared/constants.ts",
  "apps/arca-gui/src/shared/dom-utils.ts",
  "apps/arca-gui/src/shared/drop-intents.ts",
  "apps/arca-gui/src/shared/entry-list-utils.ts",
  "apps/arca-gui/src/shared/format.ts",
  "apps/arca-gui/src/shared/path-utils.ts",
  "apps/arca-gui/src/shared/pending-change-utils.ts",
  "apps/arca-gui/src/shared/prompt-utils.ts",
  "apps/arca-gui/src/shared/types.ts",
  "apps/arca-gui/src/shared/utils.ts"
]
  .map((path) => readFileSync(resolve(root, path), "utf8"))
  .join("\n");
for (const token of [
  "invoke",
  "listen",
  "getCurrentWebview",
  "onDragDropEvent",
  "@tauri-apps/plugin-dialog",
  "openDialog",
  "saveDialog",
  "startup_requests",
  "StartupRequest",
  "OperationProgress",
  "OPERATION_PROGRESS_EVENT",
  "arca-operation-progress",
  "CLOSE_BLOCKED_EVENT",
  "arca-close-blocked",
  "STARTUP_REQUESTS_EVENT",
  "arca-startup-requests",
  "startupRequest",
  "runStartupRequest",
  "CloseBlockedPromptState",
  "closeBlockedPrompt",
  "Waiting for commit",
  "Close blocked",
  "Please wait",
  "\"committing\"",
  "processed: number | null",
  "total: number | null",
  "operationProgressPercent",
  "operationProgressLabel",
  "operationProgress",
  "role=\"progressbar\"",
  "aria-valuenow",
  "begin_operation",
  "discard_operation",
  "cancel_operation",
  "withOperation",
  "cancelActiveOperation",
  "activeOperation",
  "Cancel requested",
  "CircleStop",
  "handleStartupRequest",
  "runStartupTest",
  "runStartupExtract",
  "list_archive",
  "test_archive",
  "test_selected_entries",
  "extract_archive",
  "extract_selected_entries",
  "create_archive",
  "plan_direct_edit_add",
  "save_direct_edit",
  "ArchiveManifest",
  "ArchiveValidation",
  "archiveStatusLabel",
  "archiveValidationTitle",
  "markArchiveFullyValidated",
  "refreshArchiveAfterValidation",
  "archiveManifestFullyValidated",
  "Refresh archive",
  "Test passed; refresh failed",
  "Extracted; refresh failed",
  "fullyValidatedPasswordRequired",
  "Test validated all archive payloads",
  "Extract validated all archive payloads",
  "Not tested",
  "Password required",
  "DirectEditAddPlan",
  "ExtractResult",
  "CreateResult",
  "passwordInputRef",
  "createPasswordInputRef",
  "Password required",
  "Drop archive to open",
  "selectedPaths",
  "showNativeEntryContextMenu",
  "show_entry_context_menu",
  "arca-context-extract-selection",
  "arca-context-extract-here",
  "arca-context-add-folder",
  "arca-context-copy-path",
  "entryInfoOpen",
  "Entry properties",
  "parentDirectory",
  "Test archive",
  "describeCreateDropIntent",
  "navigator.clipboard",
  "shiftKey",
  "pendingDeletePaths",
  "pendingAddEntries",
  "pendingReplaceEntries",
  "pendingUndoStack",
  "pendingRedoStack",
  "PENDING_HISTORY_LIMIT",
  "recordPendingChanges",
  "pendingChangesSnapshot",
  "restorePendingChanges",
  "undoPendingChangeSet",
  "redoPendingChangeSet",
  "clonePendingChangesSnapshot",
  "samePendingChangesSnapshot",
  "Redo available",
  "savePendingChanges",
  "planDirectEditAdd",
  "directEditReplacePrompt",
  "skipReplacementAddition",
  "confirmReplacementAddition",
  "unsavedPrompt",
  "confirmDiscardPendingChanges",
  "pendingChangesMessage",
  "handleWorkbenchShortcut",
  "isTextEditingShortcutTarget",
  "modKey",
  "event.key === \"Delete\"",
  "expectedDigestSha256",
  "Unsaved changes",
  "Unsaved changes prompt",
  "Discard",
  "Entry already exists",
  "Skip",
  "Replace",
  "Skip all",
  "overwritePrompt",
  "isOverwritePromptError",
  "Replace all",
  "chooseArchive",
  "chooseExtractDestination",
  "openCreateModal",
  "createArchive",
  "addCreateFiles",
  "addCreateFolder",
  "Password protect",
  "createEncryptionAllowed",
  "isZipOutputPath",
  "Password is only available for ZIP archives",
  "Password is available for ZIP output",
  "createSingleStreamOutput",
  "isSingleStreamOutputPath",
  "Single-stream outputs require exactly one file input",
  "dropTarget",
  "address-bar.module.css",
  "command-bar.module.css",
  "entry-workspace.module.css",
  "modals.module.css",
  "Archive information",
  "MENU_ACTION_EVENT",
  "arca-menu-action",
  "runNativeMenuAction",
  "statusBar",
  "entryFilter",
  "entrySort",
  "Filter entries",
  "Clear filter",
  "tableScroll",
  "columnSort",
  "sortListEntries",
  "listEntryMatchesFilter",
  "modKey && key === \"f\"",
  "I18nProvider",
  "useI18n",
  "SUPPORTED_LOCALES",
  "currentLocale",
  "set_native_menu_locale",
  "SettingsDialog",
  "AppSettingsProvider",
  "useAppSettings",
  "OPEN_SETTINGS_EVENT",
  "arca-menu-settings",
  "settings-dialog.module.css",
  "SUPPORTED_ARCHIVE_EXTENSIONS",
  "CREATE_ARCHIVE_EXTENSIONS",
  "currentAppSettings",
  "archiveFormatCapabilities",
  "fileAssociationStatus",
  "setNativeFileAssociation",
  "setNativeFileAssociations",
  "refreshFileAssociations",
  "fileAssociations",
  "fileAssociationStatus",
  "settings.fileTypes",
  "settings.fileAssociations",
  "settings.fileAssociationsLoading",
  "settings.refreshAssociations",
  "settings.registerAll",
  "settings.unregisterAll",
  "data-show-packed-size",
  "data-show-encrypted-column",
  "extensionGrid",
  "한국어"
]) {
  if (!frontend.includes(token)) {
    fail(`GUI frontend smoke token missing: ${token}`);
  }
}
if (frontend.includes("plugin-fs") || frontend.includes("plugin-shell")) {
  fail("GUI frontend must not import filesystem or shell plugins in M0");
}
if (frontend.includes("sourcePath")) {
  fail("GUI frontend must not receive Direct Editing source paths in plan DTOs");
}
if (/const\s*\[\s*password\s*,[^=]+useState/i.test(frontend)
  || /const\s*\[\s*[^,\]]*password[^,\]]*,[^=]+useState/i.test(frontend)
  || /useState\s*\([^)]*password[^)]*\)/i.test(frontend)) {
  fail("GUI frontend must not keep plaintext passwords in React state");
}

const backend = [
  "apps/arca-gui/src-tauri/src/lib.rs",
  "apps/arca-gui/src-tauri/src/file_associations.rs",
  "apps/arca-gui/src-tauri/src/menus.rs",
  "apps/arca-gui/src-tauri/src/operations.rs",
  "apps/arca-gui/src-tauri/src/startup.rs"
]
  .map((path) => readFileSync(resolve(root, path), "utf8"))
  .join("\n");
if (backend.includes("source_path:")) {
  fail("GUI Tauri DTOs must not expose Direct Editing source paths");
}
for (const token of [
  "#[tauri::command]",
  "fn health",
  "fn startup_requests",
  "fn close_current_window",
  "fn begin_operation",
  "fn discard_operation",
  "fn cancel_operation",
  "fn archive_format_capabilities",
  "fn file_association_status",
  "fn set_file_association",
  "fn set_all_file_associations",
  "windows_notify_association_changed",
  "SHChangeNotify",
  "target_os = \"linux\"",
  "LINUX_DESKTOP_ID",
  "com.crasnec.arca.desktop",
  "mimeapps.list",
  "kde-mimeapps.list",
  "XDG_CONFIG_HOME",
  "XDG_DATA_HOME",
  "Default Applications",
  "Added Associations",
  "linux_ensure_desktop_entry",
  "linux_set_mime_association",
  "linux_desktop_entry_contents",
  "mimeapps_prepend_desktop_id",
  "mimeapps_remove_desktop_ids",
  "format::archive_formats",
  "format::descriptor_for_extension",
  "format::detect_format_with_signature",
  "StartupRequest",
  "StartupAction",
  "OperationRegistry",
  "OperationProgress",
  "OperationPhase",
  "GuiArchiveManifest",
  "GuiArchiveValidation",
  "GuiDirectEditAddPlan",
  "GuiDirectEditPlannedEntry",
  "DirectEditPendingEntry",
  "pending_add_entries",
  "GuiArchiveManifest::from_core",
  "archive_validation",
  "metadataOnlyPasswordRequired",
  "claimed: Arc<AtomicBool>",
  "fn claim",
  "discard_unclaimed",
  "compare_exchange(false, true",
  "CoreProgress",
  "CoreProgressPhase",
  "CancellationToken",
  "OperationContext",
  "ProgressSink",
  "begin_tracked_operation",
  "fail_tracked_operation",
  "finish_tracked_operation",
  "core_context",
  "emit_core_progress",
  "emit_operation",
  "OPERATION_PROGRESS_EVENT",
  "arca-operation-progress",
  "CLOSE_BLOCKED_EVENT",
  "arca-close-blocked",
  "STARTUP_REQUESTS_EVENT",
  "arca-startup-requests",
  "MENU_ACTION_EVENT",
  "arca-menu-action",
  "build_native_menu",
  "show_entry_context_menu",
  "build_entry_context_menu",
  "handle_native_menu_event",
  "MenuBuilder",
  "SubmenuBuilder",
  "MenuItemBuilder",
  "popup_menu_at",
  "arca-context-extract-selection",
  "arca-context-properties",
  ".menu(build_native_menu)",
  ".on_menu_event(handle_native_menu_event)",
  "CloseBlockedPayload",
  "handle_window_close_requested",
  "handle_app_run_event",
  "handle_single_instance_startup",
  "tauri_plugin_single_instance::init",
  "get_webview_window",
  "unminimize",
  "set_focus",
  "WindowEvent::CloseRequested",
  "RunEvent::ExitRequested",
  "prevent_close",
  "prevent_exit",
  "committing_labels",
  "pending_exit_code",
  "defer_exit",
  "take_deferred_exit_if_idle",
  "RESTART_EXIT_CODE",
  "app.exit(exit_code)",
  ".build(tauri::generate_context!())",
  "app.run(handle_app_run_event)",
  "try_state::<OperationRegistry>",
  "compress_with_context",
  "extract_with_context",
  "extract_selection_with_context",
  "test_with_context",
  "test_selection_with_context",
  "inspect_archive_with_context",
  "Committing",
  "operation_id: Option<u64>",
  "startup_requests_from_args",
  "startup_requests_from_args_in_cwd",
  "startup_path_from_arg",
  "--arca-shell-open",
  "--arca-shell-test",
  "--arca-shell-extract",
  "async fn list_archive",
  "async fn test_archive",
  "async fn test_selected_entries",
  "async fn extract_archive",
  "async fn extract_selected_entries",
  "async fn create_archive",
  "async fn delete_selected_entries",
  "async fn plan_direct_edit_add",
  "async fn save_direct_edit",
  "spawn_blocking",
  "format::detect_format",
  "FileAssociationStatus",
  "FileAssociationEntry",
  "GuiArchiveFormatCapability",
  "inspect_archive",
  "CompressOptions",
  "DeleteSelectionOptions",
  "DirectEditSaveOptions",
  "PlanDirectEditAddOptions",
  "TestOptions",
  "TestSelectionOptions",
  "ExtractOptions",
  "ExtractSelectionOptions",
  "Encryption::Aes256",
  "delete_selection",
  "core_plan_direct_edit_add",
  "core_save_direct_edit",
  "overwrite",
  "Password",
  "password_from_string",
  "tauri_plugin_dialog::init",
  "arca_native::native_backend_enabled"
]) {
  if (!backend.includes(token)) {
    fail(`GUI backend smoke token missing: ${token}`);
  }
}
for (const token of ["std::process", "Command::new", "shell::", "plugin_shell"]) {
  if (backend.includes(token)) {
    fail(`GUI backend must not invoke external processes in M0: ${token}`);
  }
}

const coreOps = readFileSync(resolve(root, "crates/arca-core/src/ops.rs"), "utf8");
for (const token of [
  "ArchiveFormatDescriptor",
  "FormatSignature",
  "archive_formats",
  "archive_file_extensions",
  "descriptor_for_extension",
  "detect_format_with_signature",
  "format_matches_signature",
  "ZIP_SIGNATURES",
  "TAR_SIGNATURES",
  "GZIP_SIGNATURES",
  "BZIP2_SIGNATURES",
  "XZ_SIGNATURES",
  "supports_direct_edit",
  "application/x-compressed-tar",
  "application/x-bzip-compressed-tar",
  "application/x-xz-compressed-tar"
]) {
  if (!coreFormat.includes(token)) {
    fail(`core format registry smoke token missing: ${token}`);
  }
}
if (backend.includes("SUPPORTED_FILE_ASSOCIATION_EXTENSIONS")) {
  fail("GUI backend file associations must derive from the core format registry");
}
for (const token of [
  "CoreProgressPhase::Extracting",
  "Extracting archive entry payload",
  "Extracting single-stream payload",
  "Extracting tar entries",
  "Testing ZIP entries",
  "Testing tar entries",
  "Testing single-stream payload",
  "Rewriting ZIP entries",
  "Compressing ZIP file data",
  "Extracting ZIP entries",
  "Testing selected ZIP entries",
  "SharedProgressCounter",
  "expected_size",
  "Some(rewrite_total)"
]) {
  if (!coreOps.includes(token)) {
    fail(`core progress smoke token missing: ${token}`);
  }
}
if (coreOps.includes("Copying archive payload\", total, Some(max_bytes)")) {
  fail("core archive payload progress must use operation-specific phases instead of a generic copy message");
}

const sourceArchiveScript = readFileSync(resolve(root, "scripts/source-archive.sh"), "utf8");
for (const token of ["--exclude=apps/arca-gui/dist", "npm ci", "gui:smoke", "gui:web:build"]) {
  if (!sourceArchiveScript.includes(token)) {
    fail(`source archive GUI coverage token missing: ${token}`);
  }
}

const nsisHook = readFileSync(
  resolve(root, "apps/arca-gui/src-tauri/windows/nsis-shell-context.nsh"),
  "utf8"
);
const installedAssociationExts = nsisHookMacroExtensions(nsisHook, "ARCA_REGISTER_FILE_ASSOCIATION");
const uninstalledAssociationExts = nsisHookMacroExtensions(nsisHook, "ARCA_UNREGISTER_FILE_ASSOCIATION");
const installedShellExts = nsisHookMacroExtensions(nsisHook, "ARCA_REGISTER_ARCHIVE_SHELL");
const uninstalledShellExts = nsisHookMacroExtensions(nsisHook, "ARCA_UNREGISTER_ARCHIVE_SHELL");
if (!sameItems(installedAssociationExts, expectedAssociationExts)) {
  fail(`Windows file association install extensions must match core formats: ${expectedAssociationExts.join(", ")}`);
}
if (!sameItems(uninstalledAssociationExts, expectedAssociationExts)) {
  fail(`Windows file association uninstall extensions must match core formats: ${expectedAssociationExts.join(", ")}`);
}
if (!sameItems(installedShellExts, expectedAssociationExts)) {
  fail(`Windows shell context install extensions must match file associations: ${expectedAssociationExts.join(", ")}`);
}
if (!sameItems(uninstalledShellExts, expectedAssociationExts)) {
  fail(`Windows shell context uninstall extensions must match file associations: ${expectedAssociationExts.join(", ")}`);
}
if (!sameItems(installedShellExts, uninstalledShellExts)) {
  fail("Windows shell context install/uninstall extension sets must be symmetric");
}
if (!nsisHook.includes('WriteRegStr HKCU "Software\\Classes\\.${EXT}" "" "Arca.${EXT}"')) {
  fail("Windows installer must register Arca ProgID as the per-user default handler");
}
if (!nsisHook.includes('WriteRegStr HKCU "Software\\Classes\\.${EXT}\\OpenWithProgids" "Arca.${EXT}" ""')) {
  fail("Windows installer must register Arca in OpenWithProgids");
}
if (!nsisHook.includes('WriteRegStr HKCU "Software\\Classes\\Arca.${EXT}\\DefaultIcon" "" "$INSTDIR\\arca-gui.exe,0"')) {
  fail("Windows installer must register file association icons");
}
if (!nsisHook.includes('WriteRegStr HKCU "Software\\Classes\\Arca.${EXT}\\shell\\open\\command" "" "$\\"$INSTDIR\\arca-gui.exe$\\" --arca-shell-open $\\"%1$\\""')) {
  fail("Windows installer must register the default open command");
}
if (!nsisHook.includes("SHChangeNotify")) {
  fail("Windows installer must notify Explorer after association changes");
}
for (const action of [
  {
    key: "ArcaOpen",
    label: "Open in Arca",
    flag: "--arca-shell-open"
  },
  {
    key: "ArcaExtract",
    label: "Extract with Arca",
    flag: "--arca-shell-extract"
  },
  {
    key: "ArcaTest",
    label: "Test with Arca",
    flag: "--arca-shell-test"
  }
]) {
  const baseKey = `Software\\Classes\\SystemFileAssociations\\.$\{EXT}\\shell\\${action.key}`;
  if (!nsisHook.includes(`WriteRegStr HKCU "${baseKey}" "" "${action.label}"`)) {
    fail(`Windows shell context hook missing label registration for ${action.key}`);
  }
  if (!nsisHook.includes(`WriteRegStr HKCU "${baseKey}" "Icon" "$INSTDIR\\arca-gui.exe,0"`)) {
    fail(`Windows shell context hook missing icon registration for ${action.key}`);
  }
  if (!nsisHook.includes(`WriteRegStr HKCU "${baseKey}\\command" "" "$\\"$INSTDIR\\arca-gui.exe$\\" ${action.flag} $\\"%1$\\""`)) {
    fail(`Windows shell context hook missing quoted command registration for ${action.key}`);
  }
  if (!nsisHook.includes(`DeleteRegKey HKCU "${baseKey}"`)) {
    fail(`Windows shell context hook missing uninstall key removal for ${action.key}`);
  }
}
for (const unsupportedExt of unsupportedAssociationExts) {
  if (nsisHook.includes(`"${unsupportedExt}"`) || nsisHook.includes(`.${unsupportedExt}`)) {
    fail(`Windows shell context hook must not register unsupported extension: ${unsupportedExt}`);
  }
}

console.log("gui smoke ok");

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

function sameItems(actual, expected) {
  const uniqueActual = [...new Set(actual)].sort();
  const uniqueExpected = [...new Set(expected)].sort();
  return uniqueActual.length === uniqueExpected.length
    && uniqueActual.every((value, index) => value === uniqueExpected[index]);
}

function nsisHookMacroExtensions(source, macroName) {
  return [...source.matchAll(new RegExp(`!insertmacro\\s+${macroName}\\s+"([^"]+)"`, "g"))]
    .map((match) => match[1]);
}

function parseCoreFormatDescriptors(source) {
  const descriptors = [];
  for (const match of source.matchAll(/ArchiveFormatDescriptor\s*\{([\s\S]*?)\n\s*\},/g)) {
    const block = match[1];
    const extensionMatch = block.match(/extensions:\s*&\[([^\]]*)\]/);
    const mimeMatch = block.match(/mime_type:\s*"([^"]+)"/);
    if (!extensionMatch || !mimeMatch) {
      continue;
    }
    const extensions = [...extensionMatch[1].matchAll(/"([^"]+)"/g)].map((item) => item[1]);
    if (extensions.length > 0) {
      descriptors.push({
        extensions,
        mimeType: mimeMatch[1]
      });
    }
  }
  if (descriptors.length === 0) {
    fail("core format registry descriptors could not be parsed");
  }
  return descriptors;
}
