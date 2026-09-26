import { useId } from "react";
import { useTranslation } from "react-i18next";
import { RubricIcon } from "../design-system/icons";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { Certificate } from "./certificate";
import type { Rubric, RubricFailure } from "./rubric";
import { rubricRuleFor, type VisibleSignature } from "./visibleSignature";

interface ModelFieldsetProps {
  signature: VisibleSignature;
  onChangeSignature: (signature: VisibleSignature) => void;
  certificate: Certificate;
  rubric: Rubric | null;
  rubricFailure: RubricFailure | null;
  onChooseRubric: () => void;
  onOpenHelp?: () => void;
}

/** Qué se estampa en el recuadro: el modelo y la rúbrica (docs/design/panel-de-firma.md § El modelo). */
export function ModelFieldset({
  signature,
  onChangeSignature,
  certificate,
  rubric,
  rubricFailure,
  onChooseRubric,
  onOpenHelp,
}: ModelFieldsetProps) {
  const { t, i18n } = useTranslation();
  const modelName = useId();
  const rule = rubricRuleFor(signature.content, signature.withRubric);
  const rubricLocked = rule.locked === "on";
  const noImageTitle = t("panel.visibleSignature.rubric.noImageTitle");

  const selectComplete = () => onChangeSignature({ ...signature, content: { model: "complete" } });
  const selectRubricOnly = () =>
    onChangeSignature({ ...signature, content: { model: "rubricOnly" }, withRubric: true });
  const toggleRubric = () => onChangeSignature({ ...signature, withRubric: !signature.withRubric });

  // Aproximación en CSS, no un render del puente: el firmante y el emisor son
  // los del certificado elegido, tal cual se ven ya en la lista de
  // certificados, sin la máscara del DNI que compone `signing::layer2_text`.
  const signedAtSample = new Intl.DateTimeFormat(i18n.language, { dateStyle: "short" }).format(
    new Date(),
  );

  return (
    <>
      <fieldset className="panel__model">
        <legend className="rf-label">{t("panel.visibleSignature.model.title")}</legend>
        <div className="panel__model-grid">
          <label className="panel__model-card">
            <input
              type="radio"
              className="panel__model-radio"
              name={modelName}
              checked={signature.content.model === "complete"}
              onChange={selectComplete}
            />
            <span className="panel__model-thumbnail" aria-hidden="true">
              {signature.withRubric &&
                (rubric ? (
                  <span className="panel__model-rubric">
                    <RubricIcon />
                  </span>
                ) : (
                  <span className="panel__model-rubric--empty" title={noImageTitle} />
                ))}
              <span className="panel__model-lines">
                <span>{certificate.holderName}</span>
                <span>{signedAtSample}</span>
                <span>{certificate.issuer}</span>
              </span>
            </span>
            <span className="panel__model-name">{t("panel.visibleSignature.model.complete")}</span>
          </label>

          <label
            className={
              rule.rubricOnlySelectable
                ? "panel__model-card"
                : "panel__model-card panel__model-card--disabled"
            }
            title={
              rule.rubricOnlySelectable
                ? undefined
                : t("panel.visibleSignature.model.rubricOnlyDisabled")
            }
          >
            <input
              type="radio"
              className="panel__model-radio"
              name={modelName}
              checked={signature.content.model === "rubricOnly"}
              disabled={!rule.rubricOnlySelectable}
              onChange={selectRubricOnly}
            />
            <span className="panel__model-thumbnail" aria-hidden="true">
              {rule.rubricOnlySelectable && rubric ? (
                <span className="panel__model-rubric panel__model-rubric--full">
                  <RubricIcon />
                </span>
              ) : (
                <span
                  className="panel__model-rubric--empty panel__model-rubric--full"
                  title={rule.rubricOnlySelectable ? noImageTitle : undefined}
                />
              )}
            </span>
            <span className="panel__model-name">
              {t("panel.visibleSignature.model.rubricOnly")}
            </span>
          </label>

          <label
            className="panel__model-card panel__model-card--disabled"
            title={t("panel.visibleSignature.model.customUnavailable")}
          >
            <input
              type="radio"
              className="panel__model-radio"
              name={modelName}
              checked={signature.content.model === "custom"}
              disabled
              onChange={() => {}}
            />
            <span className="panel__model-thumbnail" aria-hidden="true">
              <span className="panel__model-placeholder-lines">
                <span />
                <span />
              </span>
            </span>
            <span className="panel__model-name">{t("panel.visibleSignature.model.custom")}</span>
          </label>
        </div>
      </fieldset>

      <div className="panel__rubric">
        <div className="panel__rubric-row">
          <button
            type="button"
            role="switch"
            aria-checked={rubricLocked || signature.withRubric}
            aria-label={t("panel.visibleSignature.rubric.toggle")}
            disabled={rubricLocked}
            title={rubricLocked ? t("panel.visibleSignature.rubric.lockedTitle") : undefined}
            className="panel__rubric-switch"
            onClick={toggleRubric}
          >
            <span className="panel__rubric-track" aria-hidden="true">
              <span className="panel__rubric-knob" />
            </span>
          </button>
          <span
            className={
              rubricLocked
                ? "panel__rubric-label panel__rubric-label--locked"
                : "panel__rubric-label"
            }
          >
            {t("panel.visibleSignature.rubric.toggle")}
          </span>
          {rubric ? (
            <img
              className="panel__rubric-thumbnail"
              src={rubric.dataUrl}
              width={rubric.width}
              height={rubric.height}
              alt={t("panel.visibleSignature.rubric.thumbnail")}
            />
          ) : (
            <span
              className="panel__rubric-thumbnail panel__rubric-thumbnail--empty"
              title={noImageTitle}
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
