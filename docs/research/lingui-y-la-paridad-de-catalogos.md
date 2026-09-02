# Lingui y la paridad de catálogos: qué repone el arnés y qué cuesta migrar

Sondeo del [#151](https://github.com/sgomez/rfirma/issues/151), hijo del mapa
[#148](https://github.com/sgomez/rfirma/issues/148). Decide la **ficha 26** del
informe *rFirma después de v0.1* —las traducciones en `.po` con contexto,
comentarios de traductor y plurales ICU— y enmienda el
[ADR-0009](../adr/0009-catalogo-de-cadenas-propio-y-seis-idiomas.md) en su
mecanismo, no en su promesa.

**Lingui está elegido**; el sondeo no compara herramientas. Lo que decide es
cómo se repone en la construcción lo que hoy garantiza `tsc`, y cuánto cuesta
mover las cadenas.

**Respuestas cortas.** Las estadísticas de completitud de Lingui **existen pero
no son legibles por máquina**: `lingui extract` pinta una tabla ANSI con
`cli-table3` y no hay `--json` ni `--reporter`. La regla del arnés —*el
castellano al 100 %; los demás, al 100 % o al 0 %*— **hay que escribirla**, y
son unas veinte líneas de Node sobre el analizador de `@lingui/format-po`.
Medido: el guion distingue traducido de vacío y de obsoleto, y —esto importa—
es **inmune al respaldo al castellano**, que Lingui resuelve en tiempo de
compilación y que por tanto engaña a `lingui compile --strict`. El respaldo se
configura con una línea, `fallbackLocales: { default: "es" }`.

De la migración: **no hay codemod** y no lo va a haber; `@lingui/codemods` solo
cubre saltos entre versiones de Lingui. Las claves nombradas se conservan tal
cual como ids explícitos, que es lo que hace la migración un renombrado
mecánico y no un rediseño. Los sitios donde hoy la gramática vive en la vista
son **cuatro, y tres mensajes distintos**; se funden en tres plurales ICU y
siete claves del catálogo se quedan en tres. El obstáculo real no son esos
cuatro: son los **tres `t()` de clave dinámica** que alcanzan catorce cadenas y
que el extractor **no ve**.

Y un aviso que no viene de la migración pero que la migración destapa: **el
locale `va` no existe en CLDR**. `Intl.PluralRules("va")` cae a `und` con una
sola categoría. En cuanto haya un plural en valenciano se resolverá siempre por
la rama `other`, en silencio.

---

## Cómo se midió

Un banco de pruebas real, no solo lectura de documentación. `@lingui/cli`,
`@lingui/core`, `@lingui/react`, `@lingui/format-po` y
`@lingui/babel-plugin-lingui-macro`, todos en la **6.6.0** (última publicada,
`time.modified` de 2026-07-24), instalados en un proyecto de usar y tirar con
Node **v24.15.0**. Fecha de las mediciones: **2026-09-02**.

Sobre ese banco se extrajo, se compiló y se ejecutó: catálogos de 9 y de 203
mensajes, con ids explícitos, con ids generados, con contexto, con comentarios
de traductor, con plurales, con huérfanos introducidos a mano y con un catálogo
a medio traducir. Todo lo que este informe da como cifra o como comportamiento
está observado ahí, salvo donde diga lo contrario.

En paralelo se contrastó la documentación oficial (`lingui.dev`) y el código
del repositorio `lingui/js-lingui`, que es donde viven las respuestas que la
documentación no da.

---

## El veredicto

| Lo que hoy garantiza el compilador | Con qué se repone | ¿De serie? |
| --- | --- | --- |
| `Catalog = typeof es`, paridad exacta de claves | El código pasa a ser la fuente de verdad; la paridad se vuelve «cada catálogo cubre lo extraído» | — |
| `isComplete()`, filtro del desplegable | Un guion de ~20 líneas sobre `@lingui/format-po`, en la construcción | **No** |
| Clave huérfana que se pinta en pantalla | El mismo guion: una `t()` sin entrada en castellano baja el castellano del 100 % | **No** |
| — | Respaldo al castellano | Sí, `fallbackLocales` |

Lingui **no trae ninguna puerta de CI lista para esto**. Lo que trae es
`compile --strict`, que no sirve aquí por dos motivos medidos más abajo. La
petición de un `lingui check` lleva abierta desde 2020: la
[#722](https://github.com/lingui/js-lingui/issues/722) se cerró en noviembre de
2020 sin implementarse, y la [#2195](https://github.com/lingui/js-lingui/issues/2195)
sigue abierta hoy pidiendo lo mismo.

Esto no desmiente la elección de Lingui. Dice qué hay que construir con ella.

---

## 1. De dónde salen las estadísticas, y en qué formato

### La tabla de `lingui extract`

`lingui extract` imprime al terminar una tabla de completitud por idioma. Es la
salida literal del banco, con 203 mensajes y el euskara traducido a medias:

```
Catalog statistics for locales/{locale}:
┌─────────────┬─────────────┬─────────┐
│ Language    │ Total count │ Missing │
├─────────────┼─────────────┼─────────┤
│ es (source) │     203     │    -    │
│ ca          │     203     │   203   │
│ en          │     203     │   203   │
│ eu          │     203     │   202   │
│ gl          │     203     │   203   │
│ va          │     203     │   203   │
└─────────────┴─────────────┴─────────┘
```

Y ahí se acaba. **No hay salida legible por máquina.** `lingui extract --help`,
en la 6.6.0, lista exactamente nueve opciones y ninguna es de formato:

```
--config <path>            --locale <locale, [...]>   --workers <n>
--overwrite                --clean                    --debounce <delay>
--verbose                  --watch                    -h, --help
```

No hay `--json`, ni `--reporter`, ni `--check`. La tabla se dibuja con
`cli-table3`, con bordes Unicode y colores ANSI: analizarla en un guion es
frágil por construcción y no se recomienda.

**Y `extract` sale con código 0 aunque falte todo.** Sale con 1 en tres casos, y
ninguno es «hay traducciones que faltan»: un `--locale` que no está en la
configuración, y dos fallos de la extracción en sí. Como puerta, no vale.

### Lo que `extract` sí hace, y que el arnés aprovecha

Dos comportamientos medidos que importan para saber **qué cuenta como 100 %**:

**Distingue traducido de vacío.** Un mensaje sin traducir es `msgstr ""`. Es lo
que cuenta la columna `Missing`.

**Distingue traducido de obsoleto.** Cuando una cadena desaparece del código,
`extract` no la borra: la marca obsoleta con el prefijo `#~` de gettext. Medido
en el banco, tras quitar dos claves del código y volver a extraer:

```po
#~ msgid "actions.deleted2"
#~ msgid "actions.deleted"
```

Con `--clean` desaparecen del fichero en lugar de quedarse marcadas. Para el
arnés la diferencia es material: **las obsoletas no cuentan ni arriba ni abajo**
—no están en el catálogo de origen, así que no son «cadenas que faltan»— pero
si se dejan acumular, un `.po` puede tener el doble de bultos que de mensajes
vivos. La recomendación es correr `extract --clean` en la construcción, de modo
que el `.po` versionado solo contenga mensajes vivos y la cuenta sea directa.

### El analizador, que es lo que sí es API

La cuenta que hace `getStats` dentro del CLI es trivial —total de claves y
claves sin `translation`— pero **`getStats` no se exporta**. Lo que sí es
público, y es la vía limpia, es el analizador del propio formateador:

```js
import { formatter } from "@lingui/format-po";
const fmt = formatter({ lineNumbers: false });
const catalog = fmt.parse(readFileSync("locales/eu.po", "utf8"), {
  locale: "eu", sourceLocale: "es", filename: "locales/eu.po",
});
// -> { "actions.sign": { translation: "Dokumentua sinatu", ... }, ... }
```

Devuelve un objeto indexado por id con el campo `translation`. Es sobre esto
sobre lo que se escribe el arnés.

---

## 2. El arnés: el castellano al 100 %, los demás al 100 % o al 0 %

### Por qué no vale `lingui compile --strict`

Es la opción que parece hecha para esto —la documentación la describe como
«fail if a catalog has missing translations»— y **falla por dos motivos, los
dos medidos**.

**Primero: es todo o nada entre idiomas.** `lingui compile` no acepta
`--locale`; en el banco devuelve `error: unknown option '--locale'`. O se exige
el 100 % a los seis catálogos, o a ninguno. Con cuatro catálogos vacíos a
propósito, `--strict` está en rojo permanente:

```
Error: Failed to compile catalog for locale en!
Missing 13 translation(s)

Error: Failed to compile catalog for locale ca!
Missing 13 translation(s)
```

La regla que se quiere no es «todos al 100 %», es «al 100 % **o** al 0 %», y
esa disyunción `--strict` no la sabe expresar.

**Segundo, y peor: el respaldo lo ciega.** Con `fallbackLocales` configurado,
`--strict` valida el catálogo **después** de aplicar los respaldos, así que un
catálogo vacío pasa la puerta con el castellano dentro. Es un desajuste
conocido y abierto, la
[#2506](https://github.com/lingui/js-lingui/issues/2506), y en el banco se
reproduce entero: con `fallbackLocales: { default: "es" }` y el euskara
traducido a 1 de 203, `eu.js` se compila **con el mismo tamaño exacto** que el
castellano —22 800 bytes frente a 22 799— porque lleva las 202 cadenas
españolas incrustadas.

Como el respaldo al castellano es ahora una decisión tomada (§3), esto liquida
`--strict` para este uso. El arnés tiene que leer los `.po`, no lo compilado.

### El guion

`rfirma-app/scripts/i18n-gate.mjs`. Es el mínimo que expresa la frase entera:

```js
// El castellano al 100 %; los demás, al 100 % o al 0 %.
// Un idioma a medias no se publica (ADR-0009, enmendado).
import { formatter } from "@lingui/format-po";
import { readFileSync, writeFileSync } from "node:fs";

const SOURCE = "es";
const LOCALES = ["es", "ca", "eu", "gl", "va", "en"];
const fmt = formatter({ lineNumbers: false });

const read = (l) =>
  fmt.parse(readFileSync(`src/i18n/locales/${l}.po`, "utf8"), {
    locale: l, sourceLocale: SOURCE, filename: `src/i18n/locales/${l}.po`,
  });

const sourceIds = Object.keys(read(SOURCE));
const complete = [];
let failed = false;

for (const locale of LOCALES) {
  const catalog = read(locale);
  const done = sourceIds.filter((id) => catalog[id]?.translation?.trim());
  const orphan = Object.keys(catalog).filter((id) => !sourceIds.includes(id));
  const pct = done.length / sourceIds.length;

  if (orphan.length > 0) {
    console.error(`✗ ${locale}: ${orphan.length} clave(s) que ya no existen: ${orphan.join(", ")}`);
    failed = true;
  }
  if (locale === SOURCE ? pct < 1 : pct !== 0 && pct !== 1) {
    const missing = sourceIds.filter((id) => !done.includes(id));
    console.error(
      `✗ ${locale}: ${done.length}/${sourceIds.length} — ni al 100 % ni al 0 %.\n` +
        `  faltan: ${missing.slice(0, 10).join(", ")}${missing.length > 10 ? "…" : ""}`,
    );
    failed = true;
  } else {
    console.log(`✓ ${locale}: ${done.length}/${sourceIds.length}`);
    if (pct === 1) complete.push(locale);
  }
}

writeFileSync("src/i18n/complete-languages.json", `${JSON.stringify(complete, null, 2)}\n`);
process.exit(failed ? 1 : 0);
```

Está probado en el banco. Con el euskara a 1 de 203 y el respaldo al castellano
activo en la configuración, distingue lo que tiene que distinguir:

```
es traducidas 203 / 203  huérfanas 0  completo True
ca traducidas   0 / 203  huérfanas 0  completo False
eu traducidas   1 / 203  huérfanas 0  completo False   <- el que rompe la puerta
gl traducidas   0 / 203  huérfanas 0  completo False
va traducidas   0 / 203  huérfanas 0  completo False
en traducidas   0 / 203  huérfanas 0  completo False
```

Los cuatro al 0 % pasan; el euskara a medias no. Es exactamente la frase.

Tres cosas que el guion resuelve y conviene no perder de vista:

- **Es inmune al respaldo.** Lee los `.po` de origen, donde el castellano de
  respaldo todavía no se ha incrustado. Por eso ve `eu` a 1/203 mientras
  `compile --strict` lo vería a 203/203.
- **Caza la clave huérfana sin ser una comprobación aparte.** Una `t()` nueva
  sin pasar por `extract` deja el castellano por debajo del 100 % y la puerta se
  pone roja. Y al revés: una entrada en el `.po` que ya no está en el código sale
  por la lista `orphan`. Medido en el banco: la clave `actions.deleted2` que
  metí a mano aparece en la lista.
- **Escribe la lista de idiomas completos.** `complete-languages.json` es el
  sustituto directo de `completeLanguages()`, pero calculado en la construcción
  en vez de en el arranque. El desplegable de Preferencias lo importa y se acabó
  el recorrido recursivo del catálogo en tiempo de ejecución.

### Dónde se cuelga

En el `justfile`, dentro de `lint-ts`, que ya está debajo de `lint` y por tanto
de `check`. `docs/agents/code-host.md` promete que el CI ejecuta `just check` y
nada más, así que **no se toca `.github/workflows/ci.yml`**: la puerta entra
sola por el paso «just check» que ya existe (`ci.yml:153-156`).

```just
lint-ts: deps
    cd {{ app }} && pnpm exec biome ci .
    # La ficha 26: los catálogos al día con el código, y ningún idioma a medias.
    cd {{ app }} && pnpm exec lingui extract --clean
    cd {{ app }} && git diff --exit-code -- src/i18n/locales/
    cd {{ app }} && node scripts/i18n-gate.mjs
```

Los tres mandos son tres cosas distintas y las tres hacen falta:

1. `lingui extract --clean` **regenera** los `.po` desde el código y borra los
   obsoletos.
2. `git diff --exit-code` falla si esa regeneración cambió algo, o sea si quien
   escribió el código no volvió a extraer. Es lo más parecido que hay a
   `Catalog = typeof es`.
3. El guion aplica la regla del 100 % o el 0 % y escribe la lista.

Para que el paso 2 no dé falsos positivos hay que poner **`lineNumbers: false`**
en el formateador de `lingui.config.ts`. Con las líneas activadas, las
referencias `#:` llevan `fichero:línea` y cualquier reordenación de código
—añadir un `import`— reescribe los seis `.po` y pone la puerta en rojo sin que
haya cambiado ni una cadena. Con `lineNumbers: false` la referencia se queda en
la ruta del fichero, que es lo que el traductor necesita.

Como el `.po` se regenera en `lint-ts`, se versiona igual que hoy se versionan
los `.ts`: el `git diff` es la puerta, no un efecto secundario.

---

## 3. El respaldo al castellano

Una línea en `rfirma-app/lingui.config.ts`:

```ts
import { defineConfig } from "@lingui/cli";
import { formatter } from "@lingui/format-po";

export default defineConfig({
  sourceLocale: "es",
  locales: ["es", "ca", "eu", "gl", "va", "en"],
  fallbackLocales: { default: "es" },
  catalogs: [{ path: "<rootDir>/src/i18n/locales/{locale}", include: ["src"] }],
  format: formatter({ lineNumbers: false }),
});
```

**Lo importante es cuándo actúa: en la compilación, no en el arranque.** Medido:
con esa configuración y el euskara traducido a 1 de 203, el `eu.js` compilado
lleva dentro las 202 cadenas castellanas, texto a texto, incluidos los plurales:

```js
{"actions.cancel":["Cancelar"],"actions.sign":["Dokumentua sinatu"],
 "panel.document.pages":[["n","plural",{"one":["#"," página"],"other":["#"," páginas"]}]], …}
```

De ahí tres consecuencias, ninguna obvia:

- **No hay cadena de respaldo en tiempo de ejecución.** La aplicación carga un
  catálogo y ya está completo. No hay que cargar dos, ni ordenarlos, ni pagar
  una búsqueda por cada cadena.
- **Los seis catálogos compilados pesan lo mismo.** 22 799 bytes el castellano,
  22 800 el euskara para 203 mensajes; 1,5 KB comprimidos. Un idioma vacío no
  ahorra nada. Con carga perezosa por idioma se paga uno solo, así que da igual.
- **Y es la razón por la que el arnés del §2 no puede leer lo compilado.** Todo
  parece traducido después de compilar. El respaldo es la red de quien
  desarrolla —que nunca vea una clave en pantalla mientras trabaja— y por eso
  mismo borra la señal que la puerta necesita.

### Lo que pasa cuando no hay respaldo al que caer

El caso que el respaldo no cubre: una `t()` cuyo id no está **en ningún**
catálogo. Medido con `@lingui/core` sobre un catálogo con una sola clave:

```
1 clave presente        : "Dokumentua sinatu"
2 ausente, con message  : "Cancelar"        <- el literal del código salva
3 ausente a secas       : "actions.gone"    <- la clave, cruda, en pantalla
4 eventos "missing"     : [{"id":"actions.cancel","locale":"eu"},
                           {"id":"actions.gone","locale":"eu"}]
```

El caso 3 es el que el ADR-0009 no quiere ver nunca, y el arnés del §2 lo
impide antes de publicar. El caso 2 merece un aviso: **funciona porque el macro
conserva el `message:` del código**, y eso deja de ser cierto en producción. La
opción `descriptorFields` vale `"auto"` por omisión, y «auto» significa
*en producción, solo el id*. Es decir: el literal que salva en desarrollo **no
está en el binario publicado**. Otra razón para que la puerta sea de
construcción y no una confianza en el tiempo de ejecución.

Lo cuarto de la lista es aprovechable: Lingui emite un evento `missing`. Un
`missingHandler` que lance en las pruebas convierte cualquier clave sin resolver
en un test rojo, gratis.

---

## 4. Lo que cuesta migrar

### El tamaño real del trabajo

`locales/es.ts` tiene **451 líneas y 200 cadenas hoja**; los otros cinco, 362-365
líneas cada uno, con las mismas 200 claves vacías. En el código hay **142
llamadas `t("…")` con clave literal distinta**, repartidas por 17 componentes.

(El ticket habla de 425 cadenas; son 425 *líneas* de una versión anterior. Las
cadenas son 200. El trabajo es la mitad de grande de lo que parecía.)

### No hay codemod, y no hace falta uno grande

`@lingui/codemods` existe pero solo cubre saltos entre versiones de Lingui
—`split-macro-imports` para el 4 → 5— y no hay ninguna herramienta para migrar
desde un catálogo propio. `lingui extract --convert-from` tampoco sirve:
convierte **entre formatos de catálogo de Lingui**, no desde un objeto
TypeScript.

Ahora bien, la conversión mecánica es de un guion de una tarde, porque **las
claves nombradas se conservan**. Los ids explícitos están plenamente soportados
y la documentación los pone como ejemplo con exactamente esta forma
(`index.header.title`, `modal.buttons.cancel`). O sea que
`t("panel.coSignature.one")` se convierte en un `t({ id: "…", message: "…" })`
con el **mismo id**, y ni los `.po` ni el código pierden la trazabilidad con lo
de hoy.

Dos opciones que hay que poner desde el primer día para que el `.po` sea
legible y el diff estable:

- **`explicitIdAsDefault: true`** en el formateador: el `msgid` sale como
  `msgid "panel.coSignature"` en vez de un hash. Sin esto, un traductor abre el
  `.po` y ve `msgid "MbT6FE"`.
- **`lineNumbers: false`**, por lo del §2.

Y una guardia barata: la regla `require-explicit-id` de `@lingui/eslint-plugin`
impide que se cuele un mensaje con id generado por descuido. Es una puerta de
convención que sí viene de serie, al contrario que las del §2.

### La trampa del id dentro del plural

Medida en el banco, y silenciosa, que es lo malo. La forma que uno escribe por
intuición **no funciona**:

```ts
plural(count, { id: "panel.document.pages", one: "# página", other: "# páginas" })
```

El `id` no se toma como id: se cuela dentro de la cadena ICU como si fuera una
rama más del plural.

```po
msgid "{count, plural, id {panel.document.pages} one {# página} other {# páginas}}"
```

`lingui extract` lo acepta sin decir nada. Quien protesta es `lingui compile`,
y **sale con 0 salvo que se le pase `--strict`**:

```
Reason: The plural case id is not valid in this locale at line 1 col 17
You can fail command execution on these errors by passing `--strict` option
```

La forma correcta es envolver el plural:

```ts
t({ id: "panel.document.pages", message: plural(count, { one: "# página", other: "# páginas" }) })
```

que da el `.po` que se espera:

```po
#. js-lingui-explicit-id
#: src/signing/SigningPanel.tsx
msgid "panel.document.pages"
msgstr "{count, plural, one {# página} other {# páginas}}"
```

El guion de migración tiene que emitir esta forma, y conviene que el paso
`compile` de la construcción lleve `--strict` **solo para esto**: aunque no
sirva como puerta de completitud (§2), sí es el único que caza el ICU
malformado. Como el `--strict` también exige el 100 % a los seis, la vía es
correr un `lingui compile` a secas y hacer fallar si la salida trae la palabra
`Reason:`, o correr `--strict` sobre una configuración que solo declare el
castellano.

### Otra trampa: el ternario que se colapsa en `{0}`

Si la gramática se deja en la vista y se envuelve entera, Lingui la aplana:

```ts
t({ id: "d.greeting", message: `${gender === "f" ? "Bienvenida" : "Bienvenido"} ${name}` })
```

```po
#. placeholder {0}: gender === "f" ? "Bienvenida" : "Bienvenido"
msgid "d.greeting"
msgstr "{0} {name}"
```

El traductor recibe `{0} {name}` y no puede traducir el saludo, que es
justamente la parte que había que traducir. La decisión gramatical tiene que
subir al catálogo como `select` o `plural`; no vale envolver el ternario.

### Los cuatro sitios donde hoy la gramática vive en la vista

Son **cuatro llamadas y tres mensajes distintos**, porque el contador de páginas
está duplicado en dos paneles:

| Sitio | Qué decide | Claves de hoy |
| --- | --- | --- |
| `signing/SigningPanel.tsx:161-163` | páginas, 1 frente a N | `panel.document.pages.one`, `.many` |
| `signing/SignedPanel.tsx:91-93` | lo mismo, duplicado | (las mismas dos) |
| `signing/SigningPanel.tsx:178-180` | firmas previas, 1 frente a N | `panel.coSignature.one`, `.many` |
| `signing/PinDialog.tsx:143-151` | intentos: ninguno conocido / 1 / N | `pin.incorrectUnknown`, `pin.incorrectOne`, `pin.incorrect` |

**Siete claves del catálogo se quedan en tres mensajes ICU.** El de las páginas:

```ts
const pages = (count: number) =>
  t({ id: "panel.document.pages",
      message: plural(count, { one: "# página", other: "# páginas" }) });
```

y en la vista desaparecen las dos ramas: `{pages(document.pages)}`.

El del PIN es el más interesante porque no es un plural puro: la rama «no se
sabe cuántos intentos quedan» no es una categoría de CLDR. Se resuelve con la
rama exacta `_0` —el `=0` de ICU, que en JSX no es un nombre de prop válido— o
dejando esa rama como un mensaje aparte y el plural para 1/N. Lo segundo es más
honesto: son dos situaciones distintas, no dos números.

El resto de ternarios `? t(…) : t(…)` que hay en el código —`failure ?
t("panel.footer.retry") : t("actions.sign")`, `locked ? … : …`, la rúbrica
`change`/`choose`— **no son gramática, son contenido condicional**, y se quedan
como están. Sería un error meterlos en un `select`.

### Lo que el extractor no ve, y es el problema de verdad

Tres llamadas construyen la clave con una plantilla:

```
preferences/PreferencesDialog.tsx:230,266   t(`preferences.sections.${section}`)
preferences/PreferencesDialog.tsx:345        t(`preferences.theme.${theme}`)
preferences/PreferencesDialog.tsx:356        t(`languages.${tag}`)
```

Entre las tres alcanzan **catorce cadenas** —4 secciones, 4 temas y los 6
endónimos de idioma— y **el extractor no puede verlas**: el macro trabaja sobre
el árbol sintáctico y ahí no hay ninguna cadena literal que extraer.

Esto choca de frente con el arnés del §2, y de las dos maneras a la vez: esas
catorce nunca entrarían en el `.po` (así que en pantalla saldría la clave
cruda), y si se meten a mano, `extract --clean` las borraría por obsoletas y el
`git diff` pondría la puerta en rojo.

La salida es la que recomienda la documentación para estos casos: declarar los
catorce mensajes explícitamente con `msg()` en un mapa, y que la vista indexe el
mapa en vez de construir la clave.

```ts
const SECTION_LABELS: Record<Section, MessageDescriptor> = {
  appearance: msg({ id: "preferences.sections.appearance", message: "Apariencia" }),
  // …
};
// en la vista: {i18n._(SECTION_LABELS[section])}
```

Es más verboso que hoy, pero devuelve la comprobación que la plantilla se
llevaba: si mañana se añade una sección, `tsc` exige su entrada en el `Record` y
el extractor la ve. **Es el único punto de la migración donde el trabajo no es
mecánico**, y conviene que sea un sub-issue propio.

### El valenciano y CLDR

Independiente de la migración, pero sale a la luz con ella. Medido en Node
v24.15.0:

| Etiqueta | Se resuelve a | Categorías de plural |
| --- | --- | --- |
| `es` | `es` | `one`, `many`, `other` |
| `ca` | `ca` | `one`, `many`, `other` |
| `eu` | `eu` | `one`, `other` |
| `gl` | `gl` | `one`, `other` |
| `en` | `en` | `one`, `other` |
| **`va`** | **`und`** | **`other`, y nada más** |
| `ca-ES-valencia` | `ca` | `one`, `many`, `other` |

`va` es el código ISO-639 de valenciano y **CLDR no lo reconoce**. Lingui no
valida ni normaliza el código: se lo pasa a `Intl.PluralRules` tal cual, y esa
llamada cae a `und`. Consecuencia: **todo plural en valenciano se resolverá
siempre por `other`**, sin error y sin aviso. Con «2 páginas» no se nota; con un
idioma que tuviera formas distintas, sí.

La etiqueta correcta es **`ca-ES-valencia`**, con la subetiqueta de variante
registrada en IANA, que resuelve a `ca` y hereda sus tres categorías. Como la
etiqueta se persiste en la preferencia y la comparte el backend
(`signing/language.rs`, `Language::tag`), cambiarla **toca los dos lados** y no
es gratis. Es una decisión para el spec, no para este informe. Lo que el informe
afirma es que `va` está roto para plurales hoy y lo seguirá estando después de
migrar.

---

## Por qué no Fluent

Se descartó sin sondearlo. El motivo de migrar es entrar en el ecosistema
`.po` —Weblate, Crowdin, Poedit— y los `.ftl` de Fluent no son gettext: Poedit
es un editor de gettext y no los abre. Lo que Fluent aporta de verdad
—selectores por género y por caso, atributos de mensaje, y que un traductor
pueda añadir en su idioma una distinción que el idioma de origen no tiene— es
real y en euskara no sería un capricho, pero es un argumento para una aplicación
con mucha más prosa que ésta. Con 200 cadenas y tres plurales, se paga un
ecosistema entero por una capacidad que hoy no se usa. Si algún día la
declinación vasca se convierte en un problema medido y no supuesto, se reabre.

---

## Lo que este sondeo obliga a decidir

1. **El arnés es código nuestro**, `rfirma-app/scripts/i18n-gate.mjs`, y va
   dentro de `lint-ts`. No hay opción de Lingui que lo haga, y las dos que se le
   parecen (`compile --strict`) están medidas y no sirven.
2. **El ADR-0009 se enmienda en el mecanismo.** La promesa —nunca media pantalla
   en otro idioma— sigue; lo que cambia es que la vigila la construcción y no un
   filtro del desplegable. `isComplete()` y `completeLanguages()` desaparecen, y
   `complete-languages.json` los sustituye.
3. **`descriptorFields` queda en `"auto"`** y con ello el literal del código no
   viaja al binario. Es coherente con lo anterior: la red es la puerta, no el
   tiempo de ejecución.
4. **`lineNumbers: false` y `explicitIdAsDefault: true`** no son gustos: sin la
   primera la puerta da falsos positivos, sin la segunda el `.po` es ilegible.
5. **Los catorce mensajes de clave dinámica son un sub-issue propio.** Es el
   único trabajo no mecánico de la migración y el único que puede dejar claves
   crudas en pantalla si se hace a medias.
6. **`va` frente a `ca-ES-valencia`** es una decisión pendiente que toca el
   backend. No la resuelve este informe.

## Lo que no se midió

- **El extractor sobre el código real de rfirma.** El banco usa ficheros
  escritos para el sondeo, no `rfirma-app/src`. Las 142 llamadas no se han
  extraído de verdad porque todavía no usan macros.
- **La vía de SWC.** La documentación marca `@lingui/swc-plugin` como
  experimental y avisa de que hay que fijar la versión exacta de `@swc/core`. En
  un proyecto empaquetado en flatpak, con `node-sources.json` vendorizado, ese
  acoplamiento es fricción; la vía de Babel con `@vitejs/plugin-react` es la
  aburrida. No se ha probado ninguna de las dos sobre este repositorio.
- **El coste en `packaging/flatpak/node-sources.json`.** Lingui añade
  dependencias de desarrollo y ese fichero se regenera con
  `flatpak-node-generator`, que `just flatpak-sources` no puede correr en este
  entorno. El número de entradas nuevas no está contado.
- **El peso en el paquete final.** Los tamaños de este informe son de los
  catálogos compilados (22,8 KB por idioma, 1,5 KB comprimidos, 203 mensajes),
  no del `vite build` de rfirma con `@lingui/core` y `@lingui/react` dentro. La
  documentación de Lingui no publica cifras de peso de sus paquetes.
