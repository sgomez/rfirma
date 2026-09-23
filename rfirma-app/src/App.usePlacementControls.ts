import { useCallback, useMemo, useState } from "react";
import type { Placing } from "./App.signingOrder";
import {
  activating,
  NO_PAGE_SETS,
  type PageChoice,
  type PageSet,
  type Placement,
  pagesOf,
  placementOf,
  sealedPages,
  standardRectOf,
  storing,
} from "./viewer/signatureBox";
import type { PdfDocument } from "./viewer/pdf";

/**
 * El bloque «Colocación»: el recuadro y las tres opciones de página que lo
 * llenan (#185, #188).
 *
 * `placement` —lo que ve el resto de la ventana y lo que cruza a firmar— es el
 * recuadro con el conjunto de la opción activa, y por eso se deriva en vez de
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
    choice: "single",
  });
  const pageChoice = placing.choice;
  const placement = useMemo(
    () => placementOf(placing.rect, placing.sets, placing.choice),
    [placing],
  );
  const pageCount = pdf?.pageCount ?? 0;

  // El único camino por el que la colocación cambia: fija las tres piezas y
  // apunta en la fila **lo que la opción activa deja ver**, que es lo que se
  // firmaría si se firmara ahora.
  const apply = useCallback(
    (next: Placing) => {
      setPlacing(next);
      void placeDocument(placementOf(next.rect, next.sets, next.choice));
    },
    [placeDocument],
  );

  /**
   * Lo mismo, pero **poniendo el recuadro si no hay ninguno** (#185).
   *
   * Es la mitad que le faltaba al bloque «Colocación»: elegir una opción o
   * teclear un rango sobre un documento sin recuadro ya no se queda esperando
   * un gesto sobre la hoja, deja el recuadro en su posición estándar. La página
   * que lo mide es la primera del conjunto, la misma que mide la firma (ID-96).
   */
  const placeStandard = useCallback(
    async (next: Placing) => {
      const pages = pagesOf(next.sets, next.choice);
      if (next.rect !== null || pages === null || pdf === null) return apply(next);
      const first = sealedPages(pages, pdf.pageCount)[0] ?? 1;
      const page = await pdf.getPage(first);
      apply({ ...next, rect: standardRectOf(page.getViewport({ scale: 1 })) });
    },
    [apply, pdf],
  );

  // Lo que llega del visor: colocar, mover o redimensionar el recuadro y tocar
  // el conjunto **de la opción activa**. `null` es quitar el sello de la última
  // página de esa opción; las otras dos conservan el suyo (ID-92, #188).
  const rememberPlacement = useCallback(
    (next: Placement | null) => {
      apply({
        rect: next?.rect ?? placing.rect,
        sets: storing(placing.sets, placing.choice, next?.pages ?? null, pageCount),
        choice: placing.choice,
      });
    },
    [apply, placing, pageCount],
  );

  // Lo que llega del panel: solo el conjunto de la opción activa cambia, y el
  // recuadro nace si no había ninguno.
  const choosePages = useCallback(
    (pages: PageSet | null) => {
      void placeStandard({
        ...placing,
        sets: storing(placing.sets, placing.choice, pages, pageCount),
      });
    },
    [placeStandard, placing, pageCount],
  );

  // Cambiar de opción **no reescribe la que dejas**: la nueva trae lo suyo, y
  // solo se siembra de la anterior si se estrena (#188).
  const changePageChoice = useCallback(
    (choice: PageChoice) => {
      const previous = pagesOf(placing.sets, placing.choice);
      void placeStandard({
        rect: placing.rect,
        sets: activating(placing.sets, choice, previous, pageCount, viewedPage),
        choice,
      });
    },
    [placeStandard, placing, pageCount, viewedPage],
  );

  return {
    placing,
    setPlacing,
    pageChoice,
    placement,
    pageCount,
    apply,
    placeStandard,
    rememberPlacement,
    choosePages,
    changePageChoice,
  };
}
