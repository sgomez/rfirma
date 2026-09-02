# i18next y el `.po`: qué sobrevive al viaje y qué se pierde en silencio

Sondeo del [#163](https://github.com/sgomez/rfirma/issues/163), hijo del mapa
[#148](https://github.com/sgomez/rfirma/issues/148). Prepara la decisión de
[#162](https://github.com/sgomez/rfirma/issues/162) —cómo se monta el
intercambio entre nuestros catálogos y el `.po`— después de que el mapa
descartara Lingui y Fluent y dejara el montaje donde está: **i18next 26.4.0**,
**react-i18next 17.0.12** y los catálogos como objetos de TypeScript en
`rfirma-app/src/i18n/locales/`, con `Catalog = typeof es` en `catalog.ts`
([ADR-0009](../adr/0009-catalogo-de-cadenas-propio-y-seis-idiomas.md)).

**Respuestas cortas.** El conversor **existe y está vivo**: `i18next-conv`
17.0.0, publicado el **2026-06-19**, con el repositorio tocado el **2026-08-20**
y 156 970 descargas el último mes. Sobre nuestro catálogo real, **hoy** hace el
viaje de ida y vuelta **sin perder un byte**: los 205 mensajes de `es.ts` salen
a `.po` y vuelven con las mismas claves, en el mismo orden y con los mismos
valores.

Y aun así **no sirve**, por tres pérdidas medidas que no avisan de nada y
devuelven código de salida 0:

1. **Aplasta los plurales de tres formas.** Su tabla de plurales es una copia
   congelada de la que i18next tenía **antes** de la v21, donde el castellano y
   el catalán tienen dos formas. `Intl.PluralRules`, que es lo que i18next
   26.4.0 usa de verdad, les da **tres** (`one`, `many`, `other`). El resultado
   medido: un mensaje con `_one`/`_many`/`_other` sale al `.po` con **dos**
   `msgstr`, la forma `_one` **desaparece**, `_many` ocupa su hueco, y a la
   vuelta el singular castellano es el texto de los millones.
2. **Se come la forma sin contar de un contexto.** `friend_male` junto a
   `friend_male_one`/`_other` vuelve **sin** `friend_male`.
3. **No emite comentarios de traductor (`#.`) ni referencias (`#:`)**, ni tiene
   por dónde recibirlos: su entrada es JSON.

Los tres fallos aparecen justo donde la ficha 26 quiere llegar —plurales,
contexto y comentarios—, así que el conversor cubre exactamente el caso que ya
tenemos y ninguno de los que queremos.

**Escribirlo cuesta 153 líneas**, medidas: 94 de `es.ts` a `.po`, 41 de vuelta
y 18 del arnés. Está montado y probado en este sondeo, y el viaje de ida y
vuelta del catálogo real devuelve el objeto **idéntico**, con `tsc` aceptando el
`.ts` generado como `Catalog` **sin ningún truco** y siguiendo rojo si falta una
clave.

Un hallazgo que no iba buscando y que toca la decisión 7 del mapa: **el `msgid`
huérfano no lo caza nadie en este montaje**. Con i18next no hay extractor que
lea el código; el `.po` se genera desde el catálogo, así que una `t()` sin
entrada no baja el castellano del 100 % —simplemente no existe para el arnés—.
Y buscarla con `grep` tampoco vale: **89 de las 205 claves (el 43 %) no
aparecen literalmente en el código**, porque se alcanzan por los **10 sitios con
`t()` de clave dinámica** que hay repartidos por la aplicación.

---

## Cómo se midió

Un banco real, no lectura de documentación. Node **v24.15.0**, npm **11.12.1**,
GNU gettext-tools **0.23.2**. Fecha de todas las mediciones: **2026-09-02**.

Instalados en un proyecto de usar y tirar: `i18next-conv` **17.0.0** (la última
publicada), `i18next` **26.4.0** (la que usamos; la última es la 26.4.1, del
2026-09-01), `gettext-parser` **9.1.1** y `typescript` **7.0.2** (la del
repositorio).

Sobre ese banco se convirtió en los dos sentidos: un catálogo de juguete con
anidamiento, interpolación, contexto y plurales; **el `es.ts` real de rfirma**,
con sus 205 hojas; y catálogos a medio traducir, con entradas dudosas
(`fuzzy`) y con huérfanos. Lo que este informe da como cifra o comportamiento
está observado ahí, salvo donde diga lo contrario. Los prototipos de generador y
arnés se escribieron y se ejecutaron; el `.ts` que sale de ellos se metió en
`rfirma-app/` y se pasó por `pnpm exec tsc -b` de verdad.

En paralelo se leyó el código de `i18next` 26.4.0 (`PluralResolver`), el de
`gettext-converter` 1.3.4 (la tripa de `i18next-conv`) y los metadatos de npm y
GitHub.

---

## El veredicto

| Lo que hace falta | `i18next-conv` 17.0.0 | Escrito por nosotros |
| --- | --- | --- |
| Aplanar y desaplanar el catálogo anidado | Sí, separador configurable | Sí |
| Viaje de ida y vuelta del catálogo de hoy | **Idéntico** | **Idéntico** |
| Plurales de 3 formas (`es`, `ca`) | **No: pierde `_one` en silencio** | Sí, con `Intl.PluralRules` |
| Contexto sin `count` junto a su plural | **No: lo pierde en silencio** | Sí |
| Comentarios de traductor (`#.`) | **No** | Sí, desde el JSDoc |
| Referencias de código (`#:`) | No | Solo para 116 de 205 claves |
| Coste en `node-sources.json` | +12 entradas | +5, o **+0** |

**Recomendación: escribirlo.** No porque el conversor esté abandonado —no lo
está—, sino porque su modelo de plurales es el de i18next v20 y el nuestro es el
de la v26, y esa distancia se paga en cadenas corrompidas sin aviso. Las 153
líneas del prototipo son menos código del que costaría envolver `i18next-conv`
para taparle los tres agujeros.

---

## 1. El conversor: vivo, y qué hace exactamente

`i18next-conv` es el paquete npm del repositorio
[i18next/i18next-gettext-converter](https://github.com/i18next/i18next-gettext-converter).
Está vivo sin discusión:

| | |
| --- | --- |
| Última versión | **17.0.0**, del **2026-06-19** |
| Anteriores | 16.0.0 (2025-11-01), 15.1.2 (2025-09-29), 15.0.0 (2024-05-01) |
| Repositorio | último `push` el **2026-08-20**, 203 estrellas, 7 incidencias abiertas, sin archivar |
| Descargas | **156 970** el último mes |
| `engines` | `node: ^22.13.0 \|\| >=24` |
| Árbol de dependencias | **12 paquetes** (`@postalsys/gettext`, `colorette`, `commander`, `content-type`, `encoding`, `gettext-converter`, `gettext-parser`, `iconv-lite`, `p-from-callback`, `safer-buffer`, `arrify` y él mismo) |

La conversión de verdad la hace **`gettext-converter` 1.3.4**, una dependencia
suya; `i18next-conv` es la línea de órdenes.

### El aplanado, y por qué el convenio importa

Nuestros catálogos son objetos anidados de hasta **4 niveles**; el `.po` es
plano. El conversor aplana con un separador que por omisión es **`##`**, de modo
que `panel.document.pages.one` sale como `panel##document##pages##one`. Con
`-k .` sale tal cual:

```
msgid "panel.document.pages.one"
msgstr "1 página"
```

Y esto **no es cosmética**: con `-k .` el `msgid` **es** la clave que el código
escribe en `t("panel.document.pages.one")`, así que quien traduce puede
encontrar el sitio con un `grep` sin que el fichero lleve referencias. Se midió
que el viaje de ida y vuelta con `-k .` devuelve el catálogo idéntico.

### El viaje de ida y vuelta del catálogo real: sale bien

`es.ts` → JSON → `.po` → JSON, con `--compatibilityJSON v4 -k . --noDate`:

```
ida: 205   vuelta: 205
valores que no vuelven iguales: 0
claves nuevas: 0
mismo orden de claves: true
JSON byte a byte: true
```

0,26 s. El `.po` ocupa 18 802 bytes frente a los 14 351 del JSON. `--noDate`
hace falta: sin él, cada ejecución cambia `POT-Creation-Date` y el fichero sale
sucio en cada `git diff`.

Que salga bien tiene una explicación aburrida: **hoy el catálogo no tiene nada
de lo que se pierde**. Cero claves con `_`, cero plurales de i18next, cero
contextos, cero comentarios por cadena. El conversor no falla porque no hay
nada que romper todavía.

---

## 2. Los plurales: la pérdida real, y es de las gordas

Esto es lo que la ficha 26 compra, y es justo lo que el conversor no sabe hacer.

### Qué hace i18next 26.4.0

Usa **`Intl.PluralRules` y nada más**. En `PluralResolver.getRule` construye
`new Intl.PluralRules(cleanedCode, { type })` y de ahí saca los sufijos, que
ordena por `zero < one < two < few < many < other`. La opción
`compatibilityJSON`, que en su día permitía volver al formato v3, **ya no
existe** en la 26: no aparece en el código y `i18next.options.compatibilityJSON`
es `undefined`.

Las categorías reales de nuestros idiomas, medidas con Node 24:

| idioma | categorías | ejemplo que las separa |
| --- | --- | --- |
| `es` | **`one`, `many`, `other`** | 1 → `one`; 2 → `other`; **1 000 000 → `many`** |
| `ca` | **`one`, `many`, `other`** | 1 000 000 → `many` |
| `gl` | `one`, `other` | 1 000 000 → `other` |
| `eu` | `one`, `other` | 1 000 000 → `other` |
| `en` | `one`, `other` | 1 000 000 → `other` |

De paso queda confirmado lo que ya midió el sondeo hermano: `Intl.PluralRules("va")`
cae a `und` con **una sola categoría**, y `ca-ES-valencia` sí resuelve a `ca` con
las tres. Es el argumento técnico de la decisión 8 del mapa.

### Qué hace el conversor

Su tabla de plurales no es `Intl.PluralRules`: es un fichero,
`gettext-converter/lib/plurals.js`, cuya primera línea dice literalmente
`// copied from: https://github.com/i18next/i18next/blob/master/src/PluralResolver.js`.
Es la tabla que i18next tenía **antes** de pasarse a `Intl.PluralRules`, y en
ella el castellano y el catalán viven en el mismo grupo que el inglés, con
`nr: [1,2]` — **dos formas**.

De ahí sale la cabecera que escribe para el castellano:

```
"Plural-Forms: nplurals=2; plural=(n != 1)\n"
```

### La medición

Entrada, con las tres formas que i18next 26 espera del castellano:

```json
{ "pages_one": "ONE", "pages_many": "MANY", "pages_other": "OTHER" }
```

Salida:

```
msgid "pages"
msgid_plural "pages"
msgstr[0] "MANY"
msgstr[1] "OTHER"
```

Y la vuelta:

```json
{ "pages_one": "MANY", "pages_other": "OTHER" }
```

**`ONE` no está**. El singular del castellano se ha ido, su hueco lo ocupa la
forma de los millones y el fichero vuelve con dos formas donde había tres. El
conversor no dice nada, no escribe ningún aviso y sale con 0. Lo mismo, idéntico,
con `-l ca`.

Dos apuntes que acotan el daño y explican por qué esto pasa desapercibido:

* **Sin `--compatibilityJSON v4` es todavía más raro.** Por omisión el conversor
  ni siquiera reconoce los sufijos: `pages_one` y `pages_other` se convierten en
  dos entradas con **`msgctxt "one"` y `msgctxt "other"`**, no en un
  `msgid_plural`. La ayuda de la orden describe la opción como «genera ficheros
  compatibles con i18next@21», lo que invita a no ponerla; y resulta que es
  obligatoria, porque el formato v4 es el único que i18next 26 entiende.
* **Que falte `_many` no rompe la aplicación.** Medido: con solo `_one`/`_other`
  en el catálogo castellano, `t("pages", { count: 1000000 })` devuelve la forma
  `other`. Es gramática mala, no una pantalla rota. Lo grave del viaje de ida y
  vuelta no es la forma que falta: es la que **cambia de sitio**.

### En el `.po`, un plural de i18next no es un plural de gettext

Merece decirse aunque el conversor lo hiciera bien. Son dos mecanismos con
formas distintas:

* En gettext, `msgid`/`msgid_plural` son **dos cadenas fuente** (singular y
  plural en inglés) y la cabecera `Plural-Forms` lleva una **expresión en C**
  que decide el índice.
* En i18next, la selección la hace `Intl.PluralRules` en tiempo de ejecución y
  la clave solo lleva un sufijo.

En el prototipo de este sondeo eso se resuelve así: el `msgid_plural` se pone
igual al `msgid` (la clave; no hay cadena fuente que poner) y en `Plural-Forms`
solo se declara `nplurals` de verdad, porque la expresión C **no la va a
evaluar nadie**: quien reparte es i18next. La expresión sigue haciendo falta
para que `msgfmt` y las herramientas de traducción acepten el fichero, pero es
decoración. Conviene que el ADR o el spec lo diga, porque un traductor con
Poedit delante verá una regla que su editor usa para pintar pestañas y que el
programa ignora.

---

## 3. El contexto: se corresponde, hasta que se cruza con el plural

i18next separa el contexto con `_` (`friend_male`); gettext lo pone en
`msgctxt`. El conversor los casa, y en los dos sentidos: `friend_male` sale como
`msgctxt "male"` / `msgid "friend"` y vuelve igual. El separador se cambia con
`--ctxSeparator`.

Pero el separador de contexto y el de plural son **el mismo carácter**, y de ahí
salen dos problemas medidos.

**El primero: cualquier `_` en una clave se convierte en contexto.** Medido con
un catálogo de juguete: `open_menu` sale como `msgctxt "menu"` / `msgid "open"`,
y `preferences.language_es` como `msgctxt "es"`. Vuelve bien porque el convenio
es simétrico, pero el `.po` que ve quien traduce miente sobre la estructura.
Hoy da igual —**cero de las 205 claves de `es.ts` llevan `_`**— y por eso
conviene escribirlo como una regla del catálogo, no descubrirlo el día que
alguien escriba `pin_locked`.

**El segundo, y este sí destruye datos.** i18next distingue `friend_male` (la
forma que se usa cuando no se pasa `count`) de `friend_male_one` y
`friend_male_other`. Entrada:

```json
{ "friend": "Amistad", "friend_male": "Amigo", "friend_female": "Amiga",
  "friend_male_one": "Un amigo", "friend_male_other": "{{count}} amigos" }
```

Vuelta:

```json
{ "friend": "Amistad", "friend_male_one": "Un amigo",
  "friend_male_other": "{{count}} amigos", "friend_female": "Amiga" }
```

**`friend_male` ha desaparecido**, absorbido por el grupo plural de su mismo
contexto. Sin aviso y con salida 0.

Un consuelo: como nuestros `msgid` son **claves**, no texto fuente, no
necesitamos `msgctxt` para lo que gettext lo inventó —desambiguar dos «Cerrar»
distintos—, porque dos claves distintas ya son dos entradas distintas. El
`msgctxt` nos queda libre para el contexto de i18next, que es lo que el
conversor hace. La correspondencia es buena; lo que está mal es la
implementación cuando se cruza con el plural.

---

## 4. Los comentarios para quien traduce: el problema no es el mecanismo

`i18next-conv` **no emite `#.` ni `#:`**, y no puede: su entrada es JSON, que no
tiene dónde llevarlos. Medido en los dos sentidos: un `.po` con `#.`, `#:` y
`#, fuzzy` convertido a JSON y de vuelta a `.po` sale **sin los `#.` ni los
`#:`** (el `fuzzy` sí sobrevive, porque va en la entrada). La opción
`-K/--keyasareference` no ayuda: lo que hace es **usar el `#:` como clave**, y
sobre nuestro fichero produce entradas del tipo
`"src/components/App.tsx:42": "rFirma"`. Es para proyectos cuyo `msgid` es texto
inglés, no claves.

El ticket preguntaba cuál de las dos salidas es menos mala: leer el JSDoc del
árbol sintáctico, o cambiar la entrada del catálogo de cadena a objeto
`{ message, comment, context }`. **La segunda está descartada de entrada**: las
hojas del catálogo dejarían de ser `string`, y con ellas se cae
`Catalog = typeof es`, `UntranslatedCatalog` y todas las llamadas a `t()`. Es
rediseñar el catálogo para ganar un comentario.

La primera **funciona y está medida** (§6). Pero al ejecutarla sobre el `es.ts`
real aparece el dato que de verdad decide:

```
bloques de comentario vistos antes de una clave: 20
  ...colgando de una cadena (hoja): 2
  ...colgando de una sección (objeto): 17
```

De los 20 bloques de comentario de `es.ts`, **solo 2 cuelgan de una cadena**.
Los otros 17 cuelgan de una sección (`errors`, `languages`, `panel`, `pin`,
`preferences`…) y uno es la cabecera del fichero. Y gettext **no tiene dónde
poner el comentario de un grupo**: `#.` pertenece a un `msgid`.

O sea que el JSDoc de hoy **no es documentación por cadena para quien traduce**:
es prosa para quien programa, con identificadores de decisión y enlaces a
`docs/research/`, y va dirigida a las secciones. Sacarla al `.po` recupera dos
comentarios de veinte.

**La conclusión, con el catálogo real delante: la vía del JSDoc es la buena, y
lo que cuesta no es el mecanismo sino el texto.** Escribir comentarios de
traductor es escribir cosas nuevas —«aquí `{{count}}` son páginas, no firmas»—,
y eso hay que hacerlo con cualquier herramienta. Lo que el mecanismo del JSDoc
aporta es que ese texto **vive pegado a la cadena en el mismo fichero que la
cadena**, se revisa en la misma PR y no hay un segundo sitio que se desincronice.

### Las referencias `#:` no salen gratis, y son de fiar a medias

Con el `msgid` igual a la clave, un `#:` se podría fabricar buscando
`t("clave")` en `src/`. Se midió qué alcance tiene: de las 205 claves,
**116 aparecen literalmente en el código y 89 no** (el 43 %). Las 89 se alcanzan
por los **10 sitios con `t()` de plantilla** que hay en la aplicación:

```
errors.situations.${situation}.title      errors.situations.${situation}.body
preferences.sections.${section}           preferences.theme.${theme}
languages.${tag}                          panel.certificate.stores.${certificate.store}
progress.stages.${each}                   progress.stages.${term}
progress.states.${state}
```

Así que un `#:` por `grep` cubriría poco más de la mitad, y sería peor que no
ponerlo: un `.po` donde la mitad de las entradas no tiene referencia parece un
`.po` roto. **Recomendación: no emitir `#:`.** La clave ya dice dónde está la
cadena mejor que un número de línea, y el sondeo de Lingui ya recomendaba
`lineNumbers: false` por el ruido que meten en el `git diff`.

---

## 5. Escribirlo: 153 líneas, y el `Catalog` aguanta

Se escribió el mínimo que pedía el punto 2 del ticket y se ejecutó sobre el
catálogo real.

| prototipo | líneas | qué hace |
| --- | --- | --- |
| `ts2po.mjs` | 94 | `es.ts` → `es.po`, con `#.` desde el JSDoc y plurales por `Intl.PluralRules` |
| `po2ts.mjs` | 41 | `xx.po` → `xx.ts` anidado y tipado como `Catalog` |
| el arnés (§7) | 18 | el 100 %/0 % |
| **total** | **153** | contando comentarios y líneas en blanco |

Dependencias: **`gettext-parser` 9.1.1** (5 paquetes en total) para leer y
escribir `.po`, y el analizador de TypeScript que ya está en el repositorio.

### Leer el JSDoc en TypeScript 7: se puede, pero no como antes

Aquí hay una piedra que conviene dejar señalizada, porque no está en ninguna
documentación y costó dos intentos. **`typescript` 7.0.2 —la reescritura
nativa— ya no expone la API de compilador en JavaScript.** `require("typescript")`
resuelve a `lib/version.cjs` y `ts.createSourceFile` es `undefined`. Lo que sí
hay son exportaciones bajo `typescript/unstable/*`, y de ellas la que sirve es
el **escáner**:

```js
import { createScanner, SyntaxKind, LanguageVariant } from "typescript/unstable/ast";
import { getLeadingCommentRanges, tokenIsIdentifierOrKeyword } from "typescript/unstable/ast/scanner";
```

No hay analizador sintáctico suelto: hay un escáner de componentes léxicos y,
por otro lado, un `Program` completo que arranca el proceso Go. Para recorrer un
objeto literal el escáner basta. Tres detalles que hacen fallar el primer
intento y que no aparecen en ningún sitio:

* **`createScanner` cambió de firma**: es `createScanner(skipTrivia, languageVariant, text)`,
  sin el `ScriptTarget` que llevaba delante en TypeScript 5. Pasarlo como antes
  no da error: el escáner se queda sin texto y **el bucle no termina nunca**.
* **`SyntaxKind.EndOfFileToken` ya no existe**; ahora es `SyntaxKind.EndOfFile`.
  Comparar contra el nombre viejo es comparar contra `undefined`, y otra vez
  bucle infinito silencioso.
* **Hay claves que son palabras reservadas.** `errors.situations.unknown` se
  escanea como `UnknownKeyword`, no como `Identifier`; un extractor que filtre
  por `Identifier` se salta esa clave y **descoloca todas las de después**. La
  guardia es `tokenIsIdentifierOrKeyword(token)`.

Con eso, el extractor saca las 205 hojas y sus comentarios en una pasada.

### El resultado

```
205 hojas, 0 plurales, 3 formas para es, 2 con comentario
msgids únicos: 205
con #. : 2
msgstr vacíos: 0
```

```
#. El nombre propio del programa. No se traduce en ningún idioma, pero vive en
#. el catálogo para que ningún componente lo escriba en línea.
msgid "app.name"
msgstr "rFirma"
```

Y el viaje completo, `es.ts` → `es.po` → `es.ts`:

```
idéntico byte a byte tras ida y vuelta: true
```

Mismas claves, mismo orden, mismos valores. El orden importa más de lo que
parece: la prueba que ya existe, `i18n.test.tsx:23`, compara las rutas de claves
con `toEqual`, que es sensible al orden. **Sigue verde sin tocarla.**

### El `.ts` generado y `Catalog = typeof es`

La pregunta del ticket era si el `.ts` generado puede satisfacer `Catalog` sin
trucos. **Sí.** El generador escribe:

```ts
// Generado por po2ts.mjs desde eu.po. No editar a mano.
import type { Catalog } from "../catalog";

export const eu: Catalog = {
  app: {
    name: "rFirma",
  },
  …
};
```

Se metió ese fichero en `rfirma-app/src/i18n/locales/` y se pasó `pnpm exec tsc -b`
de verdad. Tres pruebas:

| fichero generado | `tsc -b` |
| --- | --- |
| las 205 cadenas traducidas | **pasa** |
| las 205 cadenas a `""` | **pasa** |
| igual, pero quitando una clave a mano | **falla**: `error TS2741: Property 'sign' is missing…` |

O sea que el tipo sigue haciendo su trabajo —la paridad de claves la comprueba
`tsc`, no una prueba— y además **un catálogo a medio traducir sigue siendo un
`Catalog` válido**, porque las hojas son `string` y `""` lo es. Esto encaja con
una pieza que ya está puesta: `i18n.ts:35` fija **`returnEmptyString: false`**,
y se midió lo que eso significa: con esa opción una cadena vacía cae al
castellano, y **sin ella** i18next pinta la cadena vacía y se salta el respaldo.
Es decir, **el respaldo al castellano de la decisión 6 del mapa ya está
construido**, y el `""` que deja un `msgstr` sin traducir es exactamente la
forma que necesita.

### El bucle completo, con `msgmerge` haciendo de bisagra

El generador escribe el `.po` entero cada vez, así que no puede escribir
directamente sobre el fichero donde vive el trabajo de quien traduce. La bisagra
es `msgmerge`, de las gettext-tools del sistema —**no es una dependencia de
npm**—:

```
es.ts  --ts2po-->  es.pot (msgstr vacíos)
                      |
   xx.po (traducciones) + es.pot  --msgmerge-->  xx.po al día
                      |
                   po2ts  -->  xx.ts
```

Medido sobre el catálogo real: `msgmerge` conserva las traducciones vivas,
marca como obsoleta la entrada que ya no está, **y trae los `#.` de la plantilla
a cada idioma** (0 comentarios en el `.po` viejo → 2 en el fusionado). También
conserva la marca `#, fuzzy`.

Con un aviso medido, y este es el que hay que tener presente: **si la plantilla
lleva el castellano en `msgstr`, `msgmerge` lo copia como si fuera la traducción**.
Al probarlo con la plantilla que `i18next-conv --pot` produce —que **no** vacía
los `msgstr`, pese al nombre—, las claves nuevas aparecían en el euskera con el
texto castellano dentro y **contando como traducidas**. Con eso, el arnés del
100 %/0 % daría por bueno un idioma que está en castellano. La plantilla tiene
que llevar los `msgstr` vacíos; el prototipo los vacía.

---

## 6. El arnés del 100 %/0 %: se puede leer de los dos sitios, y uno es gratis

La decisión 7 del mapa pide que la construcción compruebe *el castellano al
100 %; los demás, al 100 % o al 0 %*. Hay dos sitios de donde leerlo.

### Del `.po`: 18 líneas y una dependencia

```
es: 205/205 (100 %) OK
ca: 205/205 (100 %), 1 dudosas FALLA
en: 205/205 (100 %) OK
eu: 0/205 (0 %) OK
gl: 103/205 (50 %) FALLA
exit=1
```

Son 18 líneas sobre `gettext-parser`. Contrastado con `msgfmt --statistics`, que
está en cualquier máquina con gettext y da lo mismo:

```
po/es.po: 205 translated messages.
po/gl.po: 103 translated messages, 102 untranslated messages.
```

Ventaja del `.po`: es el único sitio donde se ve **`#, fuzzy`**, la marca de
«traducido pero dudoso», que en el `.ts` no deja rastro. En el ejemplo de
arriba, `ca` está al 100 % y aun así el arnés lo tumba porque tiene una entrada
dudosa.

### Del `.ts`: cero líneas nuevas y cero dependencias

Y esta es la opción que hay que mirar dos veces, porque **el código ya está
escrito**. `catalog.ts` tiene `catalogValues()` e `isComplete()`, y la regla
entera es:

```ts
const values = catalogValues(CATALOGS[tag]);
const done = values.filter((v) => v.trim() !== "").length;
// es: done === values.length. Los demás: done === values.length || done === 0.
```

Se cuelga como una prueba más de `i18n.test.tsx`, que ya corre en `just test-ts`
y por tanto en `just check` y en el CI. No hace falta receta nueva en el
`justfile`, ni tocar `ci.yml` (que ejecuta `just check` y nada más), ni añadir
un solo paquete.

**Recomendación: el arnés se lee del `.ts`.** El `.ts` es lo que se publica, así
que es lo que hay que medir; el `.po` es el fichero de trabajo. Si más adelante
se quiere cazar los `fuzzy`, se añade una comprobación aparte sobre los `.po`
—que es lo que son, un aviso a quien traduce— sin mezclarla con la puerta de
publicación.

Y el desplegable de idiomas se deriva igual que hoy, con `completeLanguages()`
en `languages.ts:43`: sale el idioma que no esté al 0 %. Esa función **no hay
que tocarla**.

### La consecuencia que la decisión 7 no tiene

La decisión 7 del mapa dice que *el `msgid` huérfano no necesita comprobación
aparte*, porque una `t()` sin entrada en el catálogo castellano baja el
castellano del 100 %. **Eso es cierto con Lingui y falso aquí**, y conviene
corregirlo antes de escribir el spec.

Con Lingui hay un extractor que **lee el código** y mete en el catálogo toda
`t()` que encuentra: la clave huérfana aparece en el catálogo sin traducir y el
porcentaje baja. Con i18next no hay extractor de nada: el catálogo es la fuente
y el `.po` se deriva de él. Una `t("clave.que.no.existe")` **no entra en ningún
recuento**; el castellano sigue al 100 % y la clave se pinta tal cual en
pantalla.

Y taparlo con un `grep` no es viable con este catálogo, por lo medido en §4:
**89 de las 205 claves no aparecen literalmente en el código**, así que una
comprobación por búsqueda de texto daría 89 falsos positivos y no encontraría
las huérfanas que se esconden detrás de los 10 `t()` de plantilla. Las defensas
que sí sirven, y que el spec tendrá que elegir:

* Tipar las claves (`t()` con las rutas de `Catalog` como tipo literal), que
  mata el problema en la raíz para las 116 claves literales y **no puede** con
  las de plantilla.
* Cubrir con pruebas los sitios de clave dinámica —que son 10, están
  enumerados— comprobando que cada valor posible del enumerado tiene entrada.
* Aceptarlo y confiar en que una clave sin traducir se ve a simple vista, que es
  lo que pasa hoy.

En todo caso, **no está resuelto por el arnés**, y el spec no debería dar por
hecho que lo está.

---

## 7. El coste en el flatpak: entre 12 entradas y ninguna

`packaging/flatpak/node-sources.json` tiene hoy **204 entradas** y **170 047
bytes**. Lo genera `flatpak-node-generator` desde `pnpm-lock.yaml`
(`justfile:947`), **incluye las dependencias de desarrollo** (ahí están Biome y
sus nueve binarios por plataforma, y TypeScript), y lo vigila
`check-flatpak-sources`, que es la **primera** dependencia de `just lint` y por
tanto de `just check` y del CI.

Lo que costaría cada opción:

| | paquetes nuevos | entradas nuevas |
| --- | --- | --- |
| `i18next-conv` | 12 | +12 |
| solo `gettext-parser` | 5 | +5 |
| ninguna dependencia npm | 0 | **0** |

Pero el coste de verdad no son las entradas: **es que el fichero no se puede
regenerar en este entorno.** `command -v flatpak-node-generator` falla, y
`just flatpak-sources` aborta a propósito cuando falta. Cualquier dependencia
nueva cambia `pnpm-lock.yaml`, y con él el sello de `sources.lock`, y
`check-flatpak-sources` se pone rojo hasta que alguien con el generador instalado
lo regenere. Es exactamente la piedra que `CLAUDE.md` documenta para
`cargo-sources.json`, con el agravante de que la de Cargo se puede reproducir a
mano y la de npm no.

Hay una atenuante que conviene conocer: **el manifiesto no usa `node-sources.json`
todavía**. `packaging/flatpak/me.sgomez.rfirma.yml` no ejecuta `pnpm` en ningún
sitio —`org.gnome.Sdk//50` no trae `node`— y el frontend entra ya construido
desde el anfitrión. O sea que una dependencia nueva **no añade un solo byte al
paquete**; añade trabajo de contabilidad en un fichero que hoy nadie consume.

Y sí, **se puede evitar del todo**. Dos caminos:

* **Cero dependencias.** El arnés se lee del `.ts` (§6) y no necesita nada. La
  conversión a `.po` no es un paso de construcción: se ejecuta a mano cuando hay
  que mandar cadenas fuera o traerlas, y lo generado se versiona. El conversor
  puede entonces vivir fuera de `package.json` —un `node --experimental-*` con
  `npx`, o un guion en `tools/` que se traiga lo suyo— igual que
  `flatpak-cargo-generator.py` vive fuera y nadie lo versiona.
* **Cinco entradas.** Si se prefiere que `gettext-parser` sea una dependencia de
  desarrollo normal, es la vía honesta y cuesta 5 entradas y una regeneración.

**Recomendación: el camino de cero.** No por las cinco entradas, sino porque
convierte «traducir» en una tarea que se hace cuando toca, con el resultado
versionado y revisable en la PR, en vez de un paso de construcción que hay que
mantener verde en el CI para siempre.

---

## Lo que esto obliga a decidir

Para el [#162](https://github.com/sgomez/rfirma/issues/162) y para el spec:

1. **`i18next-conv` se descarta**, y no por estar muerto. Si alguien lo quiere
   igualmente, tendrá que envolverlo para tapar tres pérdidas silenciosas, y eso
   es más código que los 153 renglones del conversor propio.
2. **El `msgid` es la clave**, con `.` de separador. No hace falta `#:`.
3. **Ninguna clave del catálogo lleva `_`.** Hoy se cumple —cero de 205— y a
   partir de la ficha 26 deja de ser una casualidad para ser una regla, porque
   `_` pasa a significar contexto y plural.
4. **Los comentarios de traductor hay que escribirlos.** El JSDoc de hoy son 20
   bloques de los cuales 2 cuelgan de una cadena; el resto es prosa de sección
   que gettext no sabe dónde poner. El mecanismo (JSDoc → `#.`) está medido y
   funciona; lo que falta es el texto.
5. **La entrada del catálogo sigue siendo una cadena**, no un objeto
   `{ message, comment, context }`. Convertirla en objeto tira `Catalog = typeof es`.
6. **El arnés se lee del `.ts`** y se cuelga de `i18n.test.tsx`. Cero
   dependencias, cero recetas nuevas, cero coste en el flatpak.
7. **La decisión 7 del mapa hay que corregirla en un punto**: el `msgid`
   huérfano **no** queda cubierto por el arnés. Hay que decir en el spec qué se
   hace con él, sabiendo que 89 de 205 claves no son visibles a un `grep`.
8. **`returnEmptyString: false` pasa a ser normativo.** Hoy está puesto en
   `i18n.ts:35` como detalle de implementación; con el `.po` en medio es la
   pieza de la que depende que un `msgstr` vacío caiga al castellano en vez de
   pintar un hueco. Merece una prueba con su nombre.
9. **La `Plural-Forms` del `.po` es decoración.** Quien reparte es
   `Intl.PluralRules` dentro de i18next. Hay que decirlo donde lo vea quien
   traduzca, porque su editor le enseñará otra cosa.
