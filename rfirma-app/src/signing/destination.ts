/**
 * **Dónde va a caer el documento firmado**, en el lado de la interfaz: la
 * carpeta y el nombre, los dos por su nombre y ninguno por su ruta (ID-63,
 * ADR-0011).
 *
 * Aquí viven las dos mitades del componente **ruta de destino** del sistema de
 * diseño: el puerto que pregunta al backend dónde caerá lo que hay delante, y
 * la **función pura** que decide cómo se recorta la línea cuando no cabe. El
 * recorte está aquí y no dentro del pie del panel porque es la pieza con más
 * reglas y menos superficie visible de todo el destino (TD-13): probarlo por la
 * pantalla obligaría a medir píxeles para comprobar una regla sobre cadenas.
 */

/** Dónde va a caer el documento que hay delante. Lo compone el backend. */
export interface Destination {
  /** El **nombre** de la carpeta, su último segmento. Nunca su ruta. */
  folder: string;
  /**
   * El nombre del fichero firmado, con su sufijo `-firmado` y su número de
   * desempate ya resueltos. `null` cuando la carpeta no está o no se deja
   * escribir: sin carpeta comprobada no hay homónimo que resolver, y aventurar
   * un nombre sería prometer un fichero que nadie va a escribir.
   */
  name: string | null;
  /**
   * Si la carpeta está y se puede escribir **ahora mismo**. Sale de
   * `CheckedFolder::check` y no de un literal (ID-67).
   */
  writable: boolean;
}

/**
 * Quién sabe dónde caerá el documento.
 *
 * Puerto, y no una llamada a Tauri, por la regla de siempre: la ventana no
 * conoce a Tauri y quien elige la implementación es `main.tsx` (ADR-0017).
 * Debajo es la orden `preview_destination`, que mira el disco —la carpeta y sus
 * homónimos— sin escribir nada y **sin crear la carpeta** (ID-38).
 */
export interface DestinationSource {
  /** Dónde caerá el documento abierto con ese identificador. */
  previewFor(documentId: string): Promise<Destination>;
}

/**
 * Quién lleva al usuario **hasta** el fichero que ha quedado escrito.
 *
 * Bajo el sandbox esto no es comodidad: la aplicación nunca conoce la ruta del
 * documento y el usuario nunca la ve (ADR-0011), así que abrir el PDF y abrir
 * su carpeta son las dos únicas formas de llegar a lo que se acaba de firmar
 * (ID-79).
 *
 * Ninguno de los dos métodos recibe nada: el documento que se abre es el de la
 * última firma, y quién es lo sabe el backend, que es el único que tiene su
 * ruta. Debajo son las órdenes `open_signed_document` y `open_signed_folder`,
 * y más abajo el portal `OpenURI`.
 */
export interface SignedDocumentOpener {
  /** Abre el PDF firmado con el visor del sistema. */
  openDocument(): Promise<void>;
  /** Abre la carpeta donde quedó, con las firmas anteriores dentro (ID-81). */
  openFolder(): Promise<void>;
}

/**
 * Un abridor que **no abre**, y lo dice: para montar la ventana en una prueba
 * sin backend. Falla en vez de fingir que ha abierto algo, que es lo único que
 * no puede hacer aquí un doble —el usuario se quedaría esperando una ventana
 * que nadie va a abrir—.
 */
export function unavailableOpener(): SignedDocumentOpener {
  const missing = () => Promise.reject(new Error("no hay quien abra el documento firmado"));
  return { openDocument: missing, openFolder: missing };
}

/** Un destino fijo, para pintar la ventana en una prueba sin backend. */
export function inMemoryDestination(destination: Destination): DestinationSource {
  return { previewFor: async () => destination };
}

/**
 * Cuántos caracteres caben en el nombre y en la carpeta antes de recortar.
 *
 * Son un presupuesto de caracteres y no una medida de píxeles a propósito: la
 * línea **envuelve** (`overflow-wrap: anywhere`), así que el recorte no está
 * para que quepa en un renglón —eso ya lo resuelve el salto— sino para que un
 * nombre desmedido no se coma el pie entero. Con la columna de 360 px del panel
 * dos renglones son ~40 caracteres cada uno.
 */
export const NAME_BUDGET = 40;
export const FOLDER_BUDGET = 24;

/** El carácter que se come lo recortado. Uno solo, no tres puntos. */
const ELLIPSIS = "…";

/**
 * El destino recortado para caber, **conservando lo que se mira**.
 *
 * Tres reglas, y son el componente entero (ID-64):
 *
 * 1. El nombre se recorta **por el medio**: se conservan siempre la extensión y
 *    el sufijo `-firmado` con su número de desempate —`-2`, `-3`—, porque son la
 *    respuesta a «¿voy a machacar el anterior?», que es justo lo que se mira. El
 *    `…` se come el centro del tronco.
 * 2. La carpeta **no se recorta nunca por el medio**; si hace falta, por la
 *    cola: un nombre de carpeta se reconoce por el principio y no tiene ninguna
 *    cola que preservar.
 * 3. Lo que ya cabe **no se toca**.
 */
export function shortenDestination(
  destination: { folder: string; name: string },
  budget: { name?: number; folder?: number } = {},
): { folder: string; name: string } {
  return {
    folder: shortenFolder(destination.folder, budget.folder ?? FOLDER_BUDGET),
    name: shortenName(destination.name, budget.name ?? NAME_BUDGET),
  };
}

/** La carpeta, recortada por la cola. */
function shortenFolder(folder: string, budget: number): string {
  const characters = [...folder];
  if (characters.length <= budget) {
    return folder;
  }
  return characters.slice(0, Math.max(budget - 1, 0)).join("") + ELLIPSIS;
}

/** El nombre, recortado por el medio con su cola intacta. */
function shortenName(name: string, budget: number): string {
  const characters = [...name];
  if (characters.length <= budget) {
    return name;
  }
  const tail = preservedTail(name);
  const trunk = [...name.slice(0, name.length - tail.length)];
  // Lo que queda para el tronco, una vez descontados la cola —que no se
  // negocia— y el propio `…`.
  const room = budget - [...tail].length - 1;
  if (room < 1) {
    return ELLIPSIS + tail;
  }
  return trunk.slice(0, room).join("") + ELLIPSIS + tail;
}

/**
 * La cola que sobrevive al recorte: la extensión y, si está, el sufijo
 * `-firmado` con su número.
 *
 * Se reconoce sobre el nombre ya compuesto por el backend
 * (`destination::naming`), así que basta con leerlo: aquí no se decide cómo se
 * llama nada.
 */
function preservedTail(name: string): string {
  const dot = name.lastIndexOf(".");
  const extension = dot > 0 ? name.slice(dot) : "";
  const stem = dot > 0 ? name.slice(0, dot) : name;
  const suffix = /-firmado(-\d+)?$/i.exec(stem);
  return (suffix?.[0] ?? "") + extension;
}
