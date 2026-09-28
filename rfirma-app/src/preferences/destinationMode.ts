/**
 * Dónde cae el siguiente firmado: junto al original o en la carpeta de
 * destino (ADR-0011). Las etiquetas son las de `documents::domain::destination::DestinationMode`
 * del backend, que es como se persiste.
 */
export const DESTINATION_MODES = ["next_to_the_original", "in_the_destination_folder"] as const;

export type DestinationMode = (typeof DESTINATION_MODES)[number];
