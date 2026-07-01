import React from "react";
import { FileType2, RotateCw, Rows3, Settings, type LucideIcon, X } from "lucide-react";
import { type Locale, useI18n } from "../../i18n";
import {
  type ArchiveExtension,
  type CreateArchiveExtension,
  type ViewDensity,
  useAppSettings
} from "../../settings";
import { CREATE_ARCHIVE_EXTENSIONS } from "../../shared/constants";
import type { FileAssociationEntry } from "../../api/file-associations";
import styles from "./settings-dialog.module.css";

export type SettingsDialogProps = {
  open: boolean;
  close: () => void;
};

type SettingsGroup = "general" | "view" | "fileTypes";

export function SettingsDialog({ open, close }: SettingsDialogProps) {
  const { locale, setLocale, t } = useI18n();
  const {
    archiveFormats,
    density,
    showPackedSize,
    showEncryptedColumn,
    defaultArchiveExtension,
    fileAssociationStatus,
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
  } = useAppSettings();
  const [activeGroup, setActiveGroup] = React.useState<SettingsGroup>("general");
  const fileAssociationEntries = fileAssociationStatus?.entries ?? [];
  const canManageFileAssociations = Boolean(fileAssociationStatus?.supported) && !fileAssociationsLoading;
  const createArchiveExtensions = React.useMemo(() => {
    const extensions = archiveFormats.flatMap((format) =>
      format.createSuffixes.map((suffix) => suffix.replace(/^\./, ""))
    );
    return extensions.length > 0 ? extensions : [...CREATE_ARCHIVE_EXTENSIONS];
  }, [archiveFormats]);

  React.useEffect(() => {
    if (!open) {
      return undefined;
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        close();
      }
    }
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [close, open]);

  if (!open) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation" onMouseDown={close}>
      <section
        className={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-label={t("settings.title")}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className={styles.header}>
          <div className={styles.title}>{t("settings.title")}</div>
          <button
            type="button"
            className={styles.closeButton}
            title={t("settings.close")}
            aria-label={t("settings.close")}
            onClick={close}
          >
            <X size={16} aria-hidden="true" />
          </button>
        </header>
        <div className={styles.content}>
          <nav className={styles.groupList} aria-label={t("settings.groups")}>
            <GroupButton
              icon={Settings}
              label={t("settings.general")}
              selected={activeGroup === "general"}
              onClick={() => setActiveGroup("general")}
            />
            <GroupButton
              icon={Rows3}
              label={t("settings.view")}
              selected={activeGroup === "view"}
              onClick={() => setActiveGroup("view")}
            />
            <GroupButton
              icon={FileType2}
              label={t("settings.fileTypes")}
              selected={activeGroup === "fileTypes"}
              onClick={() => setActiveGroup("fileTypes")}
            />
          </nav>
          <div className={styles.panel}>
            {activeGroup === "general" && (
              <section className={styles.section}>
                <div className={styles.panelTitle}>{t("settings.general")}</div>
                <label className={styles.settingRow} htmlFor="settings-language">
                  <span>{t("settings.language")}</span>
                  <select
                    id="settings-language"
                    value={locale}
                    onChange={(event) => setLocale(event.target.value as Locale)}
                  >
                    <option value="ko">{t("settings.languageKo")}</option>
                    <option value="en">{t("settings.languageEn")}</option>
                  </select>
                </label>
                <label className={styles.settingRow} htmlFor="settings-default-archive">
                  <span>{t("settings.defaultArchiveType")}</span>
                  <select
                    id="settings-default-archive"
                    value={defaultArchiveExtension}
                    onChange={(event) =>
                      setDefaultArchiveExtension(event.target.value as CreateArchiveExtension)
                    }
                  >
                    {createArchiveExtensions.map((extension) => (
                      <option key={extension} value={extension}>
                        .{extension}
                      </option>
                    ))}
                  </select>
                </label>
              </section>
            )}
            {activeGroup === "view" && (
              <section className={styles.section}>
                <div className={styles.panelTitle}>{t("settings.view")}</div>
                <div className={styles.settingRow}>
                  <span>{t("settings.density")}</span>
                  <div className={styles.segmented} role="group" aria-label={t("settings.density")}>
                    <DensityButton
                      value="comfortable"
                      selected={density === "comfortable"}
                      label={t("settings.densityComfortable")}
                      setDensity={setDensity}
                    />
                    <DensityButton
                      value="compact"
                      selected={density === "compact"}
                      label={t("settings.densityCompact")}
                      setDensity={setDensity}
                    />
                  </div>
                </div>
                <CheckboxSetting
                  id="settings-show-packed"
                  label={t("settings.showPackedSize")}
                  checked={showPackedSize}
                  onChange={setShowPackedSize}
                />
                <CheckboxSetting
                  id="settings-show-encrypted"
                  label={t("settings.showEncryptedColumn")}
                  checked={showEncryptedColumn}
                  onChange={setShowEncryptedColumn}
                />
              </section>
            )}
            {activeGroup === "fileTypes" && (
              <section className={styles.section}>
                <div className={styles.panelHeader}>
                  <div className={styles.panelTitle}>{t("settings.fileTypes")}</div>
                  <div className={styles.panelTools}>
                    <button
                      type="button"
                      disabled={fileAssociationsLoading}
                      onClick={() => void refreshFileAssociations()}
                    >
                      <RotateCw size={14} aria-hidden="true" />
                      <span>{t("settings.refreshAssociations")}</span>
                    </button>
                    <button
                      type="button"
                      disabled={!canManageFileAssociations}
                      onClick={() => void setAllFileAssociations(true)}
                    >
                      {t("settings.registerAll")}
                    </button>
                    <button
                      type="button"
                      disabled={!canManageFileAssociations}
                      onClick={() => void setAllFileAssociations(false)}
                    >
                      {t("settings.unregisterAll")}
                    </button>
                  </div>
                </div>
                {fileAssociationsLoading && (
                  <div className={styles.statusNote}>{t("settings.fileAssociationsLoading")}</div>
                )}
                {fileAssociationStatus && !fileAssociationStatus.supported && (
                  <div className={styles.statusNote}>
                    {fileAssociationStatus.message || t("settings.fileAssociationsUnsupported")}
                  </div>
                )}
                {fileAssociationsError && (
                  <div className={styles.errorNote}>{fileAssociationsError}</div>
                )}
                <div className={styles.settingBlock}>
                  <span>{t("settings.fileAssociations")}</span>
                  <div className={styles.extensionGrid} aria-label={t("settings.fileAssociations")}>
                    {fileAssociationEntries.map((entry) => (
                      <ExtensionCheckbox
                        key={entry.extension}
                        entry={entry}
                        checked={fileAssociations[entry.extension] ?? entry.enabled}
                        disabled={!canManageFileAssociations}
                        setFileAssociation={setFileAssociation}
                      />
                    ))}
                  </div>
                </div>
              </section>
            )}
          </div>
        </div>
        <footer className={styles.actions}>
          <button type="button" onClick={close}>
            {t("settings.done")}
          </button>
        </footer>
      </section>
    </div>
  );
}

