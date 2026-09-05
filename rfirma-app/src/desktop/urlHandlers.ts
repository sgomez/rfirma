/**
 * **Quién atiende los enlaces `afirma://`**: el puerto que lo pregunta y lo
 * elige, y su doble (ID-238, ID-239, ID-240).
 *
 * Debajo son `url_handlers` y `choose_url_handler`, que leen lo que el
 * escritorio diga que hay registrado y escriben un `default` explícito en el
 * `mimeapps.list` de la persona. La ventana no sabe nada de eso: ni del
 * fichero, ni de GIO, ni de en qué canal corre.
 *
 * **Ningún nombre de aplicación se escribe aquí** (ID-238). Ni «AutoFirma» ni
 * «rFirma»: lo que se enseña son los nombres que dio el escritorio, y el
 * lanzador propio llega en la misma respuesta ([`UrlHandlers.ours`]) para poder
 * saber si rFirma ya está elegida sin cablearlo.
 *
 * **Dentro del flatpak no hay desplegable ni banner** (ID-240): `available` es
 * `false` y lo único que queda es la frase fija que remite a los ajustes del
 * escritorio. Se dice, en vez de fingir que se puede.
 */

/** Un manejador registrado: lo que se lee y lo que se escribe. */
export interface UrlHandler {
  /** El fichero `.desktop`, que es lo que va al `mimeapps.list`. */
  id: string;
  /** El nombre visible, tal y como lo dio el escritorio. */
  name: string;
}

/** Lo que se puede saber de quién atiende los enlaces `afirma://`. */
export interface UrlHandlers {
  /**
   * Si el escritorio puede contestar. `false` es el flatpak, y entonces
   * `handlers` no está vacía: **es que no existe**.
   */
  available: boolean;
  /** Lo registrado, tal cual lo dio el escritorio. */
  handlers: readonly UrlHandler[];
  /** Quién está apuntado hoy, o `null` si no lo está nadie. */
  current: string | null;
  /** El fichero `.desktop` de rFirma. */
  ours: string;
}

/** Quien sabe quién atiende `afirma://` y sabe cambiarlo. Ver [`UrlHandlers`]. */
export interface UrlHandlerChoice {
  /** Lo que se puede saber ahora mismo. */
  who(): Promise<UrlHandlers>;
  /**
   * Deja apuntado que `handler` —un fichero `.desktop` de los que vinieron en
   * `handlers`— atiende los enlaces. Rechaza con la situación del ID-29 si el
   * `mimeapps.list` no se deja leer o escribir.
   */
  choose(handler: string): Promise<void>;
}

/** Si quien atiende los enlaces ya es rFirma. */
export function weAlreadyHandleThem(who: UrlHandlers): boolean {
  return who.current === who.ours;
}

/**
 * Si el banner del arranque tiene algo que preguntar (ID-239).
 *
 * Tres condiciones, y las tres son la misma pregunta: que se pueda cambiar
 * —fuera del flatpak—, que no lo atienda ya rFirma, y que no se haya pedido
 * dejar de preguntar. Es puro para poder fijarlo sin montar la ventana.
 */
export function theBannerHasSomethingToAsk(who: UrlHandlers | null, asking: boolean): boolean {
  if (who === null || !asking || !who.available) return false;
  return !weAlreadyHandleThem(who);
}

/**
 * El escritorio sin escritorio: contesta lo que se le diga y recuerda lo
 * elegido. Es el doble de las pruebas; quien pregunta de verdad es
 * `tauriUrlHandlers`.
 */
export function inMemoryUrlHandlers(
  initial: UrlHandlers = {
    available: true,
    handlers: [{ id: "rfirma.desktop", name: "rFirma" }],
    current: null,
    ours: "rfirma.desktop",
  },
  onChoose: (handler: string) => void = () => {},
): UrlHandlerChoice {
  let who = initial;
  return {
    who: async () => who,
    choose: async (handler) => {
      who = { ...who, current: handler };
      onChoose(handler);
    },
  };
}
