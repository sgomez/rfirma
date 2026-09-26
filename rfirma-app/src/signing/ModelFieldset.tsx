import { useId } from "react";
import { useTranslation } from "react-i18next";
import { ErrorNotice } from "../errors/ErrorNotice";
import { Switch } from "../preferences/Switch";
import type { Rubric, RubricFailure } from "./rubric";
import { rubricRuleFor, type VisibleSignature } from "./visibleSignature";

interface ModelFieldsetProps {
  signature: VisibleSignature;
  onChangeSignature: (signature: VisibleSignature) => void;
  rubric: Rubric | null;
  rubricFailure: RubricFailure | null;
  onChooseRubric: () => void;
  onOpenHelp?: () => void;
}

/** Qué se estampa en el recuadro: el modelo y la rúbrica (docs/design/panel-de-firma.md § El modelo). */
export function ModelFieldset({
  signature,
  onChangeSignature,
  rubric,
  rubricFailure,
  onChooseRubric,
  onOpenHelp,
}: ModelFieldsetProps) {
  const { t } = useTranslation();
  const modelName = useId();
  const rule = rubricRuleFor(signature.content, signature.withRubric);
  // Encendida sin imagen: el hueco punteado también aparece en las tarjetas,
  // no solo en la fila (docs/design/panel-de-firma.md § La rúbrica).
  const noImageTitle = signature.withRubric
    ? t("panel.visibleSignature.rubric.noImageTitle")
    : undefined;

  const selectComplete = () => onChangeSignature({ ...signature, content: { model: "complete" } });
  const selectRubricOnly = () =>
    onChangeSignature({ ...signature, content: { model: "rubricOnly" }, withRubric: true });

  return (
    <>
      <fieldset className="panel__model">
        <legend className="rf-label">{t("panel.visibleSignature.model.title")}</legend>
        <div className="panel__model-grid">
          <label className="panel__model-card">
            <input
              type="radio"
              name={modelName}
              checked={signature.content.model === "complete"}
              onChange={selectComplete}
            />
            <span className="panel__model-thumbnail" aria-hidden="true">
              {signature.withRubric && (
                <span
                  className={rubric ? "panel__model-rubric" : "panel__model-rubric--empty"}
                  title={rubric ? undefined : noImageTitle}
                />
              )}
              <span className="panel__model-lines">
                <span />
                <span />
                <span />
              </span>
            </span>
            <span className="rf-hint">{t("panel.visibleSignature.model.complete")}</span>
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
              name={modelName}
              checked={signature.content.model === "rubricOnly"}
              disabled={!rule.rubricOnlySelectable}
              onChange={selectRubricOnly}
            />
            <span className="panel__model-thumbnail" aria-hidden="true">
              <span
                className={
                  rule.rubricOnlySelectable && rubric
                    ? "panel__model-rubric panel__model-rubric--full"
                    : "panel__model-rubric--empty panel__model-rubric--full"
                }
                title={rule.rubricOnlySelectable && !rubric ? noImageTitle : undefined}
              />
            </span>
            <span className="rf-hint">{t("panel.visibleSignature.model.rubricOnly")}</span>
          </label>

          <label
            className="panel__model-card panel__model-card--disabled"
            title={t("panel.visibleSignature.model.customUnavailable")}
          >
            <input
              type="radio"
              name={modelName}
              checked={signature.content.model === "custom"}
              disabled
              onChange={() => {}}
            />
            <span className="panel__model-thumbnail" aria-hidden="true">
              <span className="panel__model-lines">
                <span />
                <span />
              </span>
            </span>
            <span className="rf-hint">{t("panel.visibleSignature.model.custom")}</span>
          </label>
        </div>
      </fieldset>

      <div className="panel__rubric">
        <Switch
          checked={rule.locked === "on" || signature.withRubric}
          disabled={rule.locked === "on"}
          title={rule.locked === "on" ? t("panel.visibleSignature.rubric.lockedTitle") : undefined}
          label={t("panel.visibleSignature.rubric.toggle")}
          onChange={(withRubric) => onChangeSignature({ ...signature, withRubric })}
        />
        <div className="panel__rubric-row">
          {rubric ? (
            <img
              className="panel__rubric-thumbnail"
              src={rubric.dataUrl}
              width={rubric.width}
              height={rubric.height}
              alt={t("panel.visibleSignature.rubric.thumbnail")}
            />
          ) : (
            signature.withRubric && (
              <span
                className="panel__rubric-thumbnail panel__rubric-thumbnail--empty"
                title={t("panel.visibleSignature.rubric.noImageTitle")}
              />
            )
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
