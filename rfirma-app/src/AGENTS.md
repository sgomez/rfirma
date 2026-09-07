# Mapa de la interfaz (React 19 + TypeScript)

Este índice **sustituye a explorar el árbol**. Localiza el módulo por su línea,
abre **solo** ese fichero.

`src-tauri/tests/agents_map_is_complete.rs` comprueba que aquí está listado, por
su ruta, todo `.ts`/`.tsx` versionado bajo `src/` que no sea un `*.test.*`. **Un
módulo nuevo se añade a esta tabla en la misma PR que lo crea**, o el PR sale en
rojo.

## Presupuesto de lectura

- **Para saber qué puede pedirle la ventana al backend, `just contract`. No abras
  `src-tauri/src/<contexto>/adapters/`.** Imprime las órdenes y los tipos que cruzan, con
  los nombres de campo que ve TypeScript (`holderName`, no `holder_name`), y sin
  el estado que Tauri inyecta y que nunca cruza. Se genera de las fuentes en cada
  ejecución, así que no puede quedarse obsoleto.
- **Para situarte, `just outline <ruta>`; nunca `cat` de un módulo de más de 300
  líneas.** El esqueleto trae cada `export`, cada `it(`/`describe(` y cada
  manejador interno con su número de línea y la primera línea de su bloque `/**`;
  desde ahí, `sed -n 'A,Bp;C,Dp'` con **todos** los tramos en una sola llamada —
  un turno por tramo sale más caro que haber leído el módulo entero.
- **El typecheck es `tsc -b` (o `just check-ts`), nunca `tsc --noEmit`.** El
  `tsconfig.json` de la raíz de `rfirma-app/` es `{"files": [], "references":
  [...]}`: `pnpm exec tsc --noEmit` sale en verde **sin mirar un solo fichero**.
  `vitest` tampoco compila los `*.test.ts`, así que un campo nuevo en un tipo
  compartido que solo usan las pruebas pasa los dos y cae en el CI.
- Los tests viven en `*.test.ts(x)` **al lado** del módulo. No los abras salvo
  que vayas a tocarlos; `grep -n "it(\|describe(" <fichero>.test.tsx` dice qué
  cubren en una línea por caso.
- **`i18n/locales/` NO ESTÁ EN EL REPOSITORIO.** Los catálogos los genera
  `tools/po-import.mjs` desde `rfirma-app/po/` en cada `just build` (ID-121). La
  fuente de verdad de una cadena es `po/messages.pot` y los cinco `po/*.po`;
  para una clave concreta, `grep -n '<clave>' po/es.po`. **Una cadena nueva se
  escribe en el `.pot`, se corre `just po`, y entonces se compila.** Editar un
  `.ts` de `locales/` no sirve de nada: se sobrescribe.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `main.tsx` | **El cableado de la ventana principal** (`index.html`): quién implementa cada puerto. Empieza aquí siempre. |
