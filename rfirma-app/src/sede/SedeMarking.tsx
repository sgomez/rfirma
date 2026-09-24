import { useState } from "react";
import { useTranslation } from "react-i18next";
import { DocumentViewer } from "../viewer/DocumentViewer";
import type { PdfDocument } from "../viewer/pdf";
import { firstSealedPage, type Placement } from "../viewer/signatureBox";
import type { MarkedArea } from "./errand";
import { SedeBody } from "./SedeFrame";

interface SedeMarkingProps {
  pdf: PdfDocument | null;
  onMark: (area: MarkedArea) => void | Promise<void>;
  onCancel: () => void;
}

/**
 * **1c · Marcar el área de la firma visible.** La sede pide `visibleSignature`
 * y la persona traza el recuadro sobre el PDF con el mismo visor de la ventana
 * principal, antes de elegir certificado.
 */
export function SedeMarking({ pdf, onMark, onCancel }: SedeMarkingProps) {
  const { t } = useTranslation();
  const [placement, setPlacement] = useState<Placement | null>(null);
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
      footer={
        <>
          <div className="sede-window__spacer" />
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </button>
          <button
            type="button"
            className="rf-btn rf-btn--primary"
            disabled={placement === null || handing}
            onClick={() => void accept()}
          >
            {t("sede.marking.accept")}
          </button>
        </>
      }
    >
      <div className="rf-stack sede-marking">
        <p className="rf-title">{t("sede.marking.title")}</p>
        {pdf === null ? (
          <p className="rf-prose">{t("sede.marking.unreadable")}</p>
        ) : (
          <>
            <p className="rf-prose rf-text-muted">{t("sede.marking.hint")}</p>
            <div className="sede-marking__viewer">
              <DocumentViewer
                pdf={pdf}
                placement={placement}
                onPlace={setPlacement}
                onOpen={noop}
                pageChoice="single"
              />
            </div>
          </>
        )}
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
