import type { ReactNode } from "react";
import { CheckMark } from "../ui/icons";

interface StepProps {
  number: 1 | 2 | 3;
  label: string;
  done: boolean;
  disabled?: boolean;
  marked?: string | null;
  summary: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
}

/** Uno de los tres pasos de la sesión, desactivado mientras el anterior no esté resuelto. */
export function Step({
  number,
  label,
  done,
  disabled = false,
  marked = null,
  summary,
  actions,
  children,
}: StepProps) {
  return (
    <section
      className={`step${done ? " is-done" : ""}${disabled ? " is-disabled" : ""}${marked ? " is-marked" : ""}`}
      aria-label={`${number}. ${label}`}
      aria-disabled={disabled || undefined}
      inert={disabled || undefined}
    >
      <div className="step-line">
        <span className="step-number" aria-hidden="true">
          {done ? <CheckMark /> : number}
        </span>
        <h2 className="step-label">{label}</h2>
        <div className="step-summary">{summary}</div>
        {marked && <span className="mark">{marked}</span>}
        {actions && <div className="step-actions">{actions}</div>}
      </div>
      {children}
    </section>
  );
}