| `tauri.ts` | Los puertos que hablan con Tauri. La otra cara de los `adapters/tauri.rs` de cada contexto. |
| `App.tsx` | El árbol de la ventana y el estado que la recorre. |
| **`shell/`** | La ventana y su cabecera (ADR-0007). |
| `shell/MainWindow.tsx` | El marco, con el hueco de la franja entre la cabecera y las regiones. |
| `shell/NotificationStrip.tsx` | La franja de notificación: el patrón, no el aviso concreto (ID-207). |
| `shell/Header.tsx` | La cabecera única, sin barra de menús. |
| `shell/menuAnchor.ts` | Dónde se ancla el menú de dos entradas. |
| **`documents/`** | La bandeja. |
| `documents/document.ts` | El vocabulario del documento: **el que se tiene delante** (`DocumentInHand`) y las insignias. No es la fila (ID-287). |
| `documents/useDocuments.ts` | El estado de la bandeja. |
| `documents/DocumentTray.tsx` | La bandeja pintada. |
| `documents/recents.ts` | Los diez recientes —**la fila que se guarda**— y su puerto, con el camino de vuelta a la mano (`taken`). Misma capacidad que `memory::recents::CAPACITY`. |
| `documents/picker.ts` | Por dónde entra un documento. |
| `documents/drops.ts` | Qué ocurre al soltar ficheros encima, y el documento con el que se invocó a la aplicación desde fuera (ID-157). |
| **`signing/`** | La firma, en el lado de la interfaz. |
| `signing/flow.ts` | Las tres etapas de la trifásica. |
| `signing/useSigning.ts` | El estado de la firma. |
| `signing/SigningPanel.tsx` | El panel. |
| `signing/CertificateSelect.tsx` | La elección de certificado. |
| `signing/PinDialog.tsx` | El diálogo del secreto del almacén. |
| `signing/secret.ts` | Cómo hay que pedirle el secreto al almacén: sin sesión, tecleado en pantalla, o en el teclado del lector. Sin React. |
| `signing/SigningProgressDialog.tsx` | El progreso. |
| `signing/UnsealedPagesDialog.tsx` | El diálogo de páginas sin sello, justo antes de firmar (ID-105, ID-106). |
| `signing/UnregisteredSignaturesDialog.tsx` | El aviso de las firmas previas que rFirma no sabe leer, en la misma fila que el anterior (ID-297…ID-301, ID-305). |
| `signing/unsealedPages.ts` | Qué páginas del conjunto elegido pierde `correctPositionSignature` en silencio, en puntos PAdES. Sin React. |
| `signing/SignedPanel.tsx` | El resumen tras firmar, y sus tres salidas (ID-79). |
| `signing/certificate.ts` | El certificado, en el lado de la interfaz, con el orden y el agrupado del desplegable. |
| `signing/destination.ts` | Dónde cae el firmado, el recorte de esa línea —la función pura del ID-64— y quién lleva al usuario hasta el fichero (ID-79). |
| `signing/visibleSignature.ts` | Qué se estampa en el recuadro. |
| `signing/rubric.ts` | La rúbrica que va dentro del recuadro. |
| `signing/pageRange.ts` | El conjunto de páginas tecleado (`1,2-3,10-20`) y su camino de vuelta a texto. Sin React. |
| `signing/token.ts` | Lo que el token puede contestar cuando algo va mal. |
| `signing/failure.ts` | El fallo de firma, clasificado. |
| `signing/stampPreview.ts` | El sello que se ve dentro del recuadro antes de firmar: el puerto del ciclo en seco, sus estados y el umbral del documento grande. Sin React. |
| `signing/useStampPreview.ts` | Cuándo se compone el sello y qué se enseña mientras tanto. Su trabajo es **no** componer. |
| **`viewer/`** | El visor de PDF. |
| `viewer/DocumentViewer.tsx` | El visor y los tres gestos del recuadro. El botón de sellar vive en `signing/SigningPanel.tsx` desde #194; el estado del sello, en su propia pastilla flotante, desde #202. |
| `viewer/pdf.ts` | La frontera con `pdf.js`, escrita como puerto. El `/Title` de los metadatos es opcional: lo mira la ventana de sede, no el visor. |
| `viewer/pdfjsLoader.ts` | El worker de `pdf.js`, empaquetado por Vite. |
| `viewer/renderQueue.ts` | Una sola pintada viva sobre el lienzo, y el observador del tamaño que dispara la siguiente. |
| `viewer/zoom.ts` | El zoom: rango continuo, «ajustar» como modo y el tope del mapa de bits. Sin React. |
| `viewer/source.ts` | De dónde salen los bytes del documento. |
| `viewer/signatureBox.ts` | El recuadro: dónde se guarda, **el conjunto propio de cada opción** (#188), cómo se redimensiona y cómo se traza. |
| `viewer/useBoxDrag.ts` | **Arrastrar** el recuadro que ya existe, y redimensionarlo por sus cuatro tiradores. |
| `viewer/useBoxTrace.ts` | **Trazar** el recuadro sobre la hoja: el gesto que lo hace nacer (#190). Hermano del anterior, no un modo suyo. |
| **`preferences/`** | Los ajustes. |
| `preferences/preferences.ts` | Lo que la aplicación recuerda. |
| `preferences/PreferencesDialog.tsx` | La pantalla completa de ajustes, con su índice de **cinco** secciones. |
| `preferences/Switch.tsx` | El interruptor. |
| `preferences/Select.tsx` | El desplegable. |
| `preferences/theme.ts` | El tema de la ventana. |
| **`i18n/`** | Catálogo propio, cinco idiomas, generado desde `po/` (ADR-0009 enmendado). Los dieciséis bloques de comentario que explican el mecanismo —no para quien traduce— están indexados en `i18n/AGENTS.md`. |
| `i18n/catalog.ts` | La forma del catálogo. |
| `i18n/i18n.ts` | La traducción. |
| `i18n/LanguageProvider.tsx` | El contexto. |
| `i18n/languages.ts` | Los idiomas publicados: reexporta lo que generó `po-import`. |
| `i18n/i18next.d.ts` | Las claves, para `tsc` y el editor. Se versiona; `resources.d.ts` no. |
| `i18n/preference.ts` | De dónde sale y a dónde vuelve el idioma. |
| `i18n/locales/*.ts` | **Generados, no versionados.** Salen de `po/`. No se leen ni se editan. |
| **`errors/`** | Los fallos que ve el usuario. |
| `errors/classify.ts` | Un fallo con la forma del ID-29: una situación, no un mensaje. |
| `errors/ErrorNotice.tsx` | El aviso. |
| **`design-system/`** | `design-system/icons.tsx`, copiados en línea de los artboards. |
| **`desktop/`** | El escritorio de la persona, en el lado de la interfaz: quién atiende `afirma://` (ID-238…ID-241). |
| `desktop/urlHandlers.ts` | El puerto que pregunta y elige quién atiende los enlaces, su doble, y las dos reglas puras: si ya los atiende rFirma y si el banner tiene algo que preguntar. Sin React. |
| `desktop/UrlHandlerBanner.tsx` | El banner del arranque, con sus tres respuestas. Ocupa el mismo hueco que la franja de notificación y le cede el sitio. |
| **`sede/`** | **La ventana que abre una sede** por `afirma://` (#362): una ventana con una secuencia de cinco momentos, no cinco pantallas. Ficha: `docs/design/ventana-de-sede.md`. |
| `sede/main.tsx` | **El cableado de la ventana de sede** (`sede.html`, ID-335): su propio montaje, con el puerto de verdad y sin nada del árbol de la principal. |
| `sede/siteErrands.ts` | El adaptador del puerto: convierte cada momento que empuja el backend en el `Errand` que espera la ventana, y pone los dos que no vienen de él —el secreto y los dos tramos de la firma—. Sin React y sin Tauri (TD-78). |
| `sede/errand.ts` | El vocabulario del trámite y su puerto `SiteErrandPort`, con el doble `noErrand`, los tres relojes (retardo, umbral, cierre a los 15 s) y el callejón sin salida que llega ya medido del backend (ID-341). Sin React. |
| `sede/SedeWindow.tsx` | El marco de 520 × 420 px y el reparto entre los cinco momentos. El PIN se monta encima, sin pantalla propia (ID-273), y el canal que ya no se va a abrir reusa la pantalla de la espera sin esperar al reloj (ID-341). |
| `sede/SedeFrame.tsx` | Cuerpo y pie —56 px clavados en firma y salida— y los dos relojes en forma de `hook`. |
| `sede/SedeWaiting.tsx` | 1 · La espera y las dos recetas de navegador, que **no diagnostican**. |
| `sede/SedeConsent.tsx` | 2 · La confirmación escrita, con el desplegable de `signing/CertificateSelect.tsx` reutilizado tal cual (ID-269), y la quinta situación —firma no reconocida (#363)—. |
| `sede/SedeSigning.tsx` | 3 · Los dos tramos de la firma, sin nombrar ninguna fase del motor. |
| `sede/SedeOutcome.tsx` | 4 · Los tres desenlaces, con el documento que se acaba de firmar, y el detalle copiable del rechazo. |
| `sede/SedeNoCertificate.tsx` | 5 · Sin certificado utilizable, y sus dos salidas distintas (ID-278). |
| **`updates/`** | `updates/newVersion.ts`: el puerto que pregunta si hay versión nueva, y su doble. Sin React. |
| **`about/`** | `about/AboutDialog.tsx`. |
| **`trust/`** | El aviso del primer arranque (#365): la CA local y el permiso de red local, explicados juntos y sin condición. No es un puerto, no habla con Tauri. |
| `trust/TrustNotice.tsx` | El diálogo del primer arranque, montado en `main.tsx` mientras `Preferences.trustNoticeSeen` siga en `false`. |
| **Andamiaje** | `test-setup.ts`, `testing/render.tsx`, `vite-env.d.ts`. No son la aplicación. |

## El circuito de cadenas (ADR-0009 enmendado, ID-121…ID-130)

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
sus dos puntos ciegos. **Una clave ensamblada con plantilla** (`t(\`a.b.${x}\`)`)
no la ve ninguna: las dos pasan en verde aunque la clave resultante no exista en
el catálogo, y el hueco solo aparece en pantalla — escribe siempre la clave
entera. Y **el extractor lee también los comentarios**, así que un `t()` de
ejemplo dentro de un bloque `/** */` cuenta como clave usada: puede poner
`extract --ci` en rojo o falsear el recuento de claves sin uso.

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
