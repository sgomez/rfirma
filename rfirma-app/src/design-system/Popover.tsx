//! El panel flotante de los desplegables: cierre al pulsar fuera, con Escape o con Tab, foco al abrir y al cerrar, y colocación opcional en un portal.

import {
  type ComponentPropsWithRef,
  type RefObject,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";

type ElementRef = RefObject<HTMLElement | null>;

export type PopoverProps = Omit<ComponentPropsWithRef<"div">, "style"> & {
  open: boolean;
  /** Pide el cierre; quien posee el estado decide qué es «cerrado». */
  onClose: () => void;
  /** El disparador y todo lo que cuenta como dentro: pulsarlo no cierra, y de él cuelga el portal. */
  anchorRef: ElementRef;
  /** Qué enfoca al abrirse: el propio panel o un elemento de fuera de él. Sin valor, el foco no se mueve. */
  initialFocus?: "panel" | ElementRef;
  /** A quién vuelve el foco al cerrar. Sin valor, no vuelve a nadie. */
  returnFocusRef?: ElementRef;
  /** `"escape"`: solo al descartar con Escape; `"always"`: también al elegir. Pulsar fuera o tabular nunca lo devuelven. */
  restoreFocus?: "escape" | "always";
  /** Pinta el panel en `body`, bajo el ancla, con el ancho de esta y recortado a la ventana. */
  portal?: { maxHeight: number };
};

interface Placement {
  top: number;
  left: number;
  width: number;
  maxHeight: number;
}

const WINDOW_MARGIN = 8;
const ANCHOR_GAP = 4;

/** El panel de un desplegable, con el comportamiento que comparten el selector, las preferencias y los menús. */
export function Popover({
  open,
  onClose,
  anchorRef,
  initialFocus,
  returnFocusRef,
  restoreFocus = "escape",
  portal,
  children,
  ref,
  ...rest
}: PopoverProps) {
  const own = useRef<HTMLDivElement | null>(null);
  const skipRestore = useRef(false);
  const dismissedByEscape = useRef(false);
  const placement = usePlacement(open && portal !== undefined, anchorRef, own, portal?.maxHeight);

  const setPanel = useCallback(
    (node: HTMLDivElement | null) => {
      own.current = node;
      if (typeof ref === "function") ref(node);
      else if (ref) ref.current = node;
    },
    [ref],
  );

  useEffect(() => {
    if (!open) return;
    skipRestore.current = false;
    dismissedByEscape.current = false;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (anchorRef.current?.contains(target) || own.current?.contains(target)) return;
      skipRestore.current = true;
      onClose();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        dismissedByEscape.current = true;
        onClose();
      } else if (event.key === "Tab") {
        skipRestore.current = true;
        onClose();
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown, true);
    };
  }, [open, anchorRef, onClose]);

  const shown = open && (portal === undefined || placement !== null);
  const wasShown = useRef(false);
  useEffect(() => {
    if (shown && !wasShown.current) {
      if (initialFocus === "panel") own.current?.focus();
      else initialFocus?.current?.focus();
    } else if (!shown && wasShown.current && !skipRestore.current) {
      if (restoreFocus === "always" || dismissedByEscape.current) returnFocusRef?.current?.focus();
    }
    wasShown.current = shown;
  }, [shown, initialFocus, returnFocusRef, restoreFocus]);

  if (!open) return null;
  if (portal === undefined) {
    return (
      <div ref={setPanel} {...rest}>
        {children}
      </div>
    );
  }
  if (placement === null) return null;
  return createPortal(
    <div ref={setPanel} style={{ position: "fixed", ...placement }} {...rest}>
      {children}
    </div>,
    document.body,
  );
}

function usePlacement(
  active: boolean,
  anchorRef: ElementRef,
  panel: ElementRef,
  maxHeight = Number.POSITIVE_INFINITY,
): Placement | null {
  const [placement, setPlacement] = useState<Placement | null>(null);
  const place = useCallback(() => {
    const rect = anchorRef.current?.getBoundingClientRect();
    if (!rect) return;
    const top = rect.bottom + ANCHOR_GAP;
    setPlacement({
      top,
      left: rect.left,
      width: rect.width,
      maxHeight: Math.min(maxHeight, window.innerHeight - top - WINDOW_MARGIN),
    });
  }, [anchorRef, maxHeight]);

  useEffect(() => {
    if (!active) {
      setPlacement(null);
      return;
    }
    place();
    const onScroll = (event: Event) => {
      if (panel.current?.contains(event.target as Node)) return;
      place();
    };
    window.addEventListener("resize", place);
    window.addEventListener("scroll", onScroll, true);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", onScroll, true);
    };
  }, [active, place, panel]);

  return placement;
}
