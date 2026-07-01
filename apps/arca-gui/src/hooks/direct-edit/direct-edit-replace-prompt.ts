import React from "react";
import type {
  DirectEditPlannedEntry,
  DirectEditReplacePromptState
} from "../../shared/types";

type DirectEditReplacePromptInput = {
  appendPendingAddPlan: (
    inputs: string[],
    additions: DirectEditPlannedEntry[],
    replacements: DirectEditPlannedEntry[]
  ) => void;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
};

export function useDirectEditReplacePrompt({
  appendPendingAddPlan,
  setStatus
}: DirectEditReplacePromptInput) {
  const [directEditReplacePrompt, setDirectEditReplacePrompt] =
    React.useState<DirectEditReplacePromptState | null>(null);

  function closeDirectEditReplacePrompt() {
    setDirectEditReplacePrompt(null);
    setStatus("Ready");
  }

  function finishReplacementPrompt(
    prompt: DirectEditReplacePromptState,
    replacements: DirectEditPlannedEntry[]
  ) {
    setDirectEditReplacePrompt(null);
    appendPendingAddPlan(prompt.inputs, prompt.plan.additions, replacements);
  }

  function skipReplacementAdditions() {
    const prompt = directEditReplacePrompt;
    if (!prompt) {
      return;
    }
    finishReplacementPrompt(prompt, prompt.acceptedReplacements);
  }

  function confirmReplacementAdditions() {
    const prompt = directEditReplacePrompt;
    if (!prompt) {
      return;
    }
    finishReplacementPrompt(prompt, [...prompt.acceptedReplacements, ...prompt.plan.replacements]);
  }

  function advanceReplacementPrompt(
    prompt: DirectEditReplacePromptState,
    acceptedReplacements: DirectEditPlannedEntry[],
    nextStatus: string
  ) {
    const remainingReplacements = prompt.plan.replacements.slice(1);
    if (remainingReplacements.length === 0) {
      finishReplacementPrompt(prompt, acceptedReplacements);
      return;
    }
    setDirectEditReplacePrompt({
      ...prompt,
      acceptedReplacements,
      plan: {
        ...prompt.plan,
        replacements: remainingReplacements
      }
    });
    setStatus(nextStatus);
  }

  function skipReplacementAddition() {
    const prompt = directEditReplacePrompt;
    if (!prompt) {
      return;
    }
    advanceReplacementPrompt(prompt, prompt.acceptedReplacements, "Replacement skipped");
  }

  function confirmReplacementAddition() {
    const prompt = directEditReplacePrompt;
    const current = prompt?.plan.replacements[0];
    if (!prompt || !current) {
      return;
    }
    advanceReplacementPrompt(
      prompt,
      [...prompt.acceptedReplacements, current],
      "Replacement accepted"
    );
  }

  return {
    directEditReplacePrompt,
    setDirectEditReplacePrompt,
    closeDirectEditReplacePrompt,
    skipReplacementAddition,
    confirmReplacementAddition,
    skipReplacementAdditions,
    confirmReplacementAdditions
  };
}
