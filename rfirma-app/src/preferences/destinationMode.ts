//! Dónde cae el siguiente firmado: junto al original o en la carpeta de destino (ADR-0011).
/**
 * Las etiquetas son las de `documents::domain::destination::DestinationMode`
 * del backend, que es como se persiste.
 */
export type DestinationMode = "next_to_the_original" | "in_the_destination_folder";
