import { useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon, CheckIcon, FileIcon, FolderIcon, InfoIcon } from "../design-system/icons";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import { Switch } from "../preferences/Switch";
import { CertificateSelect, statusWarning } from "./CertificateSelect";
import type { Certificate } from "./certificate";
import { isUsable } from "./certificate";
import type { Destination } from "./destination";
import { shortenDestination } from "./destination";
import type { SigningFailure } from "./failure";
import type { Rubric, RubricFailure } from "./rubric";
import "./SigningPanel.css";
import type { Layer2Composer, SigningIdentity, VisibleSignature } from "./visibleSignature";

/** El documento que se va a firmar, con lo que el panel enseña de él. */
export interface SigningDocument {
  name: string;
  pages: number;
  /** El tamaño, o `null` mientras nadie lo sepa: no se inventa un cero. */
  sizeBytes: number | null;
  /**
   * Cuántas firmas trae ya. Cualquier número mayor que cero es una cofirma, y
   * `null` es **no se sabe todavía**: la insignia de la bandeja dice si el PDF
   * está firmado, pero no cuántas veces, y el aviso de cofirma necesita el
   * número. Callar es mejor que decir «1 firma» a ojo.
   */
  signatures: number | null;
}

/**
 * En qué punto está la elección del certificado. Son los estados de la ficha;
 * «Listo» es `chosen` con un certificado en vigor.
 *
 * `failed` es el aterrizaje del rechazo (ID-10): sin él la búsqueda que falla
 * no tenía dónde caer y la ficha se quedaba en `loading` para siempre. No es lo
 * mismo que `empty` —«no hay ninguno» y «no he podido buscarlos» son cosas
 * distintas— y por eso son dos estados y no un booleano dentro de uno.
 */
export type CertificateState =
  | { kind: "loading" }
  | { kind: "empty" }
  | { kind: "failed"; failure: NamedFailure }
  /**
   * Hay certificados y **ninguno elegido**, que es lo que pasa la primera vez
   * con varios: el disparador dice «Elegir certificado» y el botón de firmar
   * sigue apagado. Elegir con qué identidad se firma un documento con validez
   * jurídica no lo hace la aplicación por su cuenta.
   */
  | { kind: "unchosen"; certificates: readonly Certificate[] }
  /**
   * Uno elegido, **y los demás al lado**: el desplegable los sigue listando,
   * porque cambiar de certificado es abrirlo otra vez y no un botón aparte.
   */
  | { kind: "chosen"; certificate: Certificate; certificates: readonly Certificate[] };

interface SigningPanelProps {
  document: SigningDocument;
  certificate: CertificateState;
  /** Cuál se elige en el desplegable. */
  onChooseCertificate: (certificate: Certificate) => void;
  onRetryCertificates: () => void;
  onChooseModule: () => void;
  signature: VisibleSignature;
  onChangeSignature: (signature: VisibleSignature) => void;
  /** En qué página está el recuadro, o `null` si aún no se ha colocado. */
  page: number | null;
  rubric: Rubric | null;
  /** El último fallo al elegir la rúbrica, que se cuenta aquí y no al firmar. */
  rubricFailure: RubricFailure | null;
  onChooseRubric: () => void;
  /** Quien compone el texto del recuadro. Ver [`Layer2Composer`]. */
  composer: Layer2Composer;
  /**
   * La fecha y hora que llevará el recuadro, **ya formateadas**.
   *
   * Viene de arriba y no se calcula aquí porque tiene que ser **la misma** que
   * se envíe a firmar: la vista previa enseña el texto que se va a estampar, y
   * un reloj propio en este componente enseñaría uno y estamparía otro.
   */
  signedAt: string;
  destination: Destination;
  onChangeDestination: () => void;
  onSign: () => void;
  /** Mientras la firma corre, el botón no acepta un segundo empujón. */
  signing: boolean;
  failure: SigningFailure | null;
}

/**
 * La columna derecha: **todo lo que hay que decidir antes de firmar**, y el
 * botón que firma (docs/design/panel-de-firma.md).
 *
 * Es la **única región de la aplicación con un botón primario**, y el botón va
 * al final: el panel entero se lee como una decisión que termina en una acción.
 *
 * Dos cosas que parecen detalles y son la ficha entera:
 *
 * - **No hay comodines.** El contenido del recuadro se marca con casillas y el
 *   texto lo compone Rust ya resuelto (ID-19). La vista previa enseña esa misma
 *   cadena, pedida a [`Layer2Composer`], y no una imitación local.
 * - **La miniatura de la rúbrica es honesta.** Enseña el fichero ya
 *   normalizado, que es un JPEG y por tanto opaco: un PNG con transparencia se
 *   ve aquí con su fondo blanco, antes de firmar y no dentro del PDF (ID-24).
 */
