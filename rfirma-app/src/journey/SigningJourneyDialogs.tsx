//! Los diálogos del recorrido de firma: los dos avisos previos al PIN y el progreso de las tres etapas.

import { useTranslation } from "react-i18next";
import { SignAnywayDialog } from "../signing/SignAnywayDialog";
import { SigningProgressDialog } from "../signing/SigningProgressDialog";
import { UnsealedPagesDialog } from "../signing/UnsealedPagesDialog";
import type { SigningJourney } from "./useSigningJourney";

/** Los diálogos que abre el recorrido; la ventana los monta una sola vez. */
export function SigningJourneyDialogs({ journey }: { journey: SigningJourney }) {
  const { i18n } = useTranslation();
  const { sealLoss, signAnyway, stage } = journey.dialogs;
  return (
    <>
      {sealLoss !== null && (
        <UnsealedPagesDialog
          fallen={sealLoss.fallen}
          onConfirm={() => void sealLoss.confirm()}
          onCancel={sealLoss.cancel}
        />
      )}
      {signAnyway !== null && (
        <SignAnywayDialog
          problems={signAnyway.problems}
          locale={i18n.resolvedLanguage ?? i18n.language}
          onConfirm={() => void signAnyway.confirm()}
          onCancel={signAnyway.cancel}
        />
      )}
      {stage !== null && <SigningProgressDialog stage={stage} />}
    </>
  );
}
