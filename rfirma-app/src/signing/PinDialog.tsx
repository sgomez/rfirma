import type { TFunction } from "i18next";
import { type FormEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Certificate } from "./certificate";
import "./PinDialog.css";
import type { TokenFailure } from "./token";

interface PinDialogProps {
  /** Con qué identidad se firma. Se enseña para que se vea antes de teclear. */
  certificate: Certificate;
  /**
   * El fallo del intento anterior, o `null` la primera vez. Solo llegan aquí
   * los que se resuelven dentro del diálogo (ver `belongsToPinDialog`).
   */
  failure: TokenFailure | null;
  onSubmit: (pin: string) => void;
  onCancel: () => void;
}

/**
 * El diálogo del PIN (docs/design/dialogo-pin.md).
 *
 * Aparece **después de la prefirma**: pedir el secreto que desbloquea la clave
 * sin saber todavía qué se va a firmar no tiene sentido.
 *
 * Dos cosas que la ficha pide y no son decoración:
 *
 * - **Un PIN incorrecto se reintenta aquí mismo.** El diálogo no se desmonta y
 *   el recorrido no vuelve a empezar: se vacía el campo, se enseñan los
 *   intentos que quedan y se teclea otra vez.
 * - **Los intentos restantes se enseñan.** Los cuenta el módulo PKCS#11;
 *   callarlos y dejar que alguien bloquee su tarjeta es un daño real y no
 *   siempre reversible.
 *
 * Sin color de error: el sistema de diseño no lo tiene. El fallo se señala con
 * borde, peso y glifo (`.rf-field--error`).
 */
export function PinDialog({ certificate, failure, onSubmit, onCancel }: PinDialogProps) {
  const { t } = useTranslation();
  const [pin, setPin] = useState("");
  const titleId = useId();
  const pinId = useId();
  const hintId = useId();

  const locked = failure?.situation === "pinLocked";
  const incorrect = failure?.situation === "incorrectPin";

  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSubmit(pin);
    // El campo se vacía en cuanto se envía: si el PIN estaba mal, lo siguiente
    // que hay que hacer es teclearlo entero otra vez, no corregir un carácter.
    setPin("");
  };

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog pin-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="pin-dialog__heading">
          <p className="rf-title" id={titleId}>
            {locked ? t("errors.situations.pinLocked.title") : t("pin.title")}
          </p>
          <p className="rf-prose rf-text-muted">
            {t("pin.signingAs", {
              holder: certificate.holderName,
              idNumber: certificate.idNumber,
            })}
          </p>
        </div>

        {locked && failure ? (
          <>
            <p className="rf-prose" role="alert">
              {t("errors.situations.pinLocked.body")}
            </p>
            <TechnicalDetail detail={failure.detail} />
            <hr className="rf-divider" />
            <div className="rf-row pin-dialog__actions">
              <button type="button" className="rf-btn rf-btn--primary" onClick={onCancel}>
                {t("actions.close")}
              </button>
            </div>
          </>
        ) : (
          <form onSubmit={submit}>
            <div className={incorrect ? "rf-field rf-field--error" : "rf-field"}>
              <label className="rf-label" htmlFor={pinId}>
                {t("pin.label")}
              </label>
              <input
                className="rf-input pin-dialog__pin"
                id={pinId}
                type="password"
                value={pin}
                autoComplete="off"
                aria-describedby={hintId}
                aria-invalid={incorrect}
                onChange={(event) => setPin(event.target.value)}
              />
              <p className="rf-hint" id={hintId}>
                {incorrect && failure ? attemptsHint(failure.attemptsLeft, t) : t("pin.hint")}
              </p>
            </div>

            {incorrect && failure && <TechnicalDetail detail={failure.detail} />}

            <hr className="rf-divider" />

            <div className="rf-row pin-dialog__actions">
              <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
                {t("actions.cancel")}
              </button>
              <button type="submit" className="rf-btn rf-btn--primary">
                {t("pin.submit")}
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}

/**
 * El texto original del token, aparte y crudo (ID-29). No se traduce ni se
 * recorta: está para pegarlo en un informe de fallo.
 */
function TechnicalDetail({ detail }: { detail: string }) {
  const { t } = useTranslation();

  return (
    <details className="pin-dialog__detail">
      <summary className="rf-body rf-text-muted">{t("errors.technicalDetail")}</summary>
      <pre className="pin-dialog__raw">{detail}</pre>
    </details>
  );
}

/** Los intentos que quedan, cuando el módulo los cuenta. */
function attemptsHint(attemptsLeft: number | null, t: TFunction): string {
  if (attemptsLeft === null) return t("pin.incorrectUnknown");
  return t("pin.incorrect", { count: attemptsLeft });
}
