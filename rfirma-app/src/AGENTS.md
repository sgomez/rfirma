# Mapa de la interfaz (React 19 + TypeScript)

Este índice **sustituye a explorar el árbol**. Cada módulo dice qué es en su
primera línea `//!`, y `just outline rfirma-app/src/<carpeta>/` las junta en un
índice: localiza el módulo por su línea y abre **solo** ese fichero.

`src-tauri/tests/agents_map_is_complete.rs` exige que todo `.ts`/`.tsx`
versionado bajo `src/` que no sea un `*.test.*` abra con esa línea, sin
partirla: **una frase, qué es y, si ayuda, qué no es**, de 300 caracteres como
mucho y sin citas a la spec ni a issues. **Un módulo nuevo trae su cabecera en
la misma PR que lo crea**, o el PR sale en rojo.

## Presupuesto de lectura

- **Para saber qué puede pedirle la ventana al backend, `just contract`. No abras
  `src-tauri/src/<contexto>/adapters/`.** Imprime las órdenes y los tipos que cruzan, con
  los nombres de campo que ve TypeScript (`holderName`, no `holder_name`). Se
  genera de las fuentes en cada ejecución, así que no puede quedarse obsoleto.
- **Para situarte, `just outline <ruta>`; nunca `cat` de un módulo de más de 300
  líneas.** Desde el esqueleto, `just outline a.tsx:A-B,C-D b.ts:E-F` con **todos** los tramos en
  una sola llamada — un turno por tramo sale más caro que haber leído el módulo
  entero.
- **El typecheck es `tsc -b` (o `just check-ts`), nunca `tsc --noEmit`.** El
  `tsconfig.json` de la raíz de `rfirma-app/` es `{"files": [], "references":
  [...]}`: `pnpm exec tsc --noEmit` sale en verde **sin mirar un solo fichero**.
  `vitest` tampoco compila los `*.test.ts`, así que un campo nuevo en un tipo
  compartido que solo usan las pruebas pasa los dos y cae en el CI.
- Los tests viven en `*.test.ts(x)` **al lado** del módulo. No los abras salvo
  que vayas a tocarlos; `grep -n "it(\|describe(" <fichero>.test.tsx` dice qué
  cubren en una línea por caso.
- **`i18n/locales/` NO ESTÁ EN EL REPOSITORIO.** Los catálogos los genera
  `tools/po-import.mjs` desde `rfirma-app/po/` en cada `just build-ts`. La
  fuente de verdad de una cadena es `po/messages.pot` y los cinco `po/*.po`;
  para una clave concreta, `grep -n '<clave>' po/es.po`. **Una cadena nueva se
  escribe en el `.pot`, se corre `just po`, y entonces se compila.** Editar un
  `.ts` de `locales/` no sirve de nada: se sobrescribe.

## Dónde vive qué

`just outline rfirma-app/src/<carpeta>/` da el índice de una carpeta, y
`just outline rfirma-app/src/` el de la interfaz entera. Las carpetas:

| Carpeta | Qué es |
|---|---|
| raíz: `main.tsx`, `tauri*.ts` | El cableado de la ventana principal y los adaptadores de Tauri de cada puerto; `tauri.ts` es su punto de import (ver «La regla del puerto»). |
| raíz: `App.*` | El árbol de la ventana principal, `App.tsx`, y sus ganchos y piezas, uno por fichero `App.<pieza>`. |
| `shell/` | La ventana y su cabecera (ADR-0007). |
| `documents/` | Los documentos abiertos y los recientes. |
| `signing/` | La firma, en el lado de la interfaz. |
| `viewer/` | El visor de PDF. |
| `status/` | El estado de la instalación. |
| `preferences/` | Los ajustes. |
| `i18n/` | Catálogo propio, cinco idiomas, generado desde `po/` (ADR-0009 enmendado). Los bloques de comentario que explican el mecanismo están indexados en `i18n/AGENTS.md`. `i18n/locales/*.ts` son generados y no versionados: no se leen ni se editan. |
| `errors/` | Los fallos que ve el usuario. |
| `design-system/` | Los iconos, copiados en línea de los artboards, y el bundle CSS del sistema de diseño. |
| `desktop/` | El escritorio de la persona, en el lado de la interfaz. |
| `sede/` | La ventana que abre una sede por `afirma://`: una ventana con una secuencia de momentos, no una pantalla por momento. Ficha: `docs/design/ventana-de-sede.md`, que numera los momentos (1, 1c, 2, 2b, 3, 4, 5) como las cabeceras de sus componentes. |
| `updates/` | La versión nueva: la franja que la anuncia y su instalación. |
| `about/` | El diálogo Acerca de. |
| `setup/` | El asistente del primer arranque (`docs/design/primer-arranque.md`). Usa los casos de uso del panel de estado, no tiene los suyos propios. |
| `testing/`, `test-setup.ts` | Andamiaje de las pruebas, como los `*.testSupport.tsx`, los `*Fixtures.ts` y `viewer/testing/`. No son la aplicación. |

