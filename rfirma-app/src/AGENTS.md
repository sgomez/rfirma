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
  los nombres de campo que ve TypeScript (`holderName`, no `holder_name`). Se
  genera de las fuentes en cada ejecución, así que no puede quedarse obsoleto.
- **Para situarte, `just outline <ruta>`; nunca `cat` de un módulo de más de 300
  líneas.** Desde el esqueleto, `sed -n 'A,Bp;C,Dp'` con **todos** los tramos en
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

| Módulo | Qué es |
|---|---|
| `main.tsx` | **El cableado de la ventana principal** (`index.html`): quién implementa cada puerto. Empieza aquí siempre. |
| `tauri.ts` | El punto de import de los puertos que hablan con Tauri: reexporta los `tauri*.ts` de abajo. La otra cara de los `adapters/tauri.rs` de cada contexto. |
| `tauriStage.ts` | Clasifica el fallo de una orden de Tauri y envuelve su llamada. Lo comparten `tauriSigning.ts` y `tauriSede.ts`. |
| `tauriSigning.ts` | Los puertos de Tauri de la firma: certificados, las tres etapas, la rúbrica y el sello. |
| `tauriDocuments.ts` | Los puertos de Tauri del documento: el portal, el arrastre, los recientes y el visor. |
| `tauriPreferences.ts` | Los puertos de Tauri de la configuración: ajustes, idioma, destino y la versión publicada. |
| `tauriSede.ts` | El puerto de Tauri del trámite de sede. |
| `tauriStatus.ts` | El puerto de Tauri del estado de la instalación. |
| `App.tsx` | El árbol de la ventana y el estado que la recorre. |
| `App.forgetActivity.ts` | Olvidar la actividad: los ajustes y los documentos abiertos, aunque uno de los dos falle. Sin React. |
| `App.signingOrder.ts` | La colocación guardada, la geometría de la página y la orden de firma armada en un solo sitio. Sin React. |
| `App.useCertificateSearch.ts` | Buscar certificados, instalar y quitar `.p12`, y elegir uno del desplegable. |
| `App.useDropNotices.ts` | El arrastre sobre la ventana y la invocación externa, y el aviso que dejan. |
| `App.usePlacementControls.ts` | La colocación de la firma visible: el recuadro y los tres modos de página. |
| `App.usePreferencesState.ts` | Los ajustes, el destino previsto para el documento activo y la rúbrica adoptada. |
| `App.usePreviousSignatures.ts` | El informe de firmas previas del documento activo, pedido al abrir o cargar. |
| `App.useSignFlow.ts` | La vista previa del sello y la firma, con los dos avisos que pueden interponerse antes del PIN. |
| `App.useSignedSummary.ts` | El acuse de recibo del documento firmado y los dos caminos hasta el fichero. |
| `App.useSigningFailure.ts` | El error de firma del documento activo, y su salida al cambiar de pestaña. |
| `App.useStartupNotices.ts` | El aviso de versión nueva y las filas del panel de estado, medidos al arrancar. |
| `App.useVisibleSignature.ts` | La firma visible, apagada mientras no hay certificado elegido. |
| `App.testSupport.tsx` | Los dobles y el `renderApp` que comparten las pruebas de `App`. |
| **`shell/`** | La ventana y su cabecera (ADR-0007). |
| `shell/MainWindow.tsx` | El marco, con el hueco de la franja entre la cabecera y las regiones. |
| `shell/NotificationStrip.tsx` | La franja de notificación: el patrón, no el aviso concreto. |
| `shell/Header.tsx` | La cabecera única, sin barra de menús. |
| `shell/menuAnchor.ts` | Dónde se ancla el menú de la aplicación. |
| **`documents/`** | Los documentos abiertos y los recientes. |
| `documents/document.ts` | El vocabulario del documento: el que se tiene delante y su insignia. No es la fila. |
| `documents/useDocuments.ts` | El estado de las pestañas abiertas, la activa y los recientes. |
| `documents/DocumentTabs.tsx` | La tira de pestañas bajo la cabecera, con su menú «+». |
| `documents/RecentRows.tsx` | Las filas de los recientes, que comparten el menú «+» y el estado vacío del visor. |
| `documents/recents.ts` | Los diez recientes —**la fila que se guarda**— y su puerto. |
| `documents/picker.ts` | Por dónde entra un documento. |
| `documents/drops.ts` | Qué ocurre al soltar ficheros encima, y el documento con el que se invocó a la aplicación desde fuera. |
| **`signing/`** | La firma, en el lado de la interfaz. |
| `signing/flow.ts` | Las tres etapas de la trifásica. |
| `signing/useSigning.ts` | El estado de la firma. |
| `signing/SigningPanel.tsx` | El panel, con la zona que se desliza y el pie fijo. |
| `signing/SigningPanel.testSupport.tsx` | Los dobles y ayudas que comparten las pruebas de `SigningPanel`. |
| `signing/CertificateNotice.tsx` | El aviso de «sin certificados», arriba de la zona que se desliza. |
| `signing/PreviousSignaturesNotice.tsx` | El aviso de firmas previas: línea plegable con quién firmó y cuándo. |
| `signing/previousSignatures.ts` | El vocabulario de una firma previa y su informe. Sin React. |
| `signing/PlacementFieldset.tsx` | El segmentado de páginas de la firma visible —«Una página», «Varias», «Todas»— y la línea o el campo de debajo. |
| `signing/usePlacementField.ts` | Lo tecleado en el campo de «Varias» y qué botón toca bajo el segmentado. |
| `signing/placementField.ts` | Las situaciones del campo de páginas, redactadas. Sin React. |
| `signing/ModelFieldset.tsx` | Qué se estampa en el recuadro: las tarjetas de modelo y la rúbrica. |
| `signing/PhraseEditor.tsx` | La frase de *Personalizada*: el campo con las pastillas de los datos y su menú «+ Dato». |
| `signing/phraseDom.ts` | Leer la frase del campo editable y escribirla en él, y dónde está el cursor respecto a una pastilla. No es el componente. |
| `signing/PanelFooter.tsx` | El pie fijo del panel: el destino y, según el estado, el certificado, «Reintentar»/«Volver» o el aviso de sin certificados. |
| `signing/CertificateFooterButton.tsx` | El botón partido «Firmar como…» del pie, con su lista hacia arriba. |
| `signing/panelFormat.ts` | El tamaño del documento en la unidad que reconoce el usuario. Sin React. |
| `signing/CertificateSelect.tsx` | La elección de certificado, reutilizada tal cual en la sede. |
| `signing/secret.ts` | Cómo hay que pedirle el secreto al almacén: sin sesión, tecleado en pantalla, o en el teclado del lector. Sin React. |
| `signing/SigningProgressDialog.tsx` | El progreso. |
| `signing/UnsealedPagesDialog.tsx` | El diálogo de páginas sin sello, justo antes de firmar. |
| `signing/UnregisteredSignaturesDialog.tsx` | El aviso de las firmas previas que rFirma no sabe leer, en la misma fila que el anterior. |
| `signing/unsealedPages.ts` | Qué páginas del conjunto elegido se quedan sin sello. Sin React. |
| `signing/SignedPanel.tsx` | El resumen tras firmar, y sus tres salidas. |
| `signing/certificate.ts` | El certificado, en el lado de la interfaz, con el orden y el agrupado del desplegable. |
| `signing/destination.ts` | Dónde cae el firmado, el recorte de esa línea y quién lleva al usuario hasta el fichero. |
| `signing/visibleSignature.ts` | Qué se estampa en el recuadro. |
| `signing/rubric.ts` | La rúbrica que va dentro del recuadro. |
| `signing/pageRange.ts` | El conjunto de páginas tecleado (`1,2-3,10-20`) y su camino de vuelta a texto. Sin React. |
| `signing/token.ts` | Lo que el token puede contestar cuando algo va mal. |
| `signing/failure.ts` | El fallo de firma, clasificado. |
| `signing/stampPreview.ts` | El sello que se ve dentro del recuadro antes de firmar: su puerto, sus estados y el umbral del documento grande. Sin React. |
| `signing/useStampPreview.ts` | Cuándo se compone el sello y qué se enseña mientras tanto. Su trabajo es **no** componer. |
| **`viewer/`** | El visor de PDF. |
| `viewer/DocumentViewer.tsx` | El JSX del visor: la hoja, el recuadro y la botonera. |
| `viewer/StampPill.tsx` | La pastilla flotante del estado del sello. |
| `viewer/useViewerPage.ts` | La pintada sobre el `<canvas>`, el recorrido de páginas y el zoom. |
| `viewer/useViewerBox.ts` | Los tres gestos del recuadro de la firma visible: arrastrar, redimensionar y trazar. |
| `viewer/pdf.ts` | La frontera con `pdf.js`, escrita como puerto. |
| `viewer/pdfjsLoader.ts` | El worker de `pdf.js`, empaquetado por Vite. |
| `viewer/renderQueue.ts` | Una sola pintada viva sobre el lienzo, y el observador del tamaño que dispara la siguiente. |
| `viewer/zoom.ts` | El zoom: rango continuo, «ajustar» como modo y el tope del mapa de bits. Sin React. |
| `viewer/source.ts` | De dónde salen los bytes del documento. |
| `viewer/signatureBox.ts` | El recuadro: dónde se guarda, cómo se redimensiona y cómo se traza. |
| `viewer/useBoxDrag.ts` | **Arrastrar** el recuadro que ya existe, y redimensionarlo por sus cuatro tiradores. |
| `viewer/useBoxTrace.ts` | **Trazar** el recuadro sobre la hoja: el gesto que lo hace nacer. Hermano del anterior, no un modo suyo. |
| `viewer/testing/documentViewerFixtures.ts` | El doble de `pdf.js` y los atajos de consulta del DOM que comparten las pruebas de `DocumentViewer*.test.tsx`. |
| **`status/`** | El estado de la instalación. |
| `status/status.ts` | El puerto de consulta y medición de estado, con su doble en memoria y el nombre en pantalla de cada sitio. Sin React. |
| `status/StatusView.tsx` | La vista del cuerpo con el estado de rFirma. |
| `status/WithdrawCertificateDialog.tsx` | El velo que confirma, ejecuta y cuenta la retirada del certificado de rFirma (docs/design/retirar-certificado.md). |
| **`preferences/`** | Los ajustes. |
| `preferences/preferences.ts` | Lo que la aplicación recuerda. |
| `preferences/PreferencesView.tsx` | La vista del cuerpo con los ajustes, con su índice de **cuatro** secciones. |
| `preferences/PreferencesSections.tsx` | El contenido de cada sección del índice, como componentes propios. |
| `preferences/PasswordPrompt.tsx` | El diálogo de la contraseña del `.p12`, antes de elegir el fichero. |
| `preferences/focusTrap.ts` | El tabulador que da la vuelta dentro de un modal. |
| `preferences/Switch.tsx` | El interruptor. |
| `preferences/Select.tsx` | El desplegable. |
| `preferences/theme.ts` | El tema de la ventana. |
| `preferences/testSupport.tsx` | Los dobles y ayudas que comparten sus pruebas. |
| **`i18n/`** | Catálogo propio, cinco idiomas, generado desde `po/` (ADR-0009 enmendado). Los bloques de comentario que explican el mecanismo están indexados en `i18n/AGENTS.md`. |
| `i18n/catalog.ts` | La forma del catálogo. |
| `i18n/i18n.ts` | La traducción. |
| `i18n/LanguageProvider.tsx` | El contexto. |
| `i18n/languages.ts` | Los idiomas publicados: reexporta lo que generó `po-import`. |
| `i18n/i18next.d.ts` | Las claves, para `tsc` y el editor. Se versiona; `resources.d.ts` no. |
| `i18n/preference.ts` | De dónde sale y a dónde vuelve el idioma. |
| `i18n/locales/*.ts` | **Generados, no versionados.** Salen de `po/`. No se leen ni se editan. |
| **`errors/`** | Los fallos que ve el usuario. |
| `errors/classify.ts` | El fallo como situación, no como mensaje. |
| `errors/ErrorNotice.tsx` | El aviso. |
| `errors/RenderErrorBoundary.tsx` | El *error boundary* de cada ventana: si un hijo lanza al pintarse, esto es lo que se ve en su lugar. |
| **`design-system/`** | `design-system/icons.tsx`, copiados en línea de los artboards. |
| **`desktop/`** | El escritorio de la persona, en el lado de la interfaz. |
| `desktop/externalDestination.ts` | El puerto que abre destinos web externos en el navegador, y su doble. Sin React. |
| **`sede/`** | **La ventana que abre una sede** por `afirma://`: una ventana con una secuencia de momentos, no una pantalla por momento. Ficha: `docs/design/ventana-de-sede.md`. |
| `sede/main.tsx` | **El cableado de la ventana de sede** (`sede.html`): su propio montaje, sin nada del árbol de la principal. |
| `sede/siteErrands.ts` | El adaptador del puerto: la suscripción y el recorrido de consentir, confirmar, firmar y cancelar. Sin React y sin Tauri. |
| `sede/siteErrandView.ts` | Los tipos de lo que empuja el backend (`SiteErrandView`, `SiteStageView`) y de cómo acaban sus órdenes. Sin React. |
| `sede/errandConversion.ts` | La conversión pura del momento del backend al `Errand` de la ventana, y el catálogo de rechazos. Sin React. |
| `sede/siteErrandsFixtures.ts` | Los dobles de `SiteCommands` y los momentos de ejemplo que comparten las pruebas de `siteErrands`. |
| `sede/errand.ts` | El vocabulario del trámite y su puerto `SiteErrandPort`, con el doble `noErrand` y los relojes. Sin React. |
| `sede/SedeWindow.tsx` | El marco de 520 × 420 px y el reparto entre los momentos. |
| `sede/SedeMarking.tsx` | 1c · El área de la firma visible trazada sobre el PDF con el visor de la ventana principal, antes del consentimiento. |
| `sede/sedeWindowFixtures.ts` | Los dobles de `SiteErrandPort` y los momentos de ejemplo que comparten las pruebas de `SedeWindow`. |
| `sede/SedeFrame.tsx` | Cuerpo y pie —56 px clavados en firma y salida— y los relojes de la ventana en forma de `hook`. |
| `sede/SedeWaiting.tsx` | 1 · La espera y las dos recetas de navegador, que **no diagnostican**. |
| `sede/SedeConsent.tsx` | 2 · La confirmación escrita, con el desplegable de `signing/CertificateSelect.tsx` reutilizado tal cual. |
| `sede/SedeConfirm.tsx` | 2b · La pregunta que el validador del original obliga a hacer, con las palabras del original y sus dos salidas. |
| `sede/SedeSigning.tsx` | 3 · Los dos tramos de la firma, sin nombrar ninguna fase del motor. |
| `sede/SedeTransfer.tsx` | El fichero que la sede quiere guardar o cargar, mientras el diálogo del portal está encima. No tiene acciones propias. |
| `sede/SedeOutcome.tsx` | 4 · Firmado, lote entregado, cancelado, guardado, cargado y rechazado, con el documento recién firmado y el detalle copiable del rechazo. |
| `sede/SedeOldWebClient.tsx` | El aviso de que la página usa un cliente web antiguo, que no detiene el trámite. |
| `sede/SedeNoCertificate.tsx` | 5 · Sin certificado utilizable, y sus dos salidas distintas. |
| **`updates/`** | `updates/newVersion.ts`: el puerto que pregunta si hay versión nueva, y su doble. Sin React. |
| **`about/`** | `about/AboutDialog.tsx`. |
| **`setup/`** | El asistente del primer arranque (docs/design/primer-arranque.md). Usa los casos de uso del panel de estado, no tiene los suyos propios. |
| `setup/SetupWizard.tsx` | Las dos pantallas, montadas en `main.tsx` mientras `Preferences.setupWizardSeen` siga en `false`. Sustituye al antiguo `trust/TrustNotice.tsx`. |
| **Andamiaje** | `test-setup.ts`, `testing/render.tsx`, `vite-env.d.ts`. No son la aplicación. |

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
