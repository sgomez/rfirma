import { useEffect, useId, useState } from "react";
import type { ClientKind } from "../contract/ClientKind";
import type { Snapshot } from "../contract/Snapshot";
import { useLive } from "../suite/live";
import { clientName, identityText, originName } from "../words";
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
  const [draft, setDraft] = useState<Draft>({ kind: "rfirma", binary: "", trustRoot: "" });
  const kindId = useId();
  const binaryId = useId();
  const candidatesId = useId();
  const rootId = useId();

  const trustRoot = client?.trust_root ?? "";
  const resolved = client ? [client.kind, client.binary, trustRoot].join("\n") : "";
  useEffect(() => {
    if (!resolved) return;
    const [kind, binary = "", trustRoot = ""] = resolved.split("\n");
    setDraft({ kind: kind as ClientKind, binary, trustRoot });
    setEditing(false);
  }, [resolved]);

  const choose = (kind: ClientKind) =>
    setDraft(
      client?.kind === kind
        ? { kind, binary: client.binary, trustRoot }
        : { kind, binary: "", trustRoot: "" },
    );

  const open = editing || !client;
  const unsettled =
    client !== null &&
    (draft.kind !== client.kind ||
      (draft.binary !== "" && draft.binary !== client.binary) ||
      (draft.trustRoot !== "" && draft.trustRoot !== trustRoot));
  const resolve = (binary = draft.binary) =>
    suite
      .chooseClient({
        kind: draft.kind,
        binary: binary.trim() || null,
        trust_root: draft.trustRoot.trim() || null,
      })
      .catch(complain("No se pudo preparar el cliente"));
  const offered = snapshot.candidates.filter((candidate) => candidate.kind === draft.kind);
  const pick = (binary: string) => {
    setDraft({ ...draft, binary });
    void resolve(binary);
  };

  return (
    <Step
      number={1}
      label="Cliente"
      done={client !== null && !unsettled}
      marked={unsettled ? "cambios sin aplicar" : null}
      summary={
        snapshot.resolving_client ? (
          <span className="working">Preparando el cliente…</span>
        ) : client ? (
          <span className="facts">
            <strong>{clientName(client.kind)}</strong>
            {identityText(client.identity) && (
              <span title="Versión">{identityText(client.identity)}</span>
            )}
            <span className="fact-label">{originName[client.origin]}</span>
            <code title="Binario">{client.binary}</code>
            <span className="fact-label">raíz</span>
            <code title="Raíz de confianza">{trustRoot || "la de cada almacén"}</code>
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
                    onChange={() => choose(kind)}
                  />
                  <span>{clientName(kind)}</span>
                </label>
              ))}
            </div>
          </div>
          {offered.length > 0 && (
            <div className="field-group grow">
              <span className="field-label" id={candidatesId}>
                Candidatos
              </span>
              <div className="candidates" role="radiogroup" aria-labelledby={candidatesId}>
                {offered.map((candidate) => (
                  <label key={candidate.binary} className="candidate">
                    <input
                      type="radio"
                      name="client-candidate"
                      checked={client?.binary === candidate.binary && !unsettled}
                      disabled={busy || snapshot.resolving_client}
                      onChange={() => pick(candidate.binary)}
                    />
                    <span className="facts">
                      <span>{originName[candidate.origin]}</span>
                      {identityText(candidate.identity) && (
                        <span>{identityText(candidate.identity)}</span>
                      )}
                      <code>{candidate.binary}</code>
                    </span>
                  </label>
                ))}
              </div>
            </div>
          )}
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