function GroupButton({
  icon: Icon,
  label,
  selected,
  onClick
}: {
  icon: LucideIcon;
  label: string;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={selected ? styles.groupSelected : undefined}
      aria-current={selected ? "page" : undefined}
      onClick={onClick}
    >
      <Icon size={16} aria-hidden="true" />
      <span>{label}</span>
    </button>
  );
}

function DensityButton({
  value,
  selected,
  label,
  setDensity
}: {
  value: ViewDensity;
  selected: boolean;
  label: string;
  setDensity: (density: ViewDensity) => void;
}) {
  return (
    <button
      type="button"
      className={selected ? styles.segmentSelected : undefined}
      aria-pressed={selected}
      onClick={() => setDensity(value)}
    >
      {label}
    </button>
  );
}

function CheckboxSetting({
  id,
  label,
  checked,
  onChange
}: {
  id: string;
  label: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className={styles.checkboxSetting} htmlFor={id}>
      <span>{label}</span>
      <input
        id={id}
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}

function ExtensionCheckbox({
  entry,
  checked,
  disabled,
  setFileAssociation
}: {
  entry: FileAssociationEntry;
  checked: boolean;
  disabled: boolean;
  setFileAssociation: (extension: ArchiveExtension, value: boolean) => Promise<void>;
}) {
  const { extension } = entry;
  const id = `settings-extension-${extension}`;
  return (
    <label className={styles.extensionItem} htmlFor={id}>
      <input
        id={id}
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(event) => void setFileAssociation(extension, event.target.checked)}
      />
      <span>.{extension}</span>
    </label>
  );
}
