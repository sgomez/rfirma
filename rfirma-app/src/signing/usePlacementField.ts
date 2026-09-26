import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { PageChoice, PageSet, PageSets, Placement } from "../viewer/signatureBox";
import { sealsPage } from "../viewer/signatureBox";
import { formatPageRange, parsePageRange } from "./pageRange";
import type { FieldTrouble, PageButton } from "./placementField";

interface UsePlacementFieldArgs {
  documentPages: number;
  pageSets: PageSets;
  pageChoice: PageChoice;
  placement: Placement | null;
  viewedPage: number;
  onChoosePages: (pages: PageSet | null) => void;
  onSeal: () => void;
  onUnseal: () => void;
}

/**
 * El campo de «Varias»: lo tecleado en el campo de páginas se mantiene a la
 * par del conjunto activo por **identidad** —el conjunto que este hook acaba
 * de emitir se apunta en `seenPages`, así que solo se reescribe el campo
 * cuando el conjunto cambia **desde fuera**, sellar o quitar una página en el
 * visor (ID-99)—. Sin esa distinción, teclear `1,2-3` se convertiría en `1-3`
 * bajo los dedos, porque la forma comprimida no es la que se está escribiendo.
 */
export function usePlacementField({
  documentPages,
  pageSets,
  pageChoice,
  placement,
  viewedPage,
  onChoosePages,
  onSeal,
  onUnseal,
}: UsePlacementFieldArgs) {
  const { t } = useTranslation();
  // El campo lo escribe **el conjunto de «Varias»**, y no el conjunto
  // activo: con «Una página» o «Todas» delante el campo ni se pinta, y al
  // volver tiene que traer lo que se tecleó allí, no lo que dejó la otra
  // opción (#188).
  const pages = pageSets.these;
  const [pagesText, setPagesText] = useState(() =>
    pages === null ? "" : formatPageRange(pages, documentPages),
  );
  const [seenPages, setSeenPages] = useState<PageSet | null>(pages);
  if (pages !== seenPages) {
    setSeenPages(pages);
    setPagesText(pages === null ? "" : formatPageRange(pages, documentPages));
  }
  const parsed = parsePageRange(pagesText, documentPages);
  // El campo vacío bajo «Varias» es **una situación más**, no un
  // conjunto: no nombra ninguna página, lo dice bajo el campo y apaga el botón
  // de firmar.
  const rangeError: FieldTrouble | null =
    pageChoice !== "these"
      ? null
      : pagesText.trim() === ""
        ? { kind: "empty" }
        : !parsed.ok
          ? parsed.error
          : null;
  // Elegir páginas **coloca** (#185): quien recibe esto pone el recuadro en su
  // posición estándar si todavía no había ninguno. El hook no sabe dónde cae
  // —no mide páginas— y por eso manda el conjunto y nada más.
  const place = (next: PageSet | null) => {
    setSeenPages(next);
    onChoosePages(next);
  };

  const typePages = (value: string) => {
    setPagesText(value);
    const typed = parsePageRange(value, documentPages);
    // Lo que no se entiende **no se aplica a medias**: el conjunto se queda
    // como estaba y el error apaga el botón de firmar (ID-22, ID-98). Y el
    // campo vacío tampoco se aplica: borrarlo es el paso normal para
    // reescribir el rango, y emitir `place(null)` se llevaría la colocación
    // entera sin camino de vuelta desde el campo.
    if (typed.ok && typed.pages !== null) place(typed.pages);
  };

  const here = placement !== null && sealsPage(placement.pages, viewedPage);
  const pageButton: PageButton | null =
    pageChoice === "all" || (pageChoice === "single" && here) || rangeError !== null
      ? null
      : here
        ? { label: t("panel.placement.unseal"), act: onUnseal }
        : { label: t("panel.placement.seal"), act: onSeal };

  return { pagesText, rangeError, pageButton, typePages };
}
