# Los mecanismos del catálogo, en un solo sitio

`po/messages.pot` es una **lista plana de entradas sin comentario de grupo**:
sus `#.` son para quien traduce, no para quien mantiene el arnés (ID-133). Lo
que explica *cómo funciona* el circuito —`messages.pot` → `msgmerge` →
`po/*.po` → `po-import.mjs` → `src/i18n/locales/*.ts`— vive en el propio
código, como comentario de bloque encima de cada pieza. Este fichero **no
repite** ese texto: es el índice de los **dieciséis bloques**, con su
fichero y línea, para no tener que abrir los seis ficheros uno a uno para
encontrar el que explica una decisión concreta (ID-134).

Si tocas el mecanismo, el comentario de bloque es la fuente de verdad y este
índice puede quedarse desactualizado en la línea: arréglalo en la misma PR.

## `tools/po-import.mjs`

| Bloque | Qué decide |
|---|---|
| L1-26 (cabecera del fichero) | El circuito completo en un diagrama, y las tres reglas que no son detalle de implementación: Node puro sin `gettext` para construir, un idioma incompleto no genera `.ts`, y la exportación tiene que ser `export default` (`i18next-cli` la necesita con nombre por defecto). |
| L36-49 (`PLURAL_SUFFIXES`) | Los sufijos `_one`/`_many`/`_other` son los del castellano y se escriben **en los cinco idiomas por igual**, aunque el inglés no llegue a usar `_many`: la alternativa —un juego de sufijos por idioma— rompería `Catalog = typeof es`. |
| L54-62 (`entriesOf`) | Un `msgstr` vacío no se omite, se devuelve `""`: la clave tiene que estar presente para que `returnEmptyString: false` la haga caer al castellano. Un `#, fuzzy` sí se vacía: cuenta como no traducido. |
| L86-91 (`nest`) | El orden de las claves en el catálogo generado es el del `.po` —a su vez el de la plantilla—, porque `i18n.test.tsx` compara ese orden entre los cinco idiomas. |
| L123-129 (`renderCatalog`) | El castellano se escribe **sin anotar** `: Catalog`: de él sale el tipo, así que anotarlo se mordería la cola. Los demás sí se anotan, y es `tsc` quien comprueba que no falta ni sobra una clave. |
| L139-152 (`renderSnapshot`) | La instantánea que mira `i18next-cli extract --ci` vive en `node_modules/.cache/`, nunca en `src/`: si mirase los catálogos de verdad, se llevaría por delante el `: Catalog` y el `as const` de la instantánea haría literales las hojas. El formato se replica al byte porque `--ci` sale en rojo ante cualquier diferencia, orden incluido. |
| L172-179 (`renderIndex`) | `LANGUAGES` y `CATALOGS` **se derivan de qué ficheros `.ts` existen**: un idioma a medias no tiene fichero, así que no puede aparecer en la lista. |
| L208-213 (`generate`, `--all`) | `--all` rellena los idiomas incompletos con castellano para que quien traduce vea su trabajo antes del 100 %. Nunca en el CI ni en la publicación. |
| L227-230 (purga de generados) | El estado de `locales/` y de la instantánea es función **solo** de `po/`: un `.ts` de una ejecución anterior no puede sobrevivir a que su idioma deje de estar al 100 %, aunque `index.ts` ya no lo importe. |

## `catalog.ts`

| Bloque | Qué decide |
|---|---|
| L3-11 (`Catalog`) | Sin `as const`: las hojas son `string`, no literales, así que declarar un catálogo como `Catalog` obliga a tener exactamente las mismas claves y eso lo comprueba `tsc`. `locales/es.ts` no está versionado: si el editor dice que no existe, es `just po` lo que falta ejecutar. |

## `i18n.ts`

| Bloque | Qué decide |
|---|---|
| L10-24 (`createI18n`) | Dos decisiones que no son las de por omisión: sin `i18next-browser-languagedetector` (el idioma es una preferencia guardada, no se olfatea, ID-02), y `returnEmptyString: false` (con el valor por omisión la interfaz saldría en blanco en vez de en español). |

## `languages.ts`

| Bloque | Qué decide |
|---|---|
| L4-19 (cabecera del módulo) | Los idiomas son cinco y ni la lista ni los catálogos se escriben aquí: salen de `locales/index.ts`. El valencià salió en v0.3 porque `Intl.PluralRules("va")` no da la categoría `many` que `es` y `ca` sí usan. Las etiquetas son las de `Language::tag` del backend (`signing/language.rs`): cambia una, cambian las dos. |

## `LanguageProvider.tsx`

| Bloque | Qué decide |
|---|---|
| L10-17 (`setLanguage`, en la interfaz `LanguageSelection`) | Si el disco rechaza el guardado, deshace el cambio de idioma y relanza: mismo contrato que `Preferences.save` vía `App.changeSettings`, para que el aviso de «se ha vuelto al valor anterior» sea siempre cierto. |
| L30-43 (comentario sobre `LanguageProvider`) | El cambio se aplica sin reiniciar y **antes** de guardar la preferencia: la interfaz responde al momento, sin «Guardar» ni «Cancelar», igual que el resto de Preferencias. |

## `preference.ts`

| Bloque | Qué decide |
|---|---|
| L3-14 (`LanguagePreference`) | Es un puerto y no una llamada a Tauri directa porque la ventana no conoce a Tauri: quien guarda de verdad es el backend (`memory::Configuration`, ID-31). El idioma va por su propio puerto, fuera de `Preferences`, porque se lee **antes** de que haya ventana —`createI18n` lo necesita para el primer pintado—. |

## `i18next.d.ts`

| Bloque | Qué decide |
|---|---|
| L3-16 (`declare module "i18next"`) | Se versiona a mano porque `i18next-cli types` derivaría `defaultNS` del nombre del recurso (`'es'`), que no es `NAMESPACE`. El extractor **lee también los comentarios** de este fichero: un ejemplo de `t(...)` con una clave inventada dentro de un bloque como este pone el CI en rojo. |
