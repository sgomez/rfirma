//! Las secciones de certificado y de rúbrica con espías que comparten las historias y las pruebas del panel de firma.

import { fn } from "storybook/test";
import type { CertificateSection } from "./certificate";
import type { RubricSection } from "./rubric";

/** La sección del certificado con espías en sus dos acciones, salvo las que se pasen. */
export function aCertificateSection(
  state: CertificateSection["state"],
  actions: Partial<Omit<CertificateSection, "state">> = {},
): CertificateSection {
  return { state, choose: fn(), lookAgain: fn(), ...actions };
}

/** La sección de la rúbrica con espía al elegirla, salvo lo que se pase. */
export function aRubricSection(overrides: Partial<RubricSection> = {}): RubricSection {
  return { value: null, failure: null, choose: fn(), ...overrides };
}
