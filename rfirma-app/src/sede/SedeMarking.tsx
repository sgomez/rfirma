//! 1c · El área de la firma visible trazada sobre el PDF, y sus páginas, con el visor y el segmentado de la ventana principal, antes del consentimiento.

import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { PlacementBlock } from "../placement/PlacementBlock";
import { firstSealedPage, type Placement } from "../placement/pageSets";
import { usePlacement } from "../placement/usePlacement";
import { DocumentViewer } from "../viewer/DocumentViewer";
import type { PdfDocument } from "../viewer/pdf";
import { standardRectOnPageOf } from "../viewer/signatureBox";
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
  const standardRectOn = useMemo(() => (pdf === null ? null : standardRectOnPageOf(pdf)), [pdf]);
  const document = useMemo(
    () => (pdf === null ? null : { placement: null, pageCount: pdf.pageCount }),
    [pdf],
  );
  const placementState = usePlacement({ document, standardRectOn });
  const { placement, rangeError, viewPage, moveBox, sealPage } = placementState;
  const [handing, setHanding] = useState(false);
  const continueButton = useRef<HTMLButtonElement>(null);

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
      primary={continueButton}
      onEscape={onCancel}
      footer={
        <>
          <div className="sede-window__spacer" />
          <Button variant="ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </Button>
          <Button
            ref={continueButton}
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
              onMove={moveBox}
              onTrace={sealPage}
              onPageChange={viewPage}
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
                <PlacementBlock state={placementState} />
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

function noop() {}
