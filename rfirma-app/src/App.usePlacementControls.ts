//! La colocación de la firma visible: el recuadro y los tres modos de página.

import { useCallback, useMemo, useState } from "react";
import type { Placing } from "./placement/pageSets";
import {
  activating,
  NO_PAGE_SETS,
  type PageMode,
  type PageSet,
  type Placement,
  pagesOf,
  placementOf,
  sealedPages,
  storing,
} from "./placement/pageSets";
import type { PdfDocument } from "./viewer/pdf";
import { standardRectOf } from "./viewer/signatureBox";

/**
 * La colocación de la firma visible: el recuadro y los tres modos de página que lo
 * llenan.
 *
 * `placement` —lo que ve el resto de la ventana y lo que cruza a firmar— es el
 * recuadro con el conjunto del modo activo, y por eso se deriva en vez de
 * guardarse: dos copias del mismo dato es de dónde salían las páginas que se
 * sumaban solas en `Solo 1 página`.
 */
export function usePlacementControls(
  pdf: PdfDocument | null,
  placeDocument: (placement: Placement | null) => Promise<void>,
  viewedPage: number,
) {
  const [placing, setPlacing] = useState<Placing>({
    rect: null,
    sets: NO_PAGE_SETS,
    mode: "single",
  });
  const pageMode = placing.mode;
  const placement = useMemo(() => placementOf(placing.rect, placing.sets, placing.mode), [placing]);
  const pageCount = pdf?.pageCount ?? 0;

  // El único camino por el que la colocación cambia: fija las tres piezas y
  // apunta en la fila **lo que el modo activo deja ver**, que es lo que se
  // firmaría si se firmara ahora.
  const apply = useCallback(
    (next: Placing) => {
      setPlacing(next);
      void placeDocument(placementOf(next.rect, next.sets, next.mode));
    },
    [placeDocument],
  );

  /**
   * Lo mismo, pero **poniendo el recuadro si no hay ninguno**.
   *
   * Es la mitad que le faltaba al bloque «Colocación»: elegir un modo o
   * teclear un rango sobre un documento sin recuadro ya no se queda esperando
   * un gesto sobre la hoja, deja el recuadro en su posición estándar. La página
   * que lo mide es la primera del conjunto, la misma que mide la firma.
   */
  const placeStandard = useCallback(
    async (next: Placing) => {
      const pages = pagesOf(next.sets, next.mode);
      if (next.rect !== null || pages === null || pdf === null) return apply(next);
      const first = sealedPages(pages, pdf.pageCount)[0] ?? 1;
      const page = await pdf.getPage(first);
      apply({ ...next, rect: standardRectOf(page.getViewport({ scale: 1 })) });
    },
    [apply, pdf],
  );

  // Lo que llega del visor: colocar, mover o redimensionar el recuadro y tocar
  // el conjunto **del modo activo**. `null` es quitar el sello de la última
  // página de ese modo; los otros dos conservan el suyo.
  const rememberPlacement = useCallback(
    (next: Placement | null) => {
      apply({
        rect: next?.rect ?? placing.rect,
        sets: storing(placing.sets, placing.mode, next?.pages ?? null, pageCount),
        mode: placing.mode,
      });
    },
    [apply, placing, pageCount],
  );

  // Lo que llega del panel: solo el conjunto del modo activo cambia, y el
  // recuadro nace si no había ninguno.
  const choosePages = useCallback(
    (pages: PageSet | null) => {
      void placeStandard({
        ...placing,
        sets: storing(placing.sets, placing.mode, pages, pageCount),
      });
    },
    [placeStandard, placing, pageCount],
  );

  // Cambiar de modo **no reescribe el que dejas**: el nuevo trae lo suyo, y
  // solo se siembra de la anterior si se estrena.
  const changePageMode = useCallback(
    (mode: PageMode) => {
      const previous = pagesOf(placing.sets, placing.mode);
      void placeStandard({
        rect: placing.rect,
        sets: activating(placing.sets, mode, previous, pageCount, viewedPage),
        mode,
      });
    },
    [placeStandard, placing, pageCount, viewedPage],
  );

  const placeOnViewedPage = useCallback(() => {
    const page = Math.min(Math.max(viewedPage, 1), Math.max(pageCount, 1));
    const sets = activating(placing.sets, placing.mode, null, pageCount, page);
    const pages = pagesOf(sets, placing.mode);
    void placeStandard({
      ...placing,
      sets: pages === null ? storing(sets, placing.mode, { only: [page] }, pageCount) : sets,
    });
  }, [placeStandard, placing, pageCount, viewedPage]);

  return {
    placing,
    setPlacing,
    pageMode,
    placement,
    pageCount,
    apply,
    placeStandard,
    rememberPlacement,
    choosePages,
    changePageMode,
    placeOnViewedPage,
  };
}
