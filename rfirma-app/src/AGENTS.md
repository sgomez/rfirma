# Mapa de la interfaz (React 19 + TypeScript)

Este índice **sustituye a explorar el árbol**. Localiza el módulo por su línea,
abre **solo** ese fichero.

`src-tauri/tests/agents_map_is_complete.rs` comprueba que aquí está listado, por
su ruta, todo `.ts`/`.tsx` versionado bajo `src/` que no sea un `*.test.*`. **Un
módulo nuevo se añade a esta tabla en la misma PR que lo crea**, o el PR sale en
rojo.

## Presupuesto de lectura

- **Nunca `cat` de un fichero de más de 300 líneas.** `grep -n` para situarte,
  `sed -n 'A,Bp'` para el tramo.
- Los tests viven en `*.test.ts(x)` **al lado** del módulo. No los abras salvo
  que vayas a tocarlos; `grep -n "it(\|describe(" <fichero>.test.tsx` dice qué
  cubren en una línea por caso.
- **Los seis catálogos de `i18n/locales/` son ~350 líneas cada uno y no se leen
  nunca enteros.** `es.ts` es la forma del catálogo; para una clave concreta,
  `grep -n '<clave>' i18n/locales/es.ts`. Al añadir una clave hay que añadirla a
  los seis (ADR-0009), y eso se hace con una edición puntual, no leyéndolos.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `main.tsx` | 77 | **El cableado**: quién implementa cada puerto. Empieza aquí siempre. |
| `tauri.ts` | 418 | Los puertos que hablan con Tauri. La otra cara de `commands/mod.rs`. |
| `App.tsx` | 532 | El árbol de la ventana y el estado que la recorre. |
| **`shell/`** | | La ventana y su cabecera (ADR-0007). |
| `shell/MainWindow.tsx` | 84 | El marco. |
| `shell/Header.tsx` | 118 | La cabecera única, sin barra de menús. |
| `shell/menuAnchor.ts` | 25 | Dónde se ancla el menú de dos entradas. |
| **`documents/`** | | La bandeja. |
| `documents/document.ts` | 13 | El vocabulario de la bandeja. |
| `documents/useDocuments.ts` | 131 | El estado de la bandeja. |
| `documents/DocumentTray.tsx` | 97 | La bandeja pintada. |
| `documents/recents.ts` | 162 | Los diez recientes y su puerto. Misma capacidad que `memory::recents::CAPACITY`. |
| `documents/picker.ts` | 31 | Por dónde entra un documento. |
| `documents/drops.ts` | 74 | Qué ocurre al soltar ficheros encima. |
| **`signing/`** | | La firma, en el lado de la interfaz. |
| `signing/flow.ts` | 132 | Las tres etapas de la trifásica. |
| `signing/useSigning.ts` | 148 | El estado de la firma. |
| `signing/SigningPanel.tsx` | 539 | El panel. El fichero más grande de la interfaz. |
| `signing/CertificateSelect.tsx` | 266 | La elección de certificado. |
| `signing/PinDialog.tsx` | 151 | El PIN. |
| `signing/SigningProgressDialog.tsx` | 106 | El progreso. |
| `signing/SignedPanel.tsx` | 78 | Lo que se ve al terminar. |
| `signing/certificate.ts` | 121 | El certificado, en el lado de la interfaz. |
| `signing/visibleSignature.ts` | 91 | Qué se estampa en el recuadro. |
| `signing/rubric.ts` | 72 | La rúbrica que va dentro del recuadro. |
| `signing/token.ts` | 53 | Lo que el token puede contestar cuando algo va mal. |
| `signing/failure.ts` | 55 | El fallo de firma, clasificado. |
| **`viewer/`** | | El visor de PDF. |
| `viewer/DocumentViewer.tsx` | 377 | El visor. |
| `viewer/pdf.ts` | 88 | La frontera con `pdf.js`, escrita como puerto. |
| `viewer/pdfjsLoader.ts` | 68 | El worker de `pdf.js`, empaquetado por Vite. |
| `viewer/renderQueue.ts` | 63 | Una sola pintada viva sobre el lienzo. |
| `viewer/source.ts` | 93 | De dónde salen los bytes del documento. |
| `viewer/signatureBox.ts` | 120 | El recuadro: dónde se guarda y cómo se pinta. |
| `viewer/useBoxDrag.ts` | 122 | El arrastre del recuadro. |
| **`preferences/`** | | Los ajustes. |
| `preferences/preferences.ts` | 75 | Lo que la aplicación recuerda. |
| `preferences/PreferencesDialog.tsx` | 174 | El diálogo. |
| `preferences/Switch.tsx` | 62 | El interruptor. |
| `preferences/Select.tsx` | 189 | El desplegable. |
| `preferences/theme.ts` | 40 | El tema de la ventana. |
| **`i18n/`** | | Catálogo propio, seis idiomas (ADR-0009). |
| `i18n/catalog.ts` | 50 | La forma del catálogo. |
| `i18n/i18n.ts` | 40 | La traducción. |
| `i18n/LanguageProvider.tsx` | 62 | El contexto. |
| `i18n/languages.ts` | 45 | Los seis idiomas. |
| `i18n/preference.ts` | 34 | De dónde sale y a dónde vuelve el idioma. |
| `i18n/locales/es.ts` | 425 | Castellano: además de catálogo, **la forma** del catálogo. |
| `i18n/locales/en.ts` | 345 | Inglés, el otro con contenido en v0.1. |
| `i18n/locales/ca.ts` `i18n/locales/eu.ts` `i18n/locales/gl.ts` `i18n/locales/va.ts` | ~348 | Claves sí, textos no. No se leen. |
| **`errors/`** | | Los fallos que ve el usuario. |
| `errors/classify.ts` | 57 | Un fallo con la forma del ID-29: una situación, no un mensaje. |
| `errors/ErrorNotice.tsx` | 55 | El aviso. |
| **`design-system/`** | | `design-system/icons.tsx` (201), copiados en línea de los artboards. |
| **`about/`** | | `about/AboutDialog.tsx` (78). |
| **Andamiaje** | | `test-setup.ts` (10), `testing/render.tsx` (25), `vite-env.d.ts` (5). No son la aplicación. |

## La regla del puerto

La ventana no habla con Tauri: habla con puertos declarados en su propio
módulo, y `main.tsx` elige la implementación (ADR-0017). Si vas a añadir
capacidad nueva, el orden es: el puerto en su módulo de dominio → `tauri.ts` →
`main.tsx`. Las fichas de pantalla viven en `docs/design/` (ver
`docs/AGENTS.md`).
