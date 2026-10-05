//! La geometría de la página que lleva el recuadro, leída del PDF abierto.

import { useEffect, useState } from "react";
import type { PdfDocument } from "../viewer/pdf";
import type { PageGeometry } from "./signingOrder";

/** La geometría de la página que lleva el recuadro, leída del PDF abierto. */
export function usePageGeometry(pdf: PdfDocument | null, boxPage: number | null) {
  const [geometry, setGeometry] = useState<PageGeometry | null>(null);
  useEffect(() => {
    if (pdf === null || boxPage === null) {
      setGeometry(null);
      return;
    }
    let current = true;
    void pdf.getPage(boxPage).then((page) => {
      if (current) setGeometry({ page: boxPage, view: page.view, rotate: page.rotate });
    });
    return () => {
      current = false;
    };
  }, [pdf, boxPage]);
  return geometry;
}
