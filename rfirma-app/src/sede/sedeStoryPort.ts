//! El doble de `SiteErrandPort` de las historias de sede: emite el momento pedido y sus órdenes son espías de Storybook.

import { fn } from "storybook/test";
import { type ErrandStage, noErrand, type SiteErrandPort } from "./errand";

/** Un puerto que emite `stage` de sede y no hace nada más. */
export function storyErrand(stage: ErrandStage): SiteErrandPort {
  return {
    ...noErrand(),
    watch: (onChange) => {
      onChange({ origin: "sede.ejemplo.gob.es", operation: "sign", stage });
      return () => {};
    },
    consent: fn(async () => {}),
    cancel: fn(async () => {}),
    close: fn(async () => {}),
    installLocalCa: fn(async () => {}),
  };
}
