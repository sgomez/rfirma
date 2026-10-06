//! Los documentos y recientes de ejemplo que comparten las historias de la ventana principal.

import type { DocumentInHand } from "../document";
import type { RecentDocument } from "../recents";

const NOW = Math.floor(Date.now() / 1000);

function documentNamed(name: string, badge: DocumentInHand["badge"] = "Unsigned"): DocumentInHand {
  return { id: name, name, badge, modified: null, placement: null, remembered: true };
}

export const storyTabs: readonly DocumentInHand[] = [
  documentNamed("Solicitud de subvención.pdf"),
  documentNamed("Contrato de alquiler-firmado.pdf", "Signed"),
  documentNamed("Memoria técnica.pdf"),
];

export const manyStoryTabs: readonly DocumentInHand[] = Array.from({ length: 12 }, (_, index) =>
  documentNamed(`Expediente ${index + 1} de la convocatoria.pdf`),
);

function recentNamed(name: string, extra: Partial<RecentDocument> = {}): RecentDocument {
  return {
    id: name,
    name,
    folder: "Documentos",
    location: "~/Documentos",
    badge: "Unsigned",
    modified: null,
    lastUsed: NOW,
    available: true,
    placement: null,
    ...extra,
  };
}

export const storyRecents: readonly RecentDocument[] = [
  recentNamed("Solicitud de subvención.pdf"),
  recentNamed("Contrato de alquiler-firmado.pdf", { badge: "Signed", lastUsed: NOW - 86_400 }),
  recentNamed("Memoria técnica.pdf", { lastUsed: NOW - 10 * 86_400 }),
  recentNamed("Factura de marzo.pdf", { available: false, lastUsed: NOW - 40 * 86_400 }),
];
