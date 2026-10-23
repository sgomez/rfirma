//! 1c · Marcar el área de la firma visible con el esqueleto de la ventana ampliada del consentimiento; el recuadro nace en la posición estándar.

import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Admonition } from "../design-system/Admonition";
import { Button } from "../design-system/Button";
import { Stack } from "../design-system/Stack";
import { useDefaultButton } from "../design-system/useDefaultButton";
import { PlacementBlock } from "../placement/PlacementBlock";
import { firstSealedPage, type Placement } from "../placement/pageSets";
import { usePlacement } from "../placement/usePlacement";
import { DocumentViewer } from "../viewer/DocumentViewer";
import type { PdfDocument } from "../viewer/pdf";
import { pageBoundsOnPageOf, standardRectOnPageOf } from "../viewer/signatureBox";
import type { MarkedArea } from "./errand";
import { SedeBody } from "./SedeFrame";
import { SedeOrigin } from "./SedeOrigin";
import "../signing/SigningPanel.css";
import "./SedeReceivedConsent.css";
import "./SedeAreaMarking.css";

interface SedeAreaMarkingProps {
  pdf: PdfDocument | null;
  origin: string | null;
  asks?: "signature" | "identity";
  onMark: (area: MarkedArea) => void | Promise<void>;
  onCancel: () => void;
}

/** En la columna de 400 px a la izquierda, quién pide la firma, la colocación y el pie del consentimiento ampliado, y el documento a la derecha, con su barra. */
export function SedeAreaMarking({ pdf, origin, asks, onMark, onCancel }: SedeAreaMarkingProps) {
  const { t } = useTranslation();
  const standardRectOn = useMemo(() => (pdf === null ? null : standardRectOnPageOf(pdf)), [pdf]);
  const pageBoundsOn = useMemo(() => (pdf === null ? null : pageBoundsOnPageOf(pdf)), [pdf]);
  const document = useMemo(
    () => (pdf === null ? null : { placement: null, pageCount: pdf.pageCount }),
    [pdf],
  );
  const placementState = usePlacement({ document, standardRectOn, pageBoundsOn });
  const { placement, rangeError, viewPage, moveBox, sealPage, placeOnViewedPage } = placementState;
  const placedFor = useRef<PdfDocument | null>(null);
  useEffect(() => {
    if (pdf === null || placedFor.current === pdf) return;
    placedFor.current = pdf;
    placeOnViewedPage();
  }, [pdf, placeOnViewedPage]);
  const [handing, setHanding] = useState(false);
  const ready = placement !== null && rangeError === null && !handing;
  const continueButton = useDefaultButton(ready);

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
    <div className="sede-received-wide">
      <div className="sede-received-wide__side">
        <SedeBody
          flush
          steadyFooter
          primary={continueButton}
          onEscape={onCancel}
          footer={
            <>
              <Button variant="ghost" className="sede-received__steady" onClick={onCancel}>
                {t("actions.cancel")}
              </Button>
              <Button
                ref={continueButton}
                variant="primary"
                className="sede-received__steady"
                disabled={!ready}
                onClick={() => void accept()}
              >
                {t("actions.continue")}
              </Button>
            </>
          }
        >
          <div className="sede-received">
            <SedeOrigin origin={origin} asks={asks} />
            <div className="sede-received__body sede-area-marking__body">
              <p className="rf-title sede-area-marking__title">{t("sede.marking.title")}</p>
              {pdf === null ? (
                <Admonition tone="error">{t("sede.marking.unreadable")}</Admonition>
              ) : (
                <Stack as="section" aria-label={t("panel.placement.title")}>
                  <p className="rf-label sede-area-marking__label">{t("panel.placement.title")}</p>
                  <PlacementBlock state={placementState} />
                </Stack>
              )}
            </div>
          </div>
        </SedeBody>
      </div>
      <div className="sede-received-wide__viewer">
        <div className="sede-area-marking__viewer">
          {pdf !== null && (
            <DocumentViewer
              pdf={pdf}
              placement={placement}
              onMove={moveBox}
              onTrace={sealPage}
              onPageChange={viewPage}
              onOpen={noop}
            />
          )}
        </div>
      </div>
    </div>
  );
}

function noop() {}

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