Dos módulos se leen antes que sus hermanos, porque cablean una ventana entera:
**`main.tsx`** (la principal, `index.html`), por donde se empieza siempre, y
`sede/main.tsx` (la de sede, `sede.html`), que monta la suya sin el árbol de
`App`.

## El circuito de cadenas (ADR-0009 enmendado)

```
po/messages.pot ──msgmerge──▶ po/{es,ca,eu,gl,en}.po ──po-import──▶ src/i18n/locales/*.ts
   versionado                       versionados                  generados, NO versionados
```

Cuatro comprobaciones, cada una en su sitio y sin solaparse:

| Qué falla | Quién lo caza |
|---|---|
| Las claves no cuadran entre idiomas | `tsc` sobre el `.ts` generado (`Catalog = typeof es`) |
| Un idioma a medias, o con `#, fuzzy` | `just check-po` (`msgfmt --statistics`, `msgcmp`) |
| Una `t()` sin entrada en el catálogo | `just lint-i18n` (`i18next-cli extract --ci`) |
| Una clave del catálogo que no usa nadie | `just lint-i18n` (`i18next-cli status --unused`) |

Las dos últimas miran el código en busca de `t("…")` literales, y de ahí salen
sus tres puntos ciegos. **Una clave ensamblada con plantilla** (`t(\`a.b.${x}\`)`)
no la ve ninguna: las dos pasan en verde aunque la clave resultante no exista en
el catálogo, y el hueco solo aparece en pantalla — escribe siempre la clave
entera. **El extractor lee también los comentarios**, así que un `t()` de
ejemplo dentro de un bloque `/** */` cuenta como clave usada: puede poner
`extract --ci` en rojo o falsear el recuento de claves sin uso. Y **no lee los
tests** (`i18next.config.ts` ignora `*.test.{ts,tsx}`): una clave que solo cita
un test cuenta como sin usar, y `status --unused` se pone en rojo aunque el test
la traduzca.

**Al ampliar el catálogo, la primera pasada de `just po` se espera en rojo.** El
orden es: escribir la cadena en el `.pot` → `just po` (`msgmerge` deja las
entradas nuevas vacías y `po-import.mjs` aborta con «po/es.po no está completo, y
es el original») → traducir las entradas nuevas en los `.po` → `just po` otra
vez. Esa primera pasada roja **no es una regresión**: es la receta diciendo qué
falta por traducir.

**El idioma que no está al 100 % no genera `.ts`**, así que no puede llegar al
desplegable: no es una comprobación, es que no existe. `just po --all` genera
también los incompletos, rellenando con castellano, para quien traduce; nunca
en el CI.

## La regla del puerto

La ventana no habla con Tauri: habla con puertos declarados en su propio
módulo, y `main.tsx` elige la implementación (ADR-0017). Si vas a añadir
capacidad nueva, el orden es: el puerto en su módulo de dominio → `tauri.ts` →
`main.tsx`. Las fichas de pantalla viven en `docs/design/` (ver
`docs/AGENTS.md`).

## Trampas al probar

Cada una costó un ciclo de arreglo entero, y ninguna se ve leyendo el código:

- **`jsdom` no trae `ResizeObserver` ni `scrollIntoView`.** Un módulo que los
  use necesita guardia (`typeof ResizeObserver === "undefined"`) o revienta
  cualquier prueba que monte el componente.
- **React 19 registra `wheel`, `touchstart` y `touchmove` como oyentes
  pasivos**, así que `preventDefault()` dentro de la prop `onWheel` **no hace
  nada** — y `fireEvent.wheel` pasa igual, con lo que la prueba tampoco lo
  denuncia. Engancha el evento nativo a mano con `{ passive: false }`, y en la
  prueba despacha un evento nativo `cancelable` y comprueba `defaultPrevented`.
- **Con `vi.useFakeTimers()`, `userEvent.click(...)` se cuelga** hasta agotar el
  `testTimeout`, aunque se le pasen `advanceTimers` y `delay: null`. En una
  prueba con relojes falsos, `fireEvent.click(...)`. Ejemplo real:
  `sede/SedeWindow.test.tsx`, momento 1.