export function SigningPanel({
  document,
  certificate,
  onChooseCertificate,
  onRetryCertificates,
  onChooseModule,
  signature,
  onChangeSignature,
  page,
  rubric,
  rubricFailure,
  onChooseRubric,
  composer,
  signedAt,
  destination,
  onChangeDestination,
  onSign,
  signing,
  failure,
}: SigningPanelProps) {
  const { t, i18n } = useTranslation();
  const reasonId = useId();
  const chosen = certificate.kind === "chosen" ? certificate.certificate : null;
  const usable = chosen !== null && isUsable(chosen.status);
  const language = i18n.resolvedLanguage ?? i18n.language;
  // Memoizado porque es la dependencia del efecto que pide la vista previa: un
  // objeto nuevo en cada pintada la pediría en bucle.
  const signer = useMemo<SigningIdentity | null>(
    () => (chosen === null ? null : { certificate: chosen.id, signedAt, language }),
    [chosen, signedAt, language],
  );
  const preview = useLayer2Preview(composer, signature, signer);
  // El destino recortado. Sin nombre compuesto —la carpeta no se deja
  // comprobar— se enseña el del documento, que es lo único que se sabe.
  const shortened = shortenDestination({
    folder: destination.folder,
    name: destination.name ?? document.name,
  });

  const changeField = (field: keyof VisibleSignature["fields"], checked: boolean) => {
    onChangeSignature({ ...signature, fields: { ...signature.fields, [field]: checked } });
  };

  return (
    <div className="panel">
      <div className="panel__scroll">
        <div className="panel__header">
          <span className="panel__header-icon">
            <FileIcon />
          </span>
          <div className="panel__header-text">
            <p className="rf-title panel__document">{document.name}</p>
            <p className="rf-body rf-text-muted">
              {[
                t("panel.document.pages", { count: document.pages }),
                document.sizeBytes === null ? null : formatSize(document.sizeBytes, i18n.language),
              ]
                .filter((piece) => piece !== null)
                .join(" · ")}
            </p>
          </div>
        </div>

        {document.signatures !== null && document.signatures > 0 && (
          <div className="panel__co-signature">
            <span className="panel__notice-icon">
              <InfoIcon />
            </span>
            <p className="rf-prose">{t("panel.coSignature", { count: document.signatures })}</p>
          </div>
        )}

        <hr className="rf-divider" />

        <section className="panel__section" aria-label={t("panel.certificate.title")}>
          <p className="rf-label panel__heading">{t("panel.certificate.title")}</p>
          <CertificateBlock
            state={certificate}
            onChoose={onChooseCertificate}
            onRetry={onRetryCertificates}
            onChooseModule={onChooseModule}
          />
        </section>

        <hr className="rf-divider" />

        <section
          className={usable ? "panel__section" : "panel__section panel__section--inert"}
          aria-label={t("panel.visibleSignature.title")}
          inert={!usable}
        >
          <p className="rf-label panel__heading">{t("panel.visibleSignature.title")}</p>
          <Switch
            checked={signature.enabled}
            label={t("panel.visibleSignature.toggle")}
            onChange={(enabled) => onChangeSignature({ ...signature, enabled })}
          />

          {signature.enabled && (
            <>
              <p className="rf-hint">
                {page === null
                  ? t("panel.visibleSignature.noPlacement")
                  : t("panel.visibleSignature.placement", { page })}
              </p>

              <fieldset className="panel__fields">
                <legend className="rf-label">{t("panel.visibleSignature.content")}</legend>
                <Checkbox
                  checked={signature.rubric && rubric !== null}
                  disabled={rubric === null}
                  label={t("panel.visibleSignature.fields.rubric")}
                  hint={rubric === null ? t("panel.visibleSignature.fields.rubricDisabled") : null}
                  onChange={(checked) => onChangeSignature({ ...signature, rubric: checked })}
                />
                <Checkbox
                  checked={signature.fields.signerName}
                  label={t("panel.visibleSignature.fields.signerName")}
                  onChange={(checked) => changeField("signerName", checked)}
                />
                <Checkbox
                  checked={signature.fields.idNumber}
                  label={t("panel.visibleSignature.fields.idNumber")}
                  onChange={(checked) => changeField("idNumber", checked)}
                />
                <Checkbox
                  checked={signature.fields.signedAt}
                  label={t("panel.visibleSignature.fields.signedAt")}
                  onChange={(checked) => changeField("signedAt", checked)}
                />
                <Checkbox
                  checked={signature.fields.reason}
                  label={t("panel.visibleSignature.fields.reason")}
                  onChange={(checked) => changeField("reason", checked)}
                />
              </fieldset>

              {signature.fields.reason && (
                <div className="rf-field">
                  <label className="rf-label" htmlFor={reasonId}>
                    {t("panel.visibleSignature.reason.label")}
                  </label>
                  <input
                    className="rf-input"
                    id={reasonId}
                    type="text"
                    value={signature.reason}
                    placeholder={t("panel.visibleSignature.reason.placeholder")}
                    onChange={(event) =>
                      onChangeSignature({ ...signature, reason: event.target.value })
                    }
                  />
                </div>
              )}

              <div className="panel__rubric">
                <p className="rf-label">{t("panel.visibleSignature.rubric.title")}</p>
                <div className="panel__rubric-row">
                  {rubric && (
                    <img
                      className="panel__rubric-thumbnail"
                      src={rubric.dataUrl}
                      width={rubric.width}
                      height={rubric.height}
                      alt={t("panel.visibleSignature.rubric.thumbnail")}
                    />
                  )}
                  <button
                    type="button"
                    className="rf-btn rf-btn--secondary panel__rubric-choose"
                    onClick={onChooseRubric}
                  >
                    {rubric
                      ? t("panel.visibleSignature.rubric.change")
                      : t("panel.visibleSignature.rubric.choose")}
                  </button>
                </div>
                {rubric && (
                  <p className="rf-hint">{t("panel.visibleSignature.rubric.flattened")}</p>
                )}
                {rubricFailure && (
                  <ErrorNotice
                    situation={rubricFailure.situation}
                    technicalDetail={rubricFailure.detail}
                  />
                )}
              </div>

              <div className="panel__preview">
                <p className="rf-label">{t("panel.visibleSignature.preview.title")}</p>
                <Layer2Preview text={preview} />
              </div>
            </>
          )}
        </section>
      </div>

      <footer className="panel__footer">
        {failure ? (
          <ErrorNotice situation={failure.situation} technicalDetail={failure.detail} />
        ) : (
          <div className="panel__destination">
            {/* El rótulo es una promesa, así que **desaparece** cuando no se
                puede cumplir: con la carpeta no escribible el pie dice solo
                que no se puede escribir en ella, y no las dos cosas a la vez. */}
            {destination.writable && <p className="rf-label">{t("panel.footer.savedIn")}</p>}
            <div className="rf-row rf-gap-xs panel__destination-row">
              <span className="panel__destination-icon">
                <FolderIcon />
              </span>
              {/* El destino son **dos cosas**: la carpeta, atenuada y precedida
                  de `…/` —hay carpetas por encima y no se afirma cuáles—, y el
                  nombre sin atenuar, que es el dato (ID-63). El recorte lo
                  decide `shortenDestination`; la línea envuelve antes que
                  cortarse, así que aquí no hay ninguna elipsis de CSS.

                  El aviso de que no se puede escribir **no se recorta**: es una
                  frase entera y perderla por elipsis sería perder el aviso
                  cuando más falta hace. */}
              {destination.writable ? (
                <p className="rf-prose panel__destination-path">
                  <span className="rf-text-muted">{`…/${shortened.folder}/`}</span>
                  {shortened.name}
                </p>
              ) : (
                <p className="rf-prose panel__destination-unwritable">
                  {t("panel.footer.unwritable", { folder: shortened.folder })}
                </p>
              )}
              <button
                type="button"
                className="rf-btn rf-btn--ghost panel__destination-change"
                onClick={onChangeDestination}
              >
                {t("actions.change")}
              </button>
            </div>
          </div>
        )}
        <button
          type="button"
          className="rf-btn rf-btn--primary panel__sign"
          disabled={!usable || signing}
          onClick={onSign}
        >
          {failure ? t("panel.footer.retry") : t("actions.sign")}
        </button>
      </footer>
    </div>
  );
}

