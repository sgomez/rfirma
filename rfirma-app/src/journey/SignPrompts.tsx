//! Los dos avisos que la firma interpone antes del PIN, y si alguno está abierto.

import type { SigningProblem } from "../signing/previousSignatures";
import { SignAnywayDialog } from "../signing/SignAnywayDialog";
import { UnsealedPagesDialog } from "../signing/UnsealedPagesDialog";

type SignPromptsFlow = {
  sealLossPrompt: { fallen: number } | null;
  setSealLossPrompt: (prompt: null) => void;
  signAnyway: () => Promise<void>;
  signAnywayPrompt: readonly SigningProblem[] | null;
  setSignAnywayPrompt: (prompt: null) => void;
  signDespiteProblems: () => Promise<void>;
};

export function signPromptOpen(flow: SignPromptsFlow): boolean {
  return flow.sealLossPrompt !== null || flow.signAnywayPrompt !== null;
}

export function SignPrompts({ flow, locale }: { flow: SignPromptsFlow; locale: string }) {
  return (
    <>
      {flow.sealLossPrompt !== null && (
        <UnsealedPagesDialog
          fallen={flow.sealLossPrompt.fallen}
          onConfirm={() => void flow.signAnyway()}
          onCancel={() => flow.setSealLossPrompt(null)}
        />
      )}
      {flow.signAnywayPrompt !== null && (
        <SignAnywayDialog
          problems={flow.signAnywayPrompt}
          locale={locale}
          onConfirm={() => void flow.signDespiteProblems()}
          onCancel={() => flow.setSignAnywayPrompt(null)}
        />
      )}
    </>
  );
}
