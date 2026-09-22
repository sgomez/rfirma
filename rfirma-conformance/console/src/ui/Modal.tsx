import { type ReactNode, useEffect, useId, useRef } from "react";

interface ModalProps {
  title: ReactNode;
  onClose: () => void;
  wide?: boolean;
  children: ReactNode;
  actions?: ReactNode;
}

/** Un diálogo modal nativo: el foco atrapado, Escape y el fondo los pone el navegador. */
export function Modal({ title, onClose, wide = false, children, actions }: ModalProps) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  useEffect(() => {
    const element = dialog.current;
    if (element && !element.open) element.showModal();
    return () => element?.close();
  }, []);

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: pulsar el fondo equivale a Escape, que el diálogo ya atiende
    <dialog
      ref={dialog}
      className={`modal${wide ? " wide" : ""}`}
      aria-labelledby={titleId}
      onClose={onClose}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onClick={(event) => {
        if (event.target === dialog.current) onClose();
      }}
    >
      <header className="modal-head">
        <h2 id={titleId}>{title}</h2>
        <button type="button" className="icon-button" onClick={onClose} aria-label="Cerrar">
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            aria-hidden="true"
            className="glyph stroke"
          >
            <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
          </svg>
        </button>
      </header>
      <div className="modal-body">{children}</div>
      {actions && <footer className="modal-foot">{actions}</footer>}
    </dialog>
  );
}
