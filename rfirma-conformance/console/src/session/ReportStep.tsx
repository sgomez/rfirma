import { useEffect, useId, useState } from "react";
import type { Header } from "../contract/Header";
import type { NewReport } from "../contract/NewReport";
import type { ReportEntry } from "../contract/ReportEntry";
import type { SelectionResolution } from "../contract/SelectionResolution";
import type { Snapshot } from "../contract/Snapshot";
import { useLive } from "../suite/live";
import { ExternalIcon } from "../ui/icons";
import { Modal } from "../ui/Modal";
import { calendarDate, clientName } from "../words";
import { ScopeField, useScopePreview } from "./ScopeField";
import { Step } from "./Step";

export function ReportStep({ snapshot, busy }: { snapshot: Snapshot; busy: boolean }) {
  const { suite, complain } = useLive();
  const [editing, setEditing] = useState(false);
  const [creating, setCreating] = useState(false);
  const [correcting, setCorrecting] = useState(false);
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
      summary={
        name && view ? (
          <span className="facts">
            <strong className="mono">{name}</strong>
            {view.header.title && <span>{view.header.title}</span>}
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
            <button
              type="button"
              className="button ghost small"
              onClick={() => setCorrecting(true)}
              disabled={busy}
            >
              Editar cabecera
            </button>
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
      {open && (
        <div className="step-form column">
          {snapshot.reports.length > 0 ? (
            <>
              <ReportList
                label="Se pueden continuar"
                entries={snapshot.reports.filter((entry) => entry.continuable)}
                current={name}
                busy={busy}
                open={openReport}
              />
              <ReportList
                label="Solo lectura"
                entries={snapshot.reports.filter((entry) => !entry.continuable)}
                current={name}
                busy={busy}
                open={openReport}
              />
            </>
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
              disabled={busy || client === null}
            >
              Informe nuevo…
            </button>
          </div>
        </div>
      )}
      {correcting && view && (
        <CorrectHeaderModal header={view.header} onClose={() => setCorrecting(false)} />
      )}
      {creating && <NewReportModal onClose={() => setCreating(false)} />}
    </Step>
  );
}

type Draft = NewReport;

const FIELDS: [keyof Draft, string, string?][] = [
  ["name", "Nombre", "letras, cifras, «.», «-» o «_»"],
  ["title", "Título", "para reconocerlo en la lista"],
  ["client_version", "Versión del cliente", "rFirma la dice; la de AutoFirma, a mano"],
  ["os", "Sistema"],
  ["os_version", "Versión del sistema"],
];

const OPTIONAL: (keyof Draft)[] = ["title"];

function HeaderFields({
  draft,
  resolution,
  onChange,
  withName,
}: {
  draft: Draft;
  resolution: SelectionResolution | null;
  onChange: (draft: Draft) => void;
  withName: boolean;
}) {
  return (
    <>
      {FIELDS.filter(([key]) => withName || key !== "name").map(([key, label, hint]) => (
        <label key={key} className="form-row">
          <span className="field-label">{label}</span>
          <input
            className="input"
            required={!OPTIONAL.includes(key)}
            value={draft[key] ?? ""}
            placeholder={hint}
            pattern={key === "name" ? "[A-Za-z0-9_\\-][A-Za-z0-9._\\-]*" : undefined}
            onChange={(event) => onChange({ ...draft, [key]: event.target.value })}
            spellCheck={false}
          />
        </label>
      ))}
      <label className="form-row">
        <span className="field-label">Notas</span>
        <textarea
          className="input"
          rows={3}
          value={draft.notes}
          onChange={(event) => onChange({ ...draft, notes: event.target.value })}
        />
      </label>
      <ScopeField
        value={draft.scope}
        resolution={resolution}
        onChange={(scope) => onChange({ ...draft, scope })}
      />
    </>
  );
}

function HeaderModal({
  title,
  submit,
  initial,
  withName,
  notice = null,
  onSubmit,
  onClose,
}: {
  title: string;
  submit: string;
  initial: Draft;
  withName: boolean;
  notice?: string | null;
  onSubmit: (draft: Draft) => Promise<unknown>;
  onClose: () => void;
}) {
  const formId = useId();
  const [draft, setDraft] = useState<Draft>(initial);
  const [complaint, setComplaint] = useState<string | null>(null);
  const resolution = useScopePreview(draft.scope ?? "");
  const unmatched = (resolution?.unknown.length ?? 0) > 0;

  useEffect(() => {
    setDraft(initial);
  }, [initial]);

  const send = async () => {
    try {
      await onSubmit(draft);
      onClose();
    } catch (error) {
      setComplaint(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <Modal
      title={title}
      onClose={onClose}
      actions={
        <>
          <button type="button" className="button ghost" onClick={onClose}>
            Cancelar
          </button>
          <button type="submit" form={formId} className="button primary" disabled={unmatched}>
            {submit}
          </button>
        </>
      }
    >
      <form
        id={formId}
        className="form-grid"
        onSubmit={(event) => {
          event.preventDefault();
          void send();
        }}
      >
        <HeaderFields
          draft={draft}
          resolution={resolution}
          onChange={setDraft}
          withName={withName}
        />
        {(complaint ?? notice) && <p className="complaint">{complaint ?? notice}</p>}
      </form>
    </Modal>
  );
}

const BLANK: Draft = {
  name: "",
  title: "",
  notes: "",
  client_version: "",
  os: "",
  os_version: "",
  scope: null,
};

function NewReportModal({ onClose }: { onClose: () => void }) {
  const { suite } = useLive();
  const [initial, setInitial] = useState<Draft>(BLANK);
  const [complaint, setComplaint] = useState<string | null>(null);

  useEffect(() => {
    suite
      .defaults()
      .then((deduced) => setInitial({ ...BLANK, ...deduced }))
      .catch((error: unknown) =>
        setComplaint(error instanceof Error ? error.message : String(error)),
      );
  }, [suite]);

  return (
    <HeaderModal
      title="Informe nuevo"
      submit="Crear"
      initial={initial}
      withName
      notice={complaint}
      onSubmit={(draft) => suite.createReport(draft)}
      onClose={onClose}
    />
  );
}

function CorrectHeaderModal({ header, onClose }: { header: Header; onClose: () => void }) {
  const { suite } = useLive();
  const [initial] = useState<Draft>(() => ({ ...BLANK, ...header }));
  return (
    <HeaderModal
      title="Cabecera del informe"
      submit="Guardar"
      initial={initial}
      withName={false}
      onSubmit={(draft) =>
        suite.correctHeader({
          title: draft.title,
          notes: draft.notes,
          client_version: draft.client_version,
          os: draft.os,
          os_version: draft.os_version,
          scope: draft.scope,
        })
      }
      onClose={onClose}
    />
  );
}

function ReportList({
  label,
  entries,
  current,
  busy,
  open,
}: {
  label: string;
  entries: ReportEntry[];
  current: string | null;
  busy: boolean;
  open: (name: string) => Promise<unknown>;
}) {
  const { suite } = useLive();
  if (entries.length === 0) return null;
  return (
    <ul className="report-list" aria-label={label}>
      {entries.map((entry) => (
        <li key={entry.name} className={entry.name === current ? "is-current" : undefined}>
          <span className="report-name">
            <span className="mono">{entry.name}</span>
            {entry.title && <span className="muted">{entry.title}</span>}
          </span>
          <span className="report-client">
            {entry.complaint ? (
              <span className="complaint">ilegible</span>
            ) : (
              `${clientName(entry.kind)} ${entry.client_version ?? ""}`
            )}
            {entry.why_not && <span className="muted">{entry.why_not}</span>}
          </span>
          <span className="muted report-date">{calendarDate(entry.date)}</span>
          {entry.continuable ? (
            <button
              type="button"
              className="button small"
              disabled={busy || entry.name === current}
              onClick={() => void open(entry.name)}
            >
              {entry.name === current ? "Abierto" : "Abrir"}
            </button>
          ) : (
            <a
              className="button ghost small"
              href={suite.page(`/informe/${encodeURIComponent(entry.name)}`)}
              target="_blank"
              rel="noopener"
            >
              Ver en otra ventana <ExternalIcon />
            </a>
          )}
        </li>
      ))}
    </ul>
  );
}