/** El certificado, en sus cinco estados. */
function CertificateBlock({
  state,
  onChoose,
  onRetry,
  onChooseModule,
}: {
  state: CertificateState;
  onChoose: (certificate: Certificate) => void;
  onRetry: () => void;
  onChooseModule: () => void;
}) {
  const { t, i18n } = useTranslation();

  if (state.kind === "loading") {
    return (
      <div className="panel__skeletons">
        <p className="rf-prose rf-text-muted">{t("panel.certificate.loading")}</p>
        <span className="panel__skeleton" />
        <span className="panel__skeleton" />
      </div>
    );
  }

  if (state.kind === "empty") {
    return (
      <div className="panel__no-certificates">
        <div className="panel__notice-title">
          <AlertIcon />
          <span className="rf-title">{t("panel.certificate.empty.title")}</span>
        </div>
        <p className="rf-prose rf-text-muted">{t("panel.certificate.empty.body")}</p>
        <div className="rf-row rf-gap-xs panel__no-certificates-actions">
          <button type="button" className="rf-btn rf-btn--secondary panel__retry" onClick={onRetry}>
            {t("panel.certificate.empty.retry")}
          </button>
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onChooseModule}>
            {t("panel.certificate.empty.otherModule")}
          </button>
        </div>
      </div>
    );
  }

  if (state.kind === "failed") {
    // El mismo lenguaje que `empty` —título, explicación, botón de volver a
    // buscar— con texto propio, más el fallo ya clasificado con su detalle
    // crudo debajo: quien firma tiene que poder distinguir «mete la tarjeta»
    // de «algo va mal» (ID-10).
    return (
      <div className="panel__no-certificates">
        <div className="panel__notice-title">
          <AlertIcon />
          <span className="rf-title">{t("panel.certificate.failed.title")}</span>
        </div>
        <p className="rf-prose rf-text-muted">{t("panel.certificate.failed.body")}</p>
        <ErrorNotice situation={state.failure.situation} technicalDetail={state.failure.detail} />
        <div className="rf-row rf-gap-xs panel__no-certificates-actions">
          <button type="button" className="rf-btn rf-btn--secondary panel__retry" onClick={onRetry}>
            {t("panel.certificate.failed.retry")}
          </button>
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onChooseModule}>
            {t("panel.certificate.failed.otherModule")}
          </button>
        </div>
      </div>
    );
  }

  // Elegido o no, el hueco lo ocupa el mismo desplegable: cambiar de
  // certificado es volver a abrirlo, y por eso el botón `Cambiar` de la tarjeta
  // ya no existe.
  const chosen = state.kind === "chosen" ? state.certificate : null;
  return (
    <>
      <CertificateSelect certificates={state.certificates} chosen={chosen} onChoose={onChoose} />
      {/* Con uno solo se elige solo, así que puede quedar puesto uno que no
          sirve: el aviso se queda debajo del disparador, donde estaba en la
          tarjeta. Elegido de la lista esto no se ve nunca, porque las filas
          inutilizables no se dejan elegir. */}
      {chosen !== null && !isUsable(chosen.status) && (
        <p className="rf-prose panel__certificate-warning" role="alert">
          {statusWarning(chosen.status, i18n.language, t)}
        </p>
      )}
    </>
  );
}

