import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { ErrorNotice, type ErrorSituation } from "../errors/ErrorNotice";
import { Switch } from "../preferences/Switch";
import type { Certificate } from "./certificate";
import { isUsable } from "./certificate";
import type { Rubric, RubricFailure } from "./rubric";
import "./SigningPanel.css";
import type { Layer2Composer, VisibleSignature } from "./visibleSignature";

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
 * En qué punto está la elección del certificado. Son los cuatro primeros
 * estados de la ficha; «Listo» es `chosen` con un certificado en vigor.
 */
export type CertificateState =
  | { kind: "loading" }
  | { kind: "empty" }
  | { kind: "unchosen" }
  | { kind: "chosen"; certificate: Certificate };

/** La carpeta de destino, por su **nombre**: la ruta no se enseña (ADR-0011). */
export interface Destination {
  folder: string;
  /** Si se puede escribir ahí. Falso no apaga el botón: ofrece cambiarla. */
  writable: boolean;
}

/** Un fallo de la firma, ya clasificado por el backend (ID-29). */
export interface SigningFailure {
  situation: ErrorSituation;
  /** El texto original crudo: `CKR_DEVICE_REMOVED (C_Sign)`. */
  detail: string;
}

interface SigningPanelProps {
  document: SigningDocument;
  certificate: CertificateState;
  onChooseCertificate: () => void;
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
  const preview = useLayer2Preview(composer, signature, usable);

  const changeField = (field: keyof VisibleSignature["fields"], checked: boolean) => {
    onChangeSignature({ ...signature, fields: { ...signature.fields, [field]: checked } });
  };

  return (
    <div className="panel">
      <div className="panel__scroll">
        <section className="panel__section">
          <p className="rf-prose panel__document">{document.name}</p>
          <p className="rf-hint">
            {[
              t("panel.document.pages", { pages: document.pages }),
              document.sizeBytes === null ? null : formatSize(document.sizeBytes, i18n.language),
            ]
              .filter((piece) => piece !== null)
              .join(" · ")}
          </p>
        </section>

        {document.signatures !== null && document.signatures > 0 && (
          <p className="rf-hint panel__co-signature">
            {document.signatures === 1
              ? t("panel.coSignature.one")
              : t("panel.coSignature.many", { count: document.signatures })}
          </p>
        )}

        <hr className="rf-divider" />

        <section className="panel__section" aria-label={t("panel.certificate.title")}>
          <p className="rf-label">{t("panel.certificate.title")}</p>
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
          <p className="rf-label">{t("panel.visibleSignature.title")}</p>
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
                {rubric && (
                  <>
                    <img
                      className="panel__rubric-thumbnail"
                      src={rubric.dataUrl}
                      width={rubric.width}
                      height={rubric.height}
                      alt={t("panel.visibleSignature.rubric.thumbnail")}
                    />
                    <p className="rf-hint">{t("panel.visibleSignature.rubric.flattened")}</p>
                  </>
                )}
                <button type="button" className="rf-btn rf-btn--secondary" onClick={onChooseRubric}>
                  {rubric
                    ? t("panel.visibleSignature.rubric.change")
                    : t("panel.visibleSignature.rubric.choose")}
                </button>
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
          <div className="rf-row panel__destination">
            <p className="rf-hint">
              {destination.writable
                ? `${t("panel.footer.savedIn")} ${destination.folder}`
                : t("panel.footer.unwritable", { folder: destination.folder })}
            </p>
            <button type="button" className="rf-btn rf-btn--ghost" onClick={onChangeDestination}>
              {t("actions.change")}
            </button>
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

/** El certificado, en sus cuatro estados. */
function CertificateBlock({
  state,
  onChoose,
  onRetry,
  onChooseModule,
}: {
  state: CertificateState;
  onChoose: () => void;
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
        <p className="rf-prose">{t("panel.certificate.empty.title")}</p>
        <p className="rf-hint">{t("panel.certificate.empty.body")}</p>
        <div className="rf-row">
          <button type="button" className="rf-btn rf-btn--secondary" onClick={onRetry}>
            {t("panel.certificate.empty.retry")}
          </button>
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onChooseModule}>
            {t("panel.certificate.empty.otherModule")}
          </button>
        </div>
      </div>
    );
  }

  if (state.kind === "unchosen") {
    return (
      <button type="button" className="rf-btn rf-btn--secondary" onClick={onChoose}>
        {t("panel.certificate.choose")}
      </button>
    );
  }

  const { certificate } = state;
  return (
    <div className="rf-card panel__certificate">
      <p className="rf-prose">{certificate.holderName}</p>
      <p className="rf-hint">{certificate.idNumber}</p>
      <p className="rf-hint">{t("panel.certificate.issuer", { issuer: certificate.issuer })}</p>
      {!isUsable(certificate.status) && (
        <p className="rf-prose panel__certificate-warning" role="alert">
          {statusWarning(certificate.status, i18n.language, t)}
        </p>
      )}
      <button type="button" className="rf-btn rf-btn--ghost" onClick={onChoose}>
        {t("actions.change")}
      </button>
    </div>
  );
}

/** Por qué no se puede firmar con este certificado, dicho antes del PIN. */
function statusWarning(
  status: Certificate["status"],
  locale: string,
  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  switch (status.kind) {
    case "expired":
      return t("panel.certificate.expired", {
        date: new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(status.notAfter * 1000),
      });
    case "notYetValid":
      return t("panel.certificate.notYetValid");
    case "revoked":
      return t("panel.certificate.revoked", { reason: status.reason });
    default:
      return t("panel.certificate.unreadable");
  }
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
      <label className="rf-prose panel__checkbox-label">
        <input
          type="checkbox"
          checked={checked}
          disabled={disabled}
          aria-describedby={hint ? hintId : undefined}
          onChange={(event) => onChange(event.target.checked)}
        />
        {label}
      </label>
      {hint && (
        <p className="rf-hint" id={hintId}>
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
  usable: boolean,
): string | null {
  const [preview, setPreview] = useState<string | null>(null);

  useEffect(() => {
    if (!usable || !signature.enabled) {
      setPreview(null);
      return;
    }
    let current = true;
    void composer.compose(signature).then((text) => {
      if (current) setPreview(text);
    });
    return () => {
      current = false;
    };
  }, [composer, signature, usable]);

  return preview;
}

/** «2,4 MB». El tamaño en la unidad que el usuario reconoce, no en bytes. */
export function formatSize(bytes: number, locale: string): string {
  const megabytes = bytes / 1_000_000;
  const format = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
  if (megabytes >= 1) return `${format.format(megabytes)} MB`;
  return `${format.format(bytes / 1000)} kB`;
}
