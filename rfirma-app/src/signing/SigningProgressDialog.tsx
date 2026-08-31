import { useId } from "react";
import { useTranslation } from "react-i18next";
import { SIGNING_STAGES, type SigningStage } from "./flow";
import "./SigningProgressDialog.css";

interface SigningProgressDialogProps {
  /** En qué etapa va la firma ahora mismo. */
  stage: SigningStage;
}

/**
 * El diálogo de progreso (docs/design/dialogo-progreso-firma.md).
 *
 * **Bloquea la ventana**, y no por costumbre: no hay nada que hacer mientras
 * corre, y retirar la tarjeta a mitad rompe la firma. Por eso no lleva
 * «Cancelar» ni cruz de cerrar —una vez empezada la firma en la tarjeta no hay
 * marcha atrás— y por eso el velo cubre la ventana entera.
 *
 * Las **tres** etapas se enseñan porque la postfirma regenera el PDF entero y
 * puede tardar: sin desglose, una espera larga después de teclear el PIN parece
 * un cuelgue. Y cuando algo falla, saber en qué fase fue es lo primero que hace
 * falta.
 */
export function SigningProgressDialog({ stage }: SigningProgressDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const current = SIGNING_STAGES.indexOf(stage);

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog progress-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <p className="rf-title" id={titleId}>
          {t("progress.title")}
        </p>

        <ol className="progress-dialog__stages">
          {SIGNING_STAGES.map((each, index) => {
            const state = index < current ? "done" : index === current ? "running" : "pending";
            const term = TERM_KEY[each];
            return (
              <li className={`progress-dialog__stage progress-dialog__stage--${state}`} key={each}>
                <span className="progress-dialog__mark" aria-hidden="true">
                  {state === "done" ? "✓" : state === "running" ? "●" : "○"}
                </span>
                <span className="rf-prose">
                  {t(`progress.stages.${each}`)}
                  {term && (
                    <span className="rf-text-muted progress-dialog__term">
                      {` (${t(`progress.stages.${term}`)})`}
                    </span>
                  )}
                </span>
                <span className="rf-hint progress-dialog__state">
                  {t(`progress.states.${state}`)}
                </span>
              </li>
            );
          })}
        </ol>

        <div
          className="progress-dialog__bar"
          role="progressbar"
          aria-valuemin={1}
          aria-valuemax={SIGNING_STAGES.length}
          aria-valuenow={current + 1}
          aria-labelledby={titleId}
        >
          <span
            className="progress-dialog__bar-fill"
            style={{ width: `${((current + 1) / SIGNING_STAGES.length) * 100}%` }}
          />
        </div>

        <p className="rf-prose">{t("progress.keepTheCard")}</p>
      </div>
    </div>
  );
}

/**
 * El término del dominio de cada etapa, entre paréntesis. La firma no tiene:
 * «firmando en la tarjeta» ya dice exactamente lo que pasa, y es la única de
 * las tres que toca la clave privada.
 */
const TERM_KEY: Record<SigningStage, string | null> = {
  presign: "presignTerm",
  sign: null,
  postsign: "postsignTerm",
};
