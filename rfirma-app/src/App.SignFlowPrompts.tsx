//! Los dos avisos que la firma interpone antes del PIN, abiertos por `useSignFlow`, y si alguno está abierto.

import type { useSignFlow } from "./App.useSignFlow";
import { SignAnywayDialog } from "./signing/SignAnywayDialog";
import { UnsealedPagesDialog } from "./signing/UnsealedPagesDialog";

type SignFlow = ReturnType<typeof useSignFlow>;

export function signFlowPromptOpen(flow: SignFlow): boolean {
  return flow.sealLossPrompt !== null || flow.signAnywayPrompt !== null;
}

export function SignFlowPrompts({ flow, locale }: { flow: SignFlow; locale: string }) {
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
