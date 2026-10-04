//! Los argumentos que comparten las historias de `SedeView`: un trámite de ejemplo y órdenes que son espías de Storybook.

import { fn } from "storybook/test";
import type { Errand, ErrandStage } from "../../src/sede/errand";
import type { SedeViewProps } from "../../src/sede/SedeView";

/** Las órdenes de la vista, todas espías: una historia no habla con ningún puerto. */
export const sedeViewActions = {
  onConsent: fn(),
  onConfirmSignatures: fn(),
  onMarkArea: fn(),
  onCancel: fn(),
  onClose: fn(),
  onLookAgain: fn(),
  onInstallCertificate: fn(),
  onInstallLocalCa: fn(),
  onDismissWarning: fn(),
  onOpenHelp: fn(),
} satisfies Partial<SedeViewProps>;

/** Un trámite de una sede de ejemplo en el momento `stage`; `errand` sobrescribe el resto. */
export function sedeErrand(stage: ErrandStage, errand: Partial<Errand> = {}): Errand {
  return { origin: "sede.ejemplo.gob.es", operation: "sign", stage, ...errand };
}
