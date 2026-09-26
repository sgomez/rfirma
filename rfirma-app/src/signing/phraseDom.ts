/** La frase de *Personalizada* leída de su campo editable y escrita en él; no es el componente. */
import type { Datum, PhrasePart } from "./visibleSignature";

const DATA: readonly Datum[] = ["signer", "issuer", "signedAt"];

function normalizePhrase(parts: readonly PhrasePart[]): PhrasePart[] {
  const normalized: PhrasePart[] = [];
  for (const part of parts) {
    const last = normalized.at(-1);
    if ("datum" in part) normalized.push(part);
    else if (part.text === "") continue;
    else if (last !== undefined && "text" in last) last.text += part.text;
    else normalized.push({ text: part.text });
  }
  return normalized;
}

export function samePhrase(a: readonly PhrasePart[], b: readonly PhrasePart[]): boolean {
  return JSON.stringify(normalizePhrase(a)) === JSON.stringify(normalizePhrase(b));
}

export function datumOf(node: Node | null): Datum | null {
  if (!(node instanceof HTMLElement)) return null;
  const datum = node.dataset.datum;
  return DATA.find((known) => known === datum) ?? null;
}

export function readPhrase(root: HTMLElement): PhrasePart[] {
  const parts: PhrasePart[] = [];
  for (const node of Array.from(root.childNodes)) {
    const datum = datumOf(node);
    if (datum !== null) parts.push({ datum });
    else parts.push({ text: (node.textContent ?? "").replace(/ /g, " ") });
  }
  return normalizePhrase(parts);
}

/** Si el campo solo tiene texto y pastillas, sin lo que el navegador mete al pegar o al soltar. */
export function isCanonical(root: HTMLElement): boolean {
  return Array.from(root.childNodes).every(
    (node) => node.nodeType === Node.TEXT_NODE || datumOf(node) !== null,
  );
}

export function renderPhrase(
  root: HTMLElement,
  phrase: readonly PhrasePart[],
  pill: (datum: Datum) => HTMLElement,
) {
  root.replaceChildren(
    ...phrase.map((part) =>
      "datum" in part ? pill(part.datum) : root.ownerDocument.createTextNode(part.text),
    ),
  );
}

/** La pastilla pegada al cursor, del lado hacia el que borra la tecla. */
export function pillBesideCaret(root: HTMLElement, side: "before" | "after"): HTMLElement | null {
  const selection = root.ownerDocument.getSelection();
  if (selection === null || selection.rangeCount === 0 || !selection.isCollapsed) return null;
  const { startContainer: container, startOffset: offset } = selection.getRangeAt(0);
  let neighbour: Node | null = null;
  if (container === root) {
    neighbour = root.childNodes[side === "before" ? offset - 1 : offset] ?? null;
  } else if (container instanceof Text && container.parentNode === root) {
    if (side === "before" && offset === 0) neighbour = container.previousSibling;
    if (side === "after" && offset === container.length) neighbour = container.nextSibling;
  }
  return datumOf(neighbour) === null ? null : (neighbour as HTMLElement);
}

export function selectionInside(root: HTMLElement): Range | null {
  const selection = root.ownerDocument.getSelection();
  if (selection === null || selection.rangeCount === 0) return null;
  const range = selection.getRangeAt(0);
  return root.contains(range.startContainer) && root.contains(range.endContainer)
    ? range.cloneRange()
    : null;
}

export function rangeAtEnd(root: HTMLElement): Range {
  const range = root.ownerDocument.createRange();
  range.selectNodeContents(root);
  range.collapse(false);
  return range;
}

/** El punto del campo bajo el puntero al soltar, o su final si el navegador no sabe decirlo. */
export function rangeAtPoint(root: HTMLElement, x: number, y: number): Range {
  const document = root.ownerDocument as Document & {
    caretRangeFromPoint?: (x: number, y: number) => Range | null;
    caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node; offset: number } | null;
  };
  let range = document.caretRangeFromPoint?.(x, y) ?? null;
  const position = range === null ? document.caretPositionFromPoint?.(x, y) : null;
  if (position) {
    range = document.createRange();
    range.setStart(position.offsetNode, position.offset);
  }
  return range !== null && root.contains(range.startContainer) ? range : rangeAtEnd(root);
}

export function placeCaretAfter(node: Node) {
  const range = node.ownerDocument?.createRange();
  if (!range) return;
  range.setStartAfter(node);
  range.collapse(true);
  const selection = node.ownerDocument?.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
}
