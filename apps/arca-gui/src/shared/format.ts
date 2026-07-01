import type { OperationProgress } from "./types";

export function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }
  const units = ["KB", "MB", "GB", "TB"];
  let scaled = value / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && scaled >= 1024; index += 1) {
    scaled /= 1024;
    unit = units[index];
  }
  return `${scaled.toFixed(scaled >= 10 ? 1 : 2)} ${unit}`;
}

export function operationProgressPercent(operation: OperationProgress) {
  if (operation.total === null || operation.total <= 0 || operation.processed === null) {
    return null;
  }
  const percent = Math.round((Math.min(operation.processed, operation.total) / operation.total) * 100);
  return Math.max(0, Math.min(percent, 100));
}

export function operationProgressLabel(operation: OperationProgress) {
  const percent = operationProgressPercent(operation);
  if (percent !== null) {
    return `${percent}%`;
  }
  if (operation.processed !== null) {
    return formatBytes(operation.processed);
  }
  return operation.phase;
}
