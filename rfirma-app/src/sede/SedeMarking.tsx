//! 1c · El área de la firma visible trazada sobre el PDF, y sus páginas, con el visor y el segmentado de la ventana principal, antes del consentimiento.

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { usePlacementControls } from "../App.usePlacementControls";
import { firstSealedPage, type Placement } from "../placement/pageSets";
import { PlacementFieldset } from "../signing/PlacementFieldset";
import { usePlacementField } from "../signing/usePlacementField";
import { DocumentViewer } from "../viewer/DocumentViewer";
import type { PdfDocument } from "../viewer/pdf";
import type { MarkedArea } from "./errand";
import { SedeBody } from "./SedeFrame";
import "../signing/SigningPanel.css";
import { Button } from "../design-system/Button";

interface SedeMarkingProps {
  pdf: PdfDocument | null;
  onMark: (area: MarkedArea) => void | Promise<void>;
  onCancel: () => void;
}

/**
 * **1c · Marcar el área de la firma visible.** La sede pide `visibleSignature`
 * y la persona traza el recuadro sobre el PDF con el mismo visor de la ventana
 * principal, y elige sus páginas, antes de elegir certificado.
 */
export function SedeMarking({ pdf, onMark, onCancel }: SedeMarkingProps) {
  const { t } = useTranslation();
  const [viewedPage, setViewedPage] = useState(1);
  const [placementRequest, setPlacementRequest] = useState<{
    action: "seal" | "unseal";
  } | null>(null);
  const { placing, pageChoice, placement, rememberPlacement, choosePages, changePageChoice } =
    usePlacementControls(pdf, keepNowhere, viewedPage);
  const { pagesText, rangeError, pageButton, typePages } = usePlacementField({
    documentPages: pdf?.pageCount ?? 0,
    pageSets: placing.sets,
    pageChoice,
    placement,
    viewedPage,
    onChoosePages: choosePages,
    onSeal: () => setPlacementRequest({ action: "seal" }),
    onUnseal: () => setPlacementRequest({ action: "unseal" }),
  });
  const [handing, setHanding] = useState(false);

  const accept = async () => {
    if (pdf === null || placement === null) return;
    setHanding(true);
    try {
      await onMark(await markedAreaOf(pdf, placement));
    } finally {
      setHanding(false);
    }
  };

  return (
    <SedeBody
      flush
      onEscape={onCancel}
      footer={
        <>
          <div className="sede-window__spacer" />
          <Button variant="ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </Button>
          <Button
            variant="primary"
            disabled={placement === null || rangeError !== null || handing}
            onClick={() => void accept()}
          >
            {t("actions.continue")}
          </Button>
        </>
      }
    >
      <div className="sede-marking">
        {pdf !== null && (
          <div className="sede-marking__viewer">
            <DocumentViewer
              pdf={pdf}
              placement={placement}
              onPlace={rememberPlacement}
              pageChoice={pageChoice}
              onPageChange={setViewedPage}
              placementRequest={placementRequest}
              onOpen={noop}
            />
          </div>
        )}
        <aside className="panel__scroll sede-marking__panel">
          <p className="rf-title">{t("sede.marking.title")}</p>
          {pdf === null ? (
            <p className="rf-prose">{t("sede.marking.unreadable")}</p>
          ) : (
            <>
              <p className="rf-prose rf-text-muted">{t("sede.marking.hint")}</p>
              <section className="panel__placement" aria-label={t("panel.placement.title")}>
                <p className="rf-label panel__heading">{t("panel.placement.title")}</p>
                <PlacementFieldset
                  pageSets={placing.sets}
                  pageChoice={pageChoice}
                  onChangePageChoice={changePageChoice}
                  pagesText={pagesText}
                  onTypePages={typePages}
                  rangeError={rangeError}
                  pageButton={pageButton}
                />
              </section>
            </>
          )}
        </aside>
      </div>
    </SedeBody>
  );
}

async function markedAreaOf(pdf: PdfDocument, placement: Placement): Promise<MarkedArea> {
  const first = firstSealedPage(placement) ?? 1;
  const page = await pdf.getPage(first);
  const { x0, y0, x1, y1 } = placement.rect;
  return {
    page: first,
    pages: placement.pages,
    pageCount: pdf.pageCount,
    mediaBox: page.view,
    rotation: page.rotate,
    rect: [x0, y0, x1, y1],
  };
}

async function keepNowhere() {}

function noop() {}
