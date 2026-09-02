/**
 * El conjunto de páginas **tecleado**, y su camino de vuelta a texto.
 *
 * Formato de impresión de toda la vida: `1,2-3,10-20`. Números y rangos
 * separados por comas, sin sintaxis propia. Lo que el mapa prohíbe copiar de
 * AutoFirma es **su sintaxis** —`1-3,-3--1`, con rangos negativos— y **su
 * degradación silenciosa**, no el hecho de teclear (ID-98).
 *
 * Por eso aquí **nada se recorta ni se ignora** (ID-22): cada entrada que no se
 * puede resolver se cuenta como una situación —no como un mensaje— y quien la
 * reciba apaga el botón de firmar. Un conjunto a medias que se firmara sin
 * avisar sería peor que no poder firmar.
 *
 * Es una función pura y se prueba aparte del componente (TD-29): es la pieza
 * con más reglas y menos superficie visible del hito.
 */

import { type PageSet, pageSetOf, sealedPages } from "../viewer/signatureBox";

/**
 * Por qué no se puede resolver lo tecleado. Es una **situación**, con los datos
 * que su frase necesita, y no una cadena ya redactada: quien la enseña es la
 * vista, que sabe en qué idioma está (ID-29).
 */
export type PageRangeError =
  /** Se ha nombrado una página que el documento no tiene: `99` en uno de 27. */
  | { kind: "beyond"; page: number; pageCount: number }
  /** El rango va al revés: `3-1`. */
  | { kind: "reversed"; entry: string }
  /** No hay página 0: la primera es la 1. */
  | { kind: "zero" }
  /** No es un número ni un rango: `1;2;3`, `a`, `1--2`. */
  | { kind: "malformed"; entry: string };

/**
 * Lo tecleado, ya resuelto.
 *
 * `pages: null` **no es un fallo**: es el campo vacío, y desemboca en el mismo
 * sitio que el PDF recién abierto —sin colocación— porque «colocado es tener
 * páginas» (ID-92).
 */
export type PageRangeResult =
  | { ok: true; pages: PageSet | null }
  | { ok: false; error: PageRangeError };

/** Un número suelto (`10`) o un rango cerrado (`10-20`). Nada más. */
const ENTRY = /^(\d+)(?:-(\d+))?$/;

/**
 * Lo tecleado → el conjunto, o la primera situación que lo impide.
 *
 * Se para en la **primera** entrada que no se resuelve, leyendo de izquierda a
 * derecha: quien escribe corrige una cosa cada vez, y una lista de cuatro
 * quejas bajo el campo no se lee.
 */
export function parsePageRange(text: string, pageCount: number): PageRangeResult {
  const trimmed = text.trim();
  if (trimmed === "") return { ok: true, pages: null };

  const pages: number[] = [];
  for (const raw of trimmed.split(",")) {
    const entry = raw.trim();
    const matched = ENTRY.exec(entry);
    if (matched === null) return { ok: false, error: { kind: "malformed", entry } };

    const from = Number(matched[1]);
    const to = matched[2] === undefined ? from : Number(matched[2]);
    if (from === 0 || to === 0) return { ok: false, error: { kind: "zero" } };
    if (to < from) return { ok: false, error: { kind: "reversed", entry } };
    // Se denuncia el número más alto del rango, que es el que se ha escrito de
    // más: con `10-40` en un documento de 27, la queja habla de la 40.
    if (to > pageCount) return { ok: false, error: { kind: "beyond", page: to, pageCount } };

    for (let page = from; page <= to; page += 1) pages.push(page);
  }

  return { ok: true, pages: pageSetOf(pages) };
}

/**
 * El conjunto → lo tecleado, en **forma comprimida**.
 *
 * Es el camino de vuelta del ID-99: sellar o quitar una página desde el visor
 * reescribe el campo, y así los dos caminos —teclear y pulsar— no pueden
 * discrepar. Quitar la 12 de `3,10-20` deja `3,10-11,13-20`.
 */
export function formatPageRange(pages: PageSet, pageCount: number): string {
  const list = sealedPages(pages, pageCount);
  const runs: string[] = [];
  let index = 0;
  while (index < list.length) {
    const from = list[index] as number;
    let to = from;
    while (index + 1 < list.length && list[index + 1] === to + 1) {
      index += 1;
      to = list[index] as number;
    }
    runs.push(from === to ? `${from}` : `${from}-${to}`);
    index += 1;
  }
  return runs.join(",");
}
