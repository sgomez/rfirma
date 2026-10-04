//! El estado de la colocación de la firma visible: el recuadro, lo que recuerda cada modo de páginas, el modo activo y la página a la vista, repuestos solos al cambiar de documento; mide con la posición estándar que se le pasa y no conoce el PDF.

import { useCallback, useMemo, useState } from "react";
import {
  activating,
  firstSealedPage,
  type PageMode,
  type PageSet,
  type Placement,
  type Placing,
  pagesOf,
  placementOf,
  placingFrom,
  sealedPages,
  storing,
  type UserSpaceRect,
} from "./pageSets";

/** La posición estándar del recuadro en una página del documento. */
export type StandardRectOn = (page: number) => Promise<UserSpaceRect>;

/** El documento que se tiene delante; cada valor nuevo es otro documento, o el mismo vuelto a abrir. */
export interface PlacedDocument {
  placement: Placement | null;
  pageCount: number;
}

interface PlacementOptions {
  /** `null` cuando no hay documento delante. */
  document: PlacedDocument | null;
  /** `null` cuando no hay documento que medir. */
  standardRectOn: StandardRectOn | null;
  onChange?: (placement: Placement | null) => unknown;
}

interface Front {
  document: PlacedDocument | null;
  placing: Placing;
  viewedPage: number;
}

function frontOf(document: PlacedDocument | null): Front {
  const saved = document?.placement ?? null;
  return {
    document,
    placing: placingFrom(saved, document?.pageCount ?? 0),
    viewedPage: firstSealedPage(saved) ?? 1,
  };
}

/** La colocación de la firma visible y las órdenes que la cambian. */
export function usePlacement({ document, standardRectOn, onChange }: PlacementOptions) {
  const [front, setFront] = useState(() => frontOf(document));
  let current = front;
  if (front.document !== document) {
    current = frontOf(document);
    setFront(current);
  }
  const { placing, viewedPage } = current;
  const pageCount = document?.pageCount ?? 0;
  const pageMode = placing.mode;
  const placement = useMemo(() => placementOf(placing.rect, placing.sets, placing.mode), [placing]);

  const viewPage = useCallback((page: number) => {
    setFront((was) => ({ ...was, viewedPage: page }));
  }, []);

  const apply = useCallback(
    (next: Placing) => {
      setFront((was) => ({ ...was, placing: next }));
      onChange?.(placementOf(next.rect, next.sets, next.mode));
    },
    [onChange],
  );

  const placeStandard = useCallback(
    async (next: Placing) => {
      const pages = pagesOf(next.sets, next.mode);
      if (next.rect !== null || pages === null || standardRectOn === null) return apply(next);
      const first = sealedPages(pages, pageCount)[0] ?? 1;
      apply({ ...next, rect: await standardRectOn(first) });
    },
    [apply, standardRectOn, pageCount],
  );

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

  const choosePages = useCallback(
    (pages: PageSet | null) => {
      void placeStandard({
        ...placing,
        sets: storing(placing.sets, placing.mode, pages, pageCount),
      });
    },
    [placeStandard, placing, pageCount],
  );

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
    pageMode,
    placement,
    viewedPage,
    viewPage,
    rememberPlacement,
    choosePages,
    changePageMode,
    placeOnViewedPage,
  };
}
