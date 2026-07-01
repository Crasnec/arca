import type { CommandError } from "./types";

export function isOverwritePromptError(error: CommandError) {
  return error.code === "usage" && (error.message ?? "").includes("already exists");
}
