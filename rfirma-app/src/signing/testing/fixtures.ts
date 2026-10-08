//! Las secciones de certificado, rúbrica, firma visible, destino y firmar con espías que comparten las historias y las pruebas del panel de firma.

import { fn } from "storybook/test";
import { type CertificateSection, NO_READER } from "../certificate";
import type { Destination, DestinationSection } from "../destination";
import type { SigningSection } from "../flow";
import type { RubricSection } from "../rubric";
import type { VisibleSignature, VisibleSignatureSection } from "../visibleSignature";

/** La sección del certificado con espías en sus dos acciones, salvo las que se pasen. */
export function aCertificateSection(
  state: CertificateSection["state"],
  actions: Partial<Omit<CertificateSection, "state">> = {},
): CertificateSection {
  return {
    state,
    reader: NO_READER,
    installFailure: null,
    install: fn(),
    choose: fn(),
    lookAgain: fn(),
    ...actions,
  };
}

/** La sección de la rúbrica con espía al elegirla, salvo lo que se pase. */
export function aRubricSection(overrides: Partial<RubricSection> = {}): RubricSection {
  return { value: null, failure: null, choose: fn(), ...overrides };
}

/** La sección de la firma visible con espía al cambiarla. */
export function aVisibleSignatureSection(
  value: VisibleSignature,
  overrides: Partial<Omit<VisibleSignatureSection, "value">> = {},
): VisibleSignatureSection {
  return { value, change: fn(), ...overrides };
}

/** La sección del destino con espía al elegir otro. */
export function aDestinationSection(
  value: Destination,
  overrides: Partial<Omit<DestinationSection, "value">> = {},
): DestinationSection {
  return { value, chooseSingle: fn(), ...overrides };
}

/** La sección de firmar, parada y con espías, salvo lo que se pase. */
export function aSigningSection(overrides: Partial<SigningSection> = {}): SigningSection {
  return { running: false, sign: fn(), back: fn(), ...overrides };
}
