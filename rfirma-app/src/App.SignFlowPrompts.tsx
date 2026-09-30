/** Los tres avisos que la firma interpone antes del PIN, abiertos por `useSignFlow`. */

import type { useSignFlow } from "./App.useSignFlow";
import { InvalidPreviousSignaturesDialog } from "./signing/InvalidPreviousSignaturesDialog";
import { UnregisteredSignaturesDialog } from "./signing/UnregisteredSignaturesDialog";
import { UnsealedPagesDialog } from "./signing/UnsealedPagesDialog";

type SignFlow = ReturnType<typeof useSignFlow>;

export function signFlowPromptOpen(flow: SignFlow): boolean {
  return (
    flow.unregisteredPrompt !== null ||
    flow.sealLossPrompt !== null ||
    flow.invalidPreviousSignaturesPrompt !== null
  );
}

export function SignFlowPrompts({ flow, locale }: { flow: SignFlow; locale: string }) {
  return (
    <>
      {flow.unregisteredPrompt !== null && (
        <UnregisteredSignaturesDialog
          onConfirm={() => void flow.signWithUnregisteredSignatures()}
          onCancel={() => flow.setUnregisteredPrompt(null)}
        />
      )}
      {flow.sealLossPrompt !== null && (
        <UnsealedPagesDialog
          fallen={flow.sealLossPrompt.fallen}
          chosen={flow.sealLossPrompt.chosen}
          onConfirm={() => void flow.signAnyway()}
          onCancel={() => flow.setSealLossPrompt(null)}
        />
      )}
      {flow.invalidPreviousSignaturesPrompt !== null && (
        <InvalidPreviousSignaturesDialog
          signatures={flow.invalidPreviousSignaturesPrompt}
          locale={locale}
          onConfirm={() => void flow.signDespiteInvalidPreviousSignatures()}
          onCancel={() => flow.setInvalidPreviousSignaturesPrompt(null)}
        />
      )}
    </>
  );
}
