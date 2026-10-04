//! La frase de *Personalizada*: el campo con las pastillas de los datos y su menú «+ Dato».

import {
  type ClipboardEvent,
  type DragEvent,
  type KeyboardEvent,
  useCallback,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { PlusIcon } from "../design-system/icons";
import { Menu, MenuItem } from "../design-system/Menu";
import {
  datumOf,
  isCanonical,
  pillBesideCaret,
  placeCaretAfter,
  rangeAtEnd,
  rangeAtPoint,
  readPhrase,
  renderPhrase,
  samePhrase,
  selectionInside,
} from "./phraseDom";
import type { Datum, PhrasePart } from "./visibleSignature";
import "./PhraseEditor.css";

const DATA: readonly Datum[] = ["signer", "issuer", "signedAt"];

interface PhraseEditorProps {
  phrase: PhrasePart[];
  /** El valor con el que se ve cada dato, sacado del certificado elegido. */
  samples: Record<Datum, string>;
  onChange: (phrase: PhrasePart[]) => void;
}

/** La frase de *Personalizada*: texto libre con los datos como pastillas, y el menú «+ Dato». */
export function PhraseEditor({ phrase, samples, onChange }: PhraseEditorProps) {
  const { t } = useTranslation();
  const field = useRef<HTMLDivElement>(null);
  const shown = useRef<{ phrase: PhrasePart[]; pill: unknown } | null>(null);
  const lastCaret = useRef<Range | null>(null);
  const dragged = useRef<HTMLElement | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const addButton = useRef<HTMLButtonElement>(null);
  const menuId = useId();

  const names: Record<Datum, string> = {
    signer: t("panel.signed.field.signer"),
    issuer: t("panel.signed.field.issuer"),
    signedAt: t("panel.signed.field.date"),
  };
  const { signer, issuer, signedAt } = samples;
  const pill = useCallback(
    (datum: Datum) => {
      const element = document.createElement("span");
      element.className = "panel__phrase-datum";
      element.contentEditable = "false";
      element.draggable = true;
      element.dataset.datum = datum;
      element.textContent = { signer, issuer, signedAt }[datum];
      return element;
    },
    [signer, issuer, signedAt],
  );

  useLayoutEffect(() => {
    const root = field.current;
    if (root === null) return;
    if (shown.current?.pill === pill && samePhrase(shown.current.phrase, phrase)) return;
    renderPhrase(root, phrase, pill);
    shown.current = { phrase, pill };
  }, [phrase, pill]);

  const emit = () => {
    const root = field.current;
    if (root === null) return;
    const next = readPhrase(root);
    if (!isCanonical(root)) {
      renderPhrase(root, next, pill);
      const end = rangeAtEnd(root);
      document.getSelection()?.removeAllRanges();
      document.getSelection()?.addRange(end);
    }
    shown.current = { phrase: next, pill };
    onChange(next);
  };

  const insertAt = (range: Range, node: Node) => {
    range.deleteContents();
    range.insertNode(node);
    placeCaretAfter(node);
    emit();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      return;
    }
    if (event.key !== "Backspace" && event.key !== "Delete") return;
    const root = event.currentTarget;
    const target = pillBesideCaret(root, event.key === "Backspace" ? "before" : "after");
    if (target === null) return;
    event.preventDefault();
    const previous = target.previousSibling;
    target.remove();
    if (previous === null) {
      const start = document.createRange();
      start.setStart(root, 0);
      document.getSelection()?.removeAllRanges();
      document.getSelection()?.addRange(start);
    } else {
      placeCaretAfter(previous);
    }
    emit();
  };

  const onPaste = (event: ClipboardEvent<HTMLDivElement>) => {
    event.preventDefault();
    const text = event.clipboardData.getData("text/plain").replace(/\s*\n\s*/g, " ");
    const range = selectionInside(event.currentTarget) ?? rangeAtEnd(event.currentTarget);
    insertAt(range, document.createTextNode(text));
  };

  const onDragStart = (event: DragEvent<HTMLDivElement>) => {
    const target = event.target instanceof Node ? event.target : null;
    if (datumOf(target) === null) return;
    dragged.current = target as HTMLElement;
    event.dataTransfer?.setData("text/plain", target?.textContent ?? "");
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  };

  const onDragOver = (event: DragEvent<HTMLDivElement>) => {
    if (dragged.current !== null) event.preventDefault();
  };

  const onDrop = (event: DragEvent<HTMLDivElement>) => {
    const moving = dragged.current;
    dragged.current = null;
    if (moving === null) return;
    event.preventDefault();
    const root = event.currentTarget;
    const range = rangeAtPoint(root, event.clientX, event.clientY);
    if (moving.contains(range.startContainer)) return;
    const marker = document.createTextNode("");
    range.insertNode(marker);
    root.insertBefore(moving, marker.parentNode === root ? marker : null);
    marker.remove();
    placeCaretAfter(moving);
    emit();
  };

  const pick = (datum: Datum) => {
    setMenuOpen(false);
    const root = field.current;
    if (root === null) return;
    const saved = lastCaret.current;
    const range = saved !== null && root.contains(saved.startContainer) ? saved : rangeAtEnd(root);
    root.focus();
    insertAt(range, pill(datum));
  };

  const closeMenu = useCallback(() => setMenuOpen(false), []);

  return (
    <div className="panel__phrase" ref={container}>
      <div className="panel__phrase-field">
        {/* biome-ignore lint/a11y/useSemanticElements: un `<textarea>` no puede llevar las pastillas dentro del texto. */}
        <div
          ref={field}
          className="panel__phrase-text"
          contentEditable
          suppressContentEditableWarning
          role="textbox"
          aria-label={t("panel.visibleSignature.phrase.label")}
          tabIndex={0}
          onInput={emit}
          onKeyDown={onKeyDown}
          onPaste={onPaste}
          onBlur={(event) => {
            lastCaret.current = selectionInside(event.currentTarget);
          }}
          onDragStart={onDragStart}
          onDragOver={onDragOver}
          onDrop={onDrop}
          onDragEnd={() => {
            dragged.current = null;
          }}
        />
        <Button
          variant="secondary"
          ref={addButton}
          className="panel__phrase-add"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          aria-controls={menuOpen ? menuId : undefined}
          onClick={() => setMenuOpen((open) => !open)}
        >
          <PlusIcon size={14} strokeWidth={2} />
          {t("panel.visibleSignature.phrase.addDatum")}
        </Button>
      </div>
      <Menu
        open={menuOpen}
        onClose={closeMenu}
        anchorRef={container}
        returnFocusRef={addButton}
        id={menuId}
        aria-label={t("panel.visibleSignature.phrase.addDatum")}
        className="panel__phrase-menu"
      >
        {DATA.map((datum) => (
          <MenuItem key={datum} onClick={() => pick(datum)}>
            <span className="panel__phrase-option-name">{names[datum]}</span>
            <span className="panel__phrase-option-sample">{samples[datum]}</span>
          </MenuItem>
        ))}
      </Menu>
    </div>
  );
}
