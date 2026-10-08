//! La familia de origen y la licencia de un icono: Heroicons, con su MIT, o el dibujado para rFirma.

export interface IconOrigin {
  family: string;
  license: string;
}

export const HEROICONS: IconOrigin = { family: "Heroicons", license: "MIT" };
export const OWN: IconOrigin = { family: "rFirma", license: "EUPL-1.2" };
