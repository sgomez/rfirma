import type { TFunction } from "i18next";
import { type FormEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Certificate, CertificateStoreClass } from "./certificate";
import "./PinDialog.css";
import type { TokenFailure } from "./token";

interface PinDialogProps {
  /** Con qué identidad se firma. Se enseña para que se vea antes de teclear. */
  certificate: Certificate;
  /**
   * El fallo del intento anterior, o `null` la primera vez. Solo llega aquí
   * `incorrectPin`: es el único que se resuelve dentro del diálogo (ver
   * `belongsToPinDialog`).
   */
  failure: TokenFailure | null;
  onSubmit: (pin: string) => void;
  onCancel: () => void;
}

/**
 * Qué palabra usa el diálogo, y qué le pide al almacén (ID-188).
 *
 * «PIN» para un módulo PKCS#11; «contraseña» para un fichero —un perfil NSS de
 * navegador, un `.p12` instalado—. No se discrimina por hardware: se diverge a
 * propósito de AutoFirma, que le dice «contraseña» al módulo genérico.
 */
function wordFor(store: CertificateStoreClass): "pin" | "password" {
  return store === "card" ? "pin" : "password";
}

/**
 * Qué se está abriendo, **en los términos de quien firma**, y solo si se sabe
 * (ID-208): el perfil de un navegador, o el titular y el DNI de un `.p12`. Con
 * un módulo PKCS#11 no se nombra nada — ni la clase de almacén, ni el token.
 */
function subjectFor(certificate: Certificate, t: TFunction): string | null {
  switch (certificate.store) {
    case "firefox":
      return t("pin.subjectBrowser", { browser: "Firefox" });
    case "chrome":
      return t("pin.subjectBrowser", { browser: "Chrome" });
    case "nssdb":
      return t("pin.signingAs", {
        holder: certificate.holderName,
        idNumber: certificate.idNumber,
      });
    case "card":
      return null;
  }
}

/**
 * El diálogo del secreto del almacén (docs/design/dialogo-pin.md).
 *
 * La palabra la elige el almacén (ID-188) y el contenido no lleva pista de
 * ninguna clase: ni la clase de módulo PKCS#11, ni el nombre del token, ni una
 * frase que tranquilice sobre dónde se guarda el secreto (ID-208). Un secreto
 * incorrecto se reintenta aquí mismo, con el campo vacío y un error de una
 * línea, sin remedio debajo: no hay contador de reintentos porque PKCS#11 no
 * lo cuenta nunca (ID-191), y no hay «tarjeta bloqueada» porque la v0.4 retira
 * tarjetas y DNIe del alcance.
 */
export function PinDialog({ certificate, failure, onSubmit, onCancel }: PinDialogProps) {
  const { t } = useTranslation();
  const [pin, setPin] = useState("");
  const titleId = useId();
  const pinId = useId();
  const errorId = useId();

  const word = wordFor(certificate.store);
  const subject = subjectFor(certificate, t);
  const incorrect = failure?.situation === "incorrectPin";

  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSubmit(pin);
    // El campo se vacía en cuanto se envía: si el secreto estaba mal, lo
    // siguiente que hay que hacer es teclearlo entero otra vez, no corregir un
    // carácter.
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
            {word === "pin" ? t("pin.title") : t("pin.titlePassword")}
          </p>
          <p className="rf-prose rf-text-muted pin-dialog__subject">{subject}</p>
        </div>

        <form onSubmit={submit}>
          <div className={incorrect ? "rf-field rf-field--error" : "rf-field"}>
            <label className="rf-label" htmlFor={pinId}>
              {word === "pin" ? t("pin.label") : t("pin.labelPassword")}
            </label>
            <input
              className={
                word === "pin" ? "rf-input pin-dialog__pin" : "rf-input pin-dialog__password"
              }
              id={pinId}
              type="password"
              value={pin}
              autoComplete="off"
              aria-invalid={incorrect}
              aria-describedby={errorId}
              onChange={(event) => setPin(event.target.value)}
            />
            {/* Nada debajo del campo salvo el mensaje de fallo: ni pista, ni
                promesa, ni contador de intentos. */}
            <p className="rf-hint pin-dialog__error" id={errorId}>
              {incorrect ? (word === "pin" ? t("pin.incorrect") : t("pin.incorrectPassword")) : ""}
            </p>
          </div>

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
      </div>
    </div>
  );
}
