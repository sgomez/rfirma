import { useEffect, useId, useState } from "react";
import type { ReportView } from "../contract/ReportView";
import type { Validation } from "../contract/Validation";
import { useLive } from "../suite/live";
import { ResultIcon } from "../ui/icons";
import { Modal } from "../ui/Modal";

export function ValidateModal({
  report,
  view,
  onClose,
}: {
  report: string;
  view: ReportView;
  onClose: () => void;
}) {
  const { suite } = useLive();
  const [references, setReferences] = useState<string[] | null>(null);
  const [reference, setReference] = useState("");
  const [validation, setValidation] = useState<Validation | null>(null);
  const [complaint, setComplaint] = useState<string | null>(null);
  const [validating, setValidating] = useState(false);
  const selectId = useId();

  useEffect(() => {
    suite
      .references()
      .then((names) => {
        setReferences(names);
        setReference(names[0] ?? "");
      })
      .catch((error: unknown) =>
        setComplaint(String(error instanceof Error ? error.message : error)),
      );
  }, [suite]);

  const validate = async () => {
    setValidating(true);
    setComplaint(null);
    try {
      setValidation(await suite.validate(report, reference));
    } catch (error) {
      setComplaint(error instanceof Error ? error.message : String(error));
    } finally {
      setValidating(false);
    }
  };

  return (
    <Modal wide title="Comparar con la referencia" onClose={onClose}>
      <form
        className="inline-form"
        onSubmit={(event) => {
          event.preventDefault();
          void validate();
        }}
      >
        <label htmlFor={selectId}>Referencia</label>
        <select
          id={selectId}
          value={reference}
          onChange={(event) => {
            setReference(event.target.value);
            setValidation(null);
          }}
          disabled={!references?.length}
        >
          {references?.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        <button type="submit" className="button primary" disabled={!reference || validating}>
          {validating ? "Validando…" : "Validar"}
        </button>
      </form>
      <p className="hint">
        Cruza <strong>{report}</strong> con lo que la referencia sabe de AutoFirma 1.9.2. Una
        discrepancia es un fallo de la suite o de la referencia, nunca del cliente.
      </p>
      {complaint && <p className="complaint">{complaint}</p>}
      {validation && <ValidationResult validation={validation} view={view} />}
    </Modal>
  );
}

function ValidationResult({ validation, view }: { validation: Validation; view: ReportView }) {
  if (validation.result === "validated") {
    return (
      <div className="verdict tone-ok" role="status">
        <ResultIcon result="CONFORME" size={20} />
        <div>
          <strong>Validado</strong>
          <p>Ninguna comprobación medible queda PENDIENTE y cada resultado coincide.</p>
        </div>
      </div>
    );
  }
  const { discrepancies } = validation;
  const groups = view.sets
    .map((set) => ({
      set: set.name,
      rows: discrepancies.filter((each) => set.checks.some((check) => check.id === each.id)),
    }))
    .filter((group) => group.rows.length > 0);
  return (
    <div className="discrepancies">
      <div className="verdict tone-fail" role="status">
        <ResultIcon result="NO CONFORME" size={20} />
        <div>
          <strong>
            {discrepancies.length} {discrepancies.length === 1 ? "discrepancia" : "discrepancias"}
          </strong>
          <p>Fallos de la suite o de la referencia, que se investigan.</p>
        </div>
      </div>
      {groups.map((group) => (
        <section key={group.set} className="diff-group">
          <h3>{group.set}</h3>
          <table className="table">
            <thead>
              <tr>
                <th scope="col">Comprobación</th>
                <th scope="col">Previsto en la referencia</th>
                <th scope="col">Resultado</th>
              </tr>
            </thead>
            <tbody>
              {group.rows.map((row) => (
                <tr key={row.id}>
                  <td>
                    <code>{row.id}</code>
                    {row.cause && <div className="muted">{row.cause}</div>}
                    {row.note && <div className="muted">{row.note}</div>}
                  </td>
                  <td>
                    <span className="result-label">
                      <ResultIcon result={row.expected} size={12} decorative />
                      {row.expected}
                    </span>
                  </td>
                  <td>
                    <span className="result-label">
                      <ResultIcon result={row.observed} size={12} decorative />
                      {row.observed}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ))}
    </div>
  );
}
