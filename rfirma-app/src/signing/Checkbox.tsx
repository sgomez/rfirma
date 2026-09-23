import { useId } from "react";
import { CheckIcon } from "../design-system/icons";

/** Una casilla. No sale del sistema de diseño: se maqueta con tokens. */
export function Checkbox({
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
