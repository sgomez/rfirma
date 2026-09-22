import { type ReactNode, useState } from "react";
import { NavLink } from "react-router";
import { useLive } from "../suite/live";
import { Modal } from "../ui/Modal";
import { useShortcuts } from "../ui/shortcuts";
import { ThemeToggle } from "../ui/theme";

const CONNECTION_LABEL = {
  connecting: "Conectando…",
  live: "Conectada",
  lost: "Sin conexión; reintentando…",
} as const;

export function Shell({ children }: { children: ReactNode }) {
  const { suite, connection } = useLive();
  const [help, setHelp] = useState(false);
  useShortcuts({ "?": () => setHelp(true) });

  return (
    <div className="shell">
      <a className="skip-link" href="#main">
        Saltar al contenido
      </a>
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true" />
          <span className="brand-name">Suite de conformidad</span>
          <span className="brand-sub">afirma://</span>
        </div>
        <nav className="tabs" aria-label="Vistas">
          <NavLink to={suite.page("/")} end className="tab">
            Sesión
          </NavLink>
          <NavLink to={suite.page("/comparar")} className="tab">
            Comparar
          </NavLink>
        </nav>
        <span className="grow" />
        <output className={`connection is-${connection}`} aria-live="polite">
          <span className="connection-dot" aria-hidden="true" />
          {CONNECTION_LABEL[connection]}
        </output>
        <button
          type="button"
          className="icon-button"
          onClick={() => setHelp(true)}
          aria-label="Atajos de teclado"
          aria-keyshortcuts="?"
          title="Atajos de teclado (?)"
        >
          <kbd>?</kbd>
        </button>
        <ThemeToggle />
      </header>
      <main id="main" className="page">
        {children}
      </main>
      <Toasts />
      {help && <ShortcutsModal onClose={() => setHelp(false)} />}
    </div>
  );
}

function Toasts() {
  const { toasts, dismiss } = useLive();
  return (
    <div className="toasts" role="alert" aria-live="assertive">
      {toasts.map((toast) => (
        <div key={toast.id} className="toast">
          <span>{toast.text}</span>
          <button
            type="button"
            className="icon-button"
            onClick={() => dismiss(toast.id)}
            aria-label="Descartar el aviso"
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}

const SHORTCUTS: [string, [string[], string][]][] = [
  [
    "Ejecutar",
    [
      [["p"], "Pendientes del informe"],
      [["r"], "Ejecutar la comprobación enfocada"],
      [["x"], "Saltar esta"],
      [["⇧", "X"], "Detener"],
    ],
  ],
  [
    "Responder",
    [
      [["s"], "Sí · Empezar"],
      [["n"], "No"],
      [["Esc"], "Descartar: queda pendiente"],
    ],
  ],
  [
    "Moverse",
    [
      [["j"], "Comprobación siguiente"],
      [["k"], "Comprobación anterior"],
      [["Intro"], "Desplegar o plegar la comprobación"],
      [["["], "Plegar todo"],
      [["]"], "Desplegar todo"],
    ],
  ],
  [
    "Registro",
    [
      [["l"], "Pausar o reanudar"],
      [["f"], "Ir a la comprobación en curso"],
      [["?"], "Esta ayuda"],
    ],
  ],
];

function ShortcutsModal({ onClose }: { onClose: () => void }) {
  return (
    <Modal title="Atajos de teclado" onClose={onClose}>
      <div className="shortcuts">
        {SHORTCUTS.map(([group, entries]) => (
          <section key={group}>
            <h3>{group}</h3>
            <dl>
              {entries.map(([keys, what]) => (
                <div key={what} className="shortcut">
                  <dt>
                    {keys.map((key) => (
                      <kbd key={key}>{key}</kbd>
                    ))}
                  </dt>
                  <dd>{what}</dd>
                </div>
              ))}
            </dl>
          </section>
        ))}
      </div>
    </Modal>
  );
}
