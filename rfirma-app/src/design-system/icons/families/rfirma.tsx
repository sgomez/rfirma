//! La familia rFirma: los contornos que transcriben los artboards, con los veredictos y la tarjeta de Heroicons.

import { type IconFamily, stroked } from "../glyph";
import { OWN } from "../origins";
import { SHARED_GLYPHS } from "./shared";

const chevronRight = stroked(OWN, <path d="M9 6l6 6-6 6" />);
const chevronDown = stroked(OWN, <path d="M6 9l6 6 6-6" />);
const check = stroked(OWN, <path d="M4 12.5 9.5 18 20 6" />, 3);
const plus = stroked(OWN, <path d="M12 5v14M5 12h14" />);

export const RFIRMA_ICONS: IconFamily = {
  name: "rFirma",
  glyphs: {
    ...SHARED_GLYPHS,
    add: plus,
    close: stroked(OWN, <path d="M6 6l12 12M18 6 6 18" />, 1.8),
    collapsed: chevronRight,
    copy: stroked(
      OWN,
      <>
        <rect x="9" y="9" width="11" height="11" rx="2" />
        <path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1" />
      </>,
    ),
    document: stroked(
      OWN,
      <>
        <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
        <path d="M14 3v5h5" />
      </>,
    ),
    done: check,
    dropdown: chevronDown,
    dropFile: stroked(
      OWN,
      <>
        <path d="M12 16V4M8 8l4-4 4 4" />
        <path d="M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3" />
      </>,
    ),
    expanded: chevronDown,
    externalLink: stroked(
      OWN,
      <>
        <path d="M14 4h6v6" />
        <path d="M20 4 11 13" />
        <path d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4" />
      </>,
      1.8,
    ),
    firstPage: stroked(OWN, <path d="M17 6l-6 6 6 6M9 6l-6 6 6 6" />),
    fitPage: stroked(
      OWN,
      <>
        <rect x="7" y="4" width="10" height="16" rx="1" />
        <path d="M4 8V4h3M20 8V4h-3M4 16v4h3M20 16v4h-3" />
      </>,
    ),
    fitWindow: stroked(OWN, <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />),
    folder: stroked(
      OWN,
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />,
    ),
    info: stroked(
      OWN,
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M12 11v6M12 7.5v.5" />
      </>,
    ),
    lastPage: stroked(OWN, <path d="M7 6l6 6-6 6M15 6l6 6-6 6" />),
    menu: stroked(OWN, <path d="M4 7h16M4 12h16M4 17h16" />),
    move: stroked(
      OWN,
      <path d="M12 3v18M3 12h18M12 3 9 6M12 3l3 3M12 21l-3-3M12 21l3-3M3 12l3-3M3 12l3 3M21 12l-3-3M21 12l-3 3" />,
    ),
    newVersion: stroked(
      OWN,
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M12 16V8M8.5 11.5 12 8l3.5 3.5" />
      </>,
      1.6,
    ),
    nextPage: chevronRight,
    outOfValidity: stroked(
      OWN,
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M12 7v5l3 2" />
      </>,
      1.8,
    ),
    previousPage: stroked(OWN, <path d="M15 6l-6 6 6 6" />),
    revoked: stroked(
      OWN,
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M5.6 18.4 18.4 5.6" />
      </>,
      1.8,
    ),
    search: stroked(
      OWN,
      <>
        <circle cx="11" cy="11" r="6" />
        <path d="M20 20l-4.5-4.5" />
      </>,
      1.8,
    ),
    selected: check,
    signed: stroked(OWN, <path d="M5 12.5l4.5 4.5L19 7.5" />, 2),
    signedByYou: stroked(
      OWN,
      <>
        <circle cx="12" cy="8" r="4" />
        <path d="M5 20c0-3.9 3.1-7 7-7s7 3.1 7 7" />
      </>,
    ),
    upToDate: stroked(
      OWN,
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M8 12.5 11 15.5 16.5 9" />
      </>,
      1.6,
    ),
    zoomIn: plus,
    zoomOut: stroked(OWN, <path d="M5 12h14" />),
  },
};
