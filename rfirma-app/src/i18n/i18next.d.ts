import type Resources from "./resources";

/**
 * Las claves del catálogo, para `tsc` y para el editor (ID-127).
 *
 * `resources.d.ts` lo genera `i18next-cli types` desde la instantánea del
 * castellano; **este fichero no**. `i18next-cli` lo escribe una sola vez y
 * deriva `defaultNS` del nombre del recurso —`'es'`—, que no es el espacio de
 * nombres del programa: el nuestro es `NAMESPACE` en `i18n.ts`, y la propia
 * herramienta avisa de que hay que ajustarlo. Por eso se versiona a mano.
 *
 * Con esto, una clave que no existe sale en rojo en el editor y en `tsc`, antes
 * de llegar a `i18next-cli extract --ci`. **Ojo al escribir aquí**: el
 * extractor lee también los comentarios, así que un ejemplo de `t(...)` con una
 * clave inventada dentro de un bloque como este pondría el CI en rojo.
 */
declare module "i18next" {
  interface CustomTypeOptions {
    enableSelector: false;
    defaultNS: "translation";
    resources: { translation: Resources["es"] };
  }
}
