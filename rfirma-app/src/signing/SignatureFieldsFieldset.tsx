import { useId } from "react";
import { useTranslation } from "react-i18next";
import { ErrorNotice } from "../errors/ErrorNotice";
import { Checkbox } from "./Checkbox";
import type { Rubric, RubricFailure } from "./rubric";
import type { VisibleSignature } from "./visibleSignature";

interface SignatureFieldsFieldsetProps {
  signature: VisibleSignature;
  onChangeSignature: (signature: VisibleSignature) => void;
  rubric: Rubric | null;
  rubricFailure: RubricFailure | null;
  onChooseRubric: () => void;
  onOpenHelp?: () => void;
}

/** Qué se estampa en el recuadro: las casillas de contenido, el motivo y la rúbrica. */
export function SignatureFieldsFieldset({
  signature,
  onChangeSignature,
  rubric,
  rubricFailure,
  onChooseRubric,
  onOpenHelp,
}: SignatureFieldsFieldsetProps) {
  const { t } = useTranslation();
  const reasonId = useId();

  const changeField = (field: keyof VisibleSignature["fields"], checked: boolean) => {
    onChangeSignature({ ...signature, fields: { ...signature.fields, [field]: checked } });
  };

  return (
    <>
      <fieldset className="panel__fields">
        <legend className="rf-label">{t("panel.visibleSignature.content")}</legend>
        <Checkbox
          checked={signature.fields.signerName}
          label={t("panel.visibleSignature.fields.signerName")}
          onChange={(checked) => changeField("signerName", checked)}
        />
        <Checkbox
          checked={signature.fields.issuer}
          label={t("panel.visibleSignature.fields.issuer")}
          onChange={(checked) => changeField("issuer", checked)}
        />
        <Checkbox
          checked={signature.fields.signedAt}
          label={t("panel.visibleSignature.fields.signedAt")}
          onChange={(checked) => changeField("signedAt", checked)}
        />
        <Checkbox
          checked={signature.rubric && rubric !== null}
          disabled={rubric === null}
          label={t("panel.visibleSignature.fields.rubric")}
          hint={rubric === null ? t("panel.visibleSignature.fields.rubricDisabled") : null}
          onChange={(checked) => onChangeSignature({ ...signature, rubric: checked })}
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
            onChange={(event) => onChangeSignature({ ...signature, reason: event.target.value })}
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
        {rubric && <p className="rf-hint">{t("panel.visibleSignature.rubric.flattened")}</p>}
        {rubricFailure && (
          <ErrorNotice
            situation={rubricFailure.situation}
            technicalDetail={rubricFailure.detail}
            onOpenHelp={onOpenHelp}
          />
        )}
      </div>
    </>
  );
}
