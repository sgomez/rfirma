import { type FormEvent, type Ref, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { trapTabWithinCurrentTarget } from "./focusTrap";

interface PasswordPromptProps {
  ref: Ref<HTMLDivElement>;
  labelledBy: string;
  onSubmit: (password: string) => void;
  onCancel: () => void;
}

/**
 * La contraseña **del fichero** `.p12`, tecleada antes de elegirlo.
 *
 * Ese orden no es un descuido: el selector de ficheros lo abre el backend y no
 * la ventana (ID-63), así que la orden de instalar llega con la contraseña ya
 * puesta y el selector aparece después. La contraseña no se guarda en ningún
 * estado que sobreviva al envío — de un `.p12` instalado no se recuerda nada,
 * ni la ruta ni la contraseña (ID-195, ID-196).
 *
 * Es un `.rf-dialog` propio porque aquí **todavía no hay
 * certificado**: ese diálogo se identifica por el titular con el que se va a
 * firmar, y aquí no se sabe ni cuál es ni cuántos trae el fichero.
 */
export function PasswordPrompt({ ref, labelledBy, onSubmit, onCancel }: PasswordPromptProps) {
  const { t } = useTranslation();
  const [typed, setTyped] = useState("");
  const field = useId();
  const box = useRef<HTMLInputElement>(null);

  // El foco entra en el campo y no en el marco: es lo único que se puede hacer
  // dentro de este diálogo, y quien lo abrió venía de pulsar «Añadir…».
  useEffect(() => {
    box.current?.focus();
  }, []);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSubmit(typed);
  };

  return (
    <div
      className="rf-dialog preferences__password"
      role="dialog"
      aria-modal="true"
      tabIndex={-1}
      ref={ref}
      aria-labelledby={labelledBy}
      onKeyDown={trapTabWithinCurrentTarget}
    >
      <p className="rf-title" id={labelledBy}>
        {t("pin.titlePassword")}
      </p>
      <form onSubmit={submit}>
        <div className="rf-field">
          <label className="rf-label" htmlFor={field}>
            {t("pin.labelPassword")}
          </label>
          <input
            id={field}
            className="rf-input"
            type="password"
            ref={box}
            value={typed}
            onChange={(event) => setTyped(event.target.value)}
          />
        </div>
        <div className="rf-row preferences__confirm-actions">
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </button>
          <button type="submit" className="rf-btn rf-btn--primary">
            {t("preferences.certificates.password.submit")}
          </button>
        </div>
      </form>
    </div>
  );
}
