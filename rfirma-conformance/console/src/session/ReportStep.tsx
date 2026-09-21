import { useEffect, useId, useState } from "react";
import type { NewReport } from "../contract/NewReport";
import type { Snapshot } from "../contract/Snapshot";
import { useLive } from "../suite/live";
import { ExternalIcon } from "../ui/icons";
import { Modal } from "../ui/Modal";
import { calendarDate, clientName } from "../words";
import { Step } from "./Step";

export function ReportStep({ snapshot, busy }: { snapshot: Snapshot; busy: boolean }) {
  const { suite, complain } = useLive();
  const [editing, setEditing] = useState(false);
  const [creating, setCreating] = useState(false);
  const name = snapshot.report_name;
  const view = snapshot.report;
  const client = snapshot.client;
  const open = editing || !name;

  useEffect(() => {
    if (name) setEditing(false);
  }, [name]);

  const openReport = (report: string) =>
    suite.openReport(report).catch(complain("No se pudo abrir el informe"));

  return (
    <Step
      number={2}
      label="Informe"
      done={name !== null}
      disabled={client === null}
      summary={
        name && view ? (
          <span className="facts">
            <strong className="mono">{name}</strong>
            <span>
              {clientName(view.kind)} {view.header.client_version}
            </span>
            <span className="muted">
              {view.header.os} {view.header.os_version}
            </span>
            <span className="muted">{calendarDate(view.header.date)}</span>
          </span>
        ) : (
          <span className="muted">Crea uno nuevo o abre uno existente.</span>
        )
      }
      actions={
        name && (
          <>
            {!editing && (
              <button
                type="button"
                className="button ghost small"
                onClick={() => setEditing(true)}
                disabled={busy}
              >
                Cambiar
              </button>
            )}
            <a
              className="button ghost small"
              href={suite.page(`/informe/${encodeURIComponent(name)}`)}
              target="_blank"
              rel="noopener"
            >
              Abrir en otra ventana <ExternalIcon />
            </a>
          </>
        )
      }
    >
      {open && client && (
        <div className="step-form column">
          {snapshot.reports.length > 0 ? (
            <ul className="report-list" aria-label="Informes">
              {snapshot.reports.map((entry) => {
                const continuable = !entry.complaint && entry.kind === client.kind;
                return (
                  <li key={entry.name} className={entry.name === name ? "is-current" : undefined}>
                    <span className="mono report-name">{entry.name}</span>
                    <span className="report-client">
                      {entry.complaint ? (
                        <span className="complaint">ilegible</span>
                      ) : (
                        `${clientName(entry.kind)} ${entry.client_version ?? ""}`
                      )}
                    </span>
                    <span className="muted report-date">{calendarDate(entry.date)}</span>
                    {continuable ? (
                      <button
                        type="button"
                        className="button small"
                        disabled={busy || entry.name === name}
                        onClick={() => void openReport(entry.name)}
                      >
                        {entry.name === name ? "Abierto" : "Abrir"}
                      </button>
                    ) : (
                      <a
                        className="button ghost small"
                        href={suite.page(`/informe/${encodeURIComponent(entry.name)}`)}
                        target="_blank"
                        rel="noopener"
                        title="No se puede continuar con el cliente activo; se ve en solo lectura"
                      >
                        Ver en otra ventana <ExternalIcon />
                      </a>
                    )}
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="muted">Aún no hay informes.</p>
          )}
          <div className="form-actions">
            {name && (
              <button type="button" className="button ghost" onClick={() => setEditing(false)}>
                Cancelar
              </button>
            )}
            <button
              type="button"
              className="button primary"
              onClick={() => setCreating(true)}
              disabled={busy}
            >
              Informe nuevo…
            </button>
          </div>
        </div>
      )}
      {creating && <NewReportModal onClose={() => setCreating(false)} />}
    </Step>
  );
}

const FIELDS: [keyof NewReport, string, string?][] = [
  ["name", "Nombre", "letras, cifras, «.», «-» o «_»"],
  ["client_version", "Versión del cliente", "nadie puede deducirla"],
  ["os", "Sistema"],
  ["os_version", "Versión del sistema"],
  ["store", "Almacén"],
  ["transport", "Transporte"],
];

function NewReportModal({ onClose }: { onClose: () => void }) {
  const { suite } = useLive();
  const formId = useId();
  const [draft, setDraft] = useState<NewReport>({
    name: "",
    client_version: "",
    os: "",
    os_version: "",
    transport: "",
    store: "",
  });
  const [complaint, setComplaint] = useState<string | null>(null);

  useEffect(() => {
    suite
      .defaults()
      .then((deduced) => setDraft((was) => ({ ...was, ...deduced })))
      .catch((error: unknown) =>
        setComplaint(error instanceof Error ? error.message : String(error)),
      );
  }, [suite]);

  const create = async () => {
    try {
      await suite.createReport(draft);
      onClose();
    } catch (error) {
      setComplaint(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <Modal
      title="Informe nuevo"
      onClose={onClose}
      actions={
        <>
          <button type="button" className="button ghost" onClick={onClose}>
            Cancelar
          </button>
          <button type="submit" form={formId} className="button primary">
            Crear
          </button>
        </>
      }
    >
      <form
        id={formId}
        className="form-grid"
        onSubmit={(event) => {
          event.preventDefault();
          void create();
        }}
      >
        {FIELDS.map(([key, label, hint]) => (
          <label key={key} className="form-row">
            <span className="field-label">{label}</span>
            <input
              className={`input${key === "store" ? " mono" : ""}`}
              required
              value={draft[key]}
              placeholder={hint}
              pattern={key === "name" ? "[A-Za-z0-9_\\-][A-Za-z0-9._\\-]*" : undefined}
              onChange={(event) => setDraft({ ...draft, [key]: event.target.value })}
              spellCheck={false}
            />
          </label>
        ))}
        {complaint && <p className="complaint">{complaint}</p>}
      </form>
    </Modal>
  );
}
