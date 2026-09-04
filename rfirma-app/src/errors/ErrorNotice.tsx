import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";
import type { Catalog } from "../i18n/catalog";
import "./ErrorNotice.css";

/**
 * Las situaciones que sabemos nombrar. Hoy solo la genérica: el mapeo de los
 * `CKR_*` de `cryptoki` y de las excepciones del puente es de otro sub-issue,
 * y cada situación que añada entra aquí y en `po/messages.pot`.
 */
export type ErrorSituation = keyof Catalog["errors"]["situations"];

/**
 * Las situaciones que se cuentan en **un solo renglón**: el título lo dice
 * todo, y lo que iría debajo sería jerga o el remedio obvio (ID-211).
 *
 * Son las que no tienen `body` en el catálogo, así que la lista no es un gusto:
 * `tsc` la obliga a cuadrar con las claves que existen.
 */
const ONE_LINE = ["keyNotRsa"] as const;

type OneLineSituation = (typeof ONE_LINE)[number];

function isOneLine(situation: ErrorSituation): situation is OneLineSituation {
  return (ONE_LINE as readonly string[]).includes(situation);
}

interface ErrorNoticeProps {
  /** Nuestra situación, que sí está traducida. */
  situation: ErrorSituation;
  /**
   * El texto original tal cual llegó: el `CKR_*` de `cryptoki` o el mensaje
   * incrustado de la excepción del puente. **No se traduce ni se recorta**:
   * está para pegarlo en un informe de fallo.
   *
   * Sobra —y no se pone— en una situación de un solo renglón: el detalle crudo
   * de una clave elíptica es la curva, que no le sirve a nadie que esté delante
   * de esta pantalla.
   */
  technicalDetail?: string;
}

/**
 * Un error, como manda el ID-29: una **situación** nuestra traducida y, aparte,
 * el texto original crudo en un detalle plegado.
 *
 * Los errores no se traducen, se clasifican. `cryptoki` devuelve códigos y el
 * puente Java devuelve excepciones cuyo texto está incrustado en el código
 * —`afirma-crypto-pdf` no tiene ni un `.properties` localizado—, así que
 * ninguno de los dos se enseña como mensaje. Lo que no sepamos clasificar cae
 * en `unknown` más su detalle técnico crudo (ADR-0009).
 *
 * El artboard del error de firma dibuja el detalle **desplegado**. Eso es un
 * estado congelado, no el inicial (ID-43): aquí sigue plegado, porque el
 * `CKR_*` crudo debajo del mensaje ocupa el pie entero y solo lo necesita quien
 * va a escribir un informe de fallo.
 */
export function ErrorNotice({ situation, technicalDetail }: ErrorNoticeProps) {
  const { t } = useTranslation();

  return (
    <div className="error-notice" role="alert">
      <p className="error-notice__title">
        <AlertIcon />
        <span className="rf-title">{t(`errors.situations.${situation}.title`)}</span>
      </p>
      {!isOneLine(situation) && (
        <>
          <p className="rf-prose">
            {t(`errors.situations.${situation as Exclude<ErrorSituation, OneLineSituation>}.body`)}
          </p>
          <details className="error-notice__detail">
            <summary className="rf-body rf-text-muted">{t("errors.technicalDetail")}</summary>
            <pre className="error-notice__raw">{technicalDetail}</pre>
          </details>
        </>
      )}
    </div>
  );
}
