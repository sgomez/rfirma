/**
 * El vocabulario de la bandeja, en el lado de la interfaz.
 *
 * Los tres valores son los mismos de `memory::recents` en el backend, y con
 * los mismos nombres: `Badge` es lo que se **guarda** —se conoce abriendo el
 * documento, y por eso se cachea— y `ShownBadge` es lo que se **pinta**, que
 * es la guardada más `Unavailable`, un hecho sobre el disco de ahora mismo que
 * no se persiste nunca. Si cambia un valor allí, cambia aquí.
 */
export type Badge = "Signed" | "Unsigned";

/** La insignia que se pinta en la fila. Ver [`Badge`]. */
export type ShownBadge = Badge | "Unavailable";
