import { useEffect, useId, useState } from "react";
import type { ClientKind } from "../contract/ClientKind";
import type { Snapshot } from "../contract/Snapshot";
import { useLive } from "../suite/live";
import { clientName } from "../words";
import { Step } from "./Step";

interface Draft {
  kind: ClientKind;
  binary: string;
  trustRoot: string;
}

export function ClientStep({ snapshot, busy }: { snapshot: Snapshot; busy: boolean }) {
  const { suite, complain } = useLive();
  const client = snapshot.client;
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<Draft>({ kind: "autofirma", binary: "", trustRoot: "" });
  const kindId = useId();
  const binaryId = useId();
  const rootId = useId();

  const trustRoot = client?.profiles[0]?.trust_root ?? "";
  const resolved = client ? [client.kind, client.binary, trustRoot].join("\n") : "";
  useEffect(() => {
    if (!resolved) return;
    const [kind, binary = "", trustRoot = ""] = resolved.split("\n");
    setDraft({ kind: kind as ClientKind, binary, trustRoot });
    setEditing(false);
  }, [resolved]);

  const open = editing || !client;
  const unsettled =
    client !== null &&
    (draft.kind !== client.kind ||
      (draft.binary !== "" && draft.binary !== client.binary) ||
      (draft.trustRoot !== "" && draft.trustRoot !== trustRoot));
  const resolve = () =>
    suite
      .chooseClient({
        kind: draft.kind,
        binary: draft.binary.trim() || null,
        trust_root: draft.trustRoot.trim() || null,
      })
      .catch(complain("No se pudo resolver el cliente"));

  return (
    <Step
      number={1}
      label="Cliente"
      done={client !== null && !unsettled}
      marked={unsettled ? "cambios sin resolver" : null}
      summary={
        snapshot.resolving_client ? (
          <span className="working">Resolviendo… monta el almacén aislado</span>
        ) : client ? (
          <span className="facts">
            <strong>{clientName(client.kind)}</strong>
            <code title="Binario">{client.binary}</code>
            <span className="fact-label">raíz</span>
            <code title="Raíz de confianza">{trustRoot}</code>
            <span className="fact-label">almacenes</span>
            <code title="Almacenes">
              {client.profiles.map((profile) => profile.store).join(", ")}
            </code>
          </span>
        ) : snapshot.client_complaints.length > 0 ? (
          <span className="complaint">{snapshot.client_complaints.join(" · ")}</span>
        ) : (
          <span className="muted">Elige cuál es, su binario y su raíz de confianza.</span>
        )
      }
      actions={
        client &&
        !editing && (
          <button
            type="button"
            className="button ghost small"
            onClick={() => setEditing(true)}
            disabled={busy}
          >
            Cambiar
          </button>
        )
      }
    >
      {open && (
        <form
          className="step-form"
          onSubmit={(event) => {
            event.preventDefault();
            void resolve();
          }}
        >
          <div className="field-group">
            <span className="field-label" id={kindId}>
              Cuál es
            </span>
            <div className="segmented" role="radiogroup" aria-labelledby={kindId}>
              {(["autofirma", "rfirma"] as const).map((kind) => (
                <label key={kind} className="segment-option">
                  <input
                    type="radio"
                    name="client-kind"
                    value={kind}
                    checked={draft.kind === kind}
                    onChange={() => setDraft({ ...draft, kind })}
                  />
                  <span>{clientName(kind)}</span>
                </label>
              ))}
            </div>
          </div>
          <div className="field-group grow">
            <label className="field-label" htmlFor={binaryId}>
              Binario
            </label>
            <input
              id={binaryId}
              className="input mono"
              value={draft.binary}
              placeholder="vacío: se busca en el PATH"
              onChange={(event) => setDraft({ ...draft, binary: event.target.value })}
              spellCheck={false}
            />
          </div>
          <div className="field-group grow">
            <label className="field-label" htmlFor={rootId}>
              Raíz de confianza
            </label>
            <input
              id={rootId}
              className="input mono"
              value={draft.trustRoot}
              placeholder="vacío: la que resuelva el cliente"
              onChange={(event) => setDraft({ ...draft, trustRoot: event.target.value })}
              spellCheck={false}
            />
          </div>
          <div className="form-actions">
            {client && (
              <button type="button" className="button ghost" onClick={() => setEditing(false)}>
                Cancelar
              </button>
            )}
            <button
              type="submit"
              className="button primary"
              disabled={busy || snapshot.resolving_client}
            >
              Resolver
            </button>
          </div>
        </form>
      )}
    </Step>
  );
}
