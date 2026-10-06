//! Los argumentos que comparten las historias de `SedeView`: un trámite de ejemplo y órdenes que son espías de Storybook.

import { fn } from "storybook/test";
import type { Errand, ErrandStage } from "../../errand";
import type { SedeViewProps } from "../../SedeView";

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

/** Una historia de momento cuyo trámite entero viaja en sus parámetros, para montarlo en un test de comportamiento. */
export function momentStory<Args>(args: Args, errand: Errand) {
  return { args, parameters: { errand } };
}

/** Lo que un momento recibe del trámite además de su etapa: el origen, la orden terminal y la operación. */
export function momentProps({ origin, terminalOrder, operation }: Errand) {
  return { origin, terminalOrder: terminalOrder ?? null, operation };
}