/** Una casilla. No sale del sistema de diseño: se maqueta con tokens. */
function Checkbox({
  checked,
  disabled = false,
  label,
  hint = null,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  label: string;
  hint?: string | null;
  onChange: (checked: boolean) => void;
}) {
  const hintId = useId();

  return (
    <div className="panel__checkbox">
      <label className="panel__checkbox-label">
        <input
          className="panel__checkbox-input"
          type="checkbox"
          checked={checked}
          disabled={disabled}
          aria-describedby={hint ? hintId : undefined}
          onChange={(event) => onChange(event.target.checked)}
        />
        <span className="panel__checkbox-box" aria-hidden="true">
          {checked && <CheckIcon />}
        </span>
        <span className="rf-prose">{label}</span>
      </label>
      {hint && (
        <p className="rf-hint panel__checkbox-hint" id={hintId}>
          {hint}
        </p>
      )}
    </div>
  );
}

/** Lo que va a decir el recuadro, tal cual lo compuso el backend. */
function Layer2Preview({ text }: { text: string | null }) {
  const { t } = useTranslation();

  if (text === null) {
    return <p className="rf-hint">{t("panel.visibleSignature.preview.unavailable")}</p>;
  }
  if (text.trim() === "") {
    return <p className="rf-hint">{t("panel.visibleSignature.preview.empty")}</p>;
  }
  return <pre className="panel__preview-text">{text}</pre>;
}

/**
 * Pide el texto del recuadro cada vez que cambian las casillas.
 *
 * La respuesta que llega tarde se descarta: dos cambios seguidos pueden
 * resolverse en orden inverso, y la vista previa enseñaría un estado anterior
 * al de las casillas que se ven marcadas.
 */
function useLayer2Preview(
  composer: Layer2Composer,
  signature: VisibleSignature,
  signer: SigningIdentity | null,
): string | null {
  const [preview, setPreview] = useState<string | null>(null);

  useEffect(() => {
    if (signer === null || !signature.enabled) {
      setPreview(null);
      return;
    }
    let current = true;
    void composer.compose(signature, signer).then((text) => {
      if (current) setPreview(text);
    });
    return () => {
      current = false;
    };
  }, [composer, signature, signer]);

  return preview;
}

/** «2,4 MB». El tamaño en la unidad que el usuario reconoce, no en bytes. */
export function formatSize(bytes: number, locale: string): string {
  const megabytes = bytes / 1_000_000;
  const format = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
  if (megabytes >= 1) return `${format.format(megabytes)} MB`;
  return `${format.format(bytes / 1000)} kB`;
}
