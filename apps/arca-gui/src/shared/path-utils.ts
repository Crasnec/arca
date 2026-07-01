export function basename(path: string) {
  const normalized = path.replace(/[\\/]+$/, "");
  return normalized.split(/[\\/]/).pop() || path;
}

export function parentDirectory(path: string) {
  const normalized = path.replace(/[\\/]+$/, "");
  const separatorIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  return separatorIndex > 0 ? normalized.slice(0, separatorIndex) : "";
}

export function normalizeSelectedPaths(selected: string | string[] | null) {
  if (!selected) {
    return [];
  }
  const paths = Array.isArray(selected) ? selected : [selected];
  return [...new Set(paths.filter((path) => path.trim() !== ""))];
}

export function isZipOutputPath(path: string) {
  return path.trim().toLowerCase().endsWith(".zip");
}

export function isSingleStreamOutputPath(path: string) {
  const normalized = path.trim().toLowerCase();
  if (
    normalized.endsWith(".tar.gz") ||
    normalized.endsWith(".tar.bz2") ||
    normalized.endsWith(".tar.xz")
  ) {
    return false;
  }
  return (
    normalized.endsWith(".gz") ||
    normalized.endsWith(".bz2") ||
    normalized.endsWith(".xz")
  );
}
