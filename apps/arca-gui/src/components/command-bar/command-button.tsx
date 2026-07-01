import type { LucideIcon } from "lucide-react";
import styles from "./command-bar.module.css";

export type CommandButtonProps = {
  icon: LucideIcon;
  label: string;
  title: string;
  disabled?: boolean;
  ariaLabel?: string;
  onClick?: () => void | Promise<void>;
};

export function CommandButton({
  icon: Icon,
  label,
  title,
  disabled = false,
  ariaLabel,
  onClick
}: CommandButtonProps) {
  return (
    <button
      type="button"
      className={styles.button}
      title={title}
      disabled={disabled}
      aria-label={ariaLabel}
      onClick={onClick ? () => void onClick() : undefined}
    >
      <Icon size={20} aria-hidden="true" />
      <span>{label}</span>
    </button>
  );
}
