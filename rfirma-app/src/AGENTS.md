# Mapa de la interfaz (React 19 + TypeScript)

Este índice **sustituye a explorar el árbol**. Localiza el módulo por su línea,
abre **solo** ese fichero.

`src-tauri/tests/agents_map_is_complete.rs` comprueba que aquí está listado, por
su ruta, todo `.ts`/`.tsx` versionado bajo `src/` que no sea un `*.test.*`. **Un
módulo nuevo se añade a esta tabla en la misma PR que lo crea**, o el PR sale en
rojo.

## Presupuesto de lectura

- **Para saber qué puede pedirle la ventana al backend, `just contract`. No abras
  `src-tauri/src/commands/`.** Imprime las órdenes y los tipos que cruzan, con
  los nombres de campo que ve TypeScript (`holderName`, no `holder_name`), y sin
  el estado que Tauri inyecta y que nunca cruza. Se genera de las fuentes en cada
  ejecución, así que no puede quedarse obsoleto.
- **Para situarte, `just outline <ruta>`; nunca `cat` de un módulo de más de 300
  líneas.** El esqueleto trae cada `export`, cada `it(`/`describe(` y cada
  manejador interno con su número de línea y la primera línea de su bloque `/**`;
  desde ahí, `sed -n 'A,Bp;C,Dp'` con **todos** los tramos en una sola llamada —
  un turno por tramo sale más caro que haber leído el módulo entero.
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

| Módulo | Líneas | Qué es |
|---|---|---|
| `main.tsx` | 77 | **El cableado**: quién implementa cada puerto. Empieza aquí siempre. |
| `tauri.ts` | 521 | Los puertos que hablan con Tauri. La otra cara de `commands/mod.rs`. |
| `App.tsx` | 680 | El árbol de la ventana y el estado que la recorre. |
| **`shell/`** | | La ventana y su cabecera (ADR-0007). |
| `shell/MainWindow.tsx` | 84 | El marco. |
| `shell/Header.tsx` | 118 | La cabecera única, sin barra de menús. |
| `shell/menuAnchor.ts` | 25 | Dónde se ancla el menú de dos entradas. |
| **`documents/`** | | La bandeja. |
| `documents/document.ts` | 13 | El vocabulario de la bandeja. |
| `documents/useDocuments.ts` | 156 | El estado de la bandeja. |
| `documents/DocumentTray.tsx` | 97 | La bandeja pintada. |
| `documents/recents.ts` | 162 | Los diez recientes y su puerto. Misma capacidad que `memory::recents::CAPACITY`. |
| `documents/picker.ts` | 31 | Por dónde entra un documento. |
| `documents/drops.ts` | 74 | Qué ocurre al soltar ficheros encima. |
| **`signing/`** | | La firma, en el lado de la interfaz. |
| `signing/flow.ts` | 148 | Las tres etapas de la trifásica. |
| `signing/useSigning.ts` | 148 | El estado de la firma. |
| `signing/SigningPanel.tsx` | 808 | El panel. El fichero más grande de la interfaz. |
| `signing/CertificateSelect.tsx` | 266 | La elección de certificado. |
| `signing/PinDialog.tsx` | 151 | El PIN. |
| `signing/SigningProgressDialog.tsx` | 106 | El progreso. |
| `signing/SignedPanel.tsx` | 144 | El resumen tras firmar, y sus tres salidas (ID-79). |
| `signing/certificate.ts` | 121 | El certificado, en el lado de la interfaz. |
| `signing/destination.ts` | 160 | Dónde cae el firmado, el recorte de esa línea —la función pura del ID-64— y quién lleva al usuario hasta el fichero (ID-79). |
| `signing/visibleSignature.ts` | 91 | Qué se estampa en el recuadro. |
| `signing/rubric.ts` | 77 | La rúbrica que va dentro del recuadro. |
| `signing/pageRange.ts` | 102 | El conjunto de páginas tecleado (`1,2-3,10-20`) y su camino de vuelta a texto. Sin React. |
| `signing/token.ts` | 53 | Lo que el token puede contestar cuando algo va mal. |
| `signing/failure.ts` | 55 | El fallo de firma, clasificado. |
| **`viewer/`** | | El visor de PDF. |
| `viewer/DocumentViewer.tsx` | 789 | El visor, con la pastilla bajo la hoja. |
| `viewer/pdf.ts` | 88 | La frontera con `pdf.js`, escrita como puerto. |
| `viewer/pdfjsLoader.ts` | 68 | El worker de `pdf.js`, empaquetado por Vite. |
| `viewer/renderQueue.ts` | 101 | Una sola pintada viva sobre el lienzo, y el observador del tamaño que dispara la siguiente. |
| `viewer/zoom.ts` | 176 | El zoom: rango continuo, «ajustar» como modo y el tope del mapa de bits. Sin React. |
| `viewer/source.ts` | 93 | De dónde salen los bytes del documento. |
| `viewer/signatureBox.ts` | 285 | El recuadro: dónde se guarda, en qué páginas y cómo se redimensiona. |
| `viewer/useBoxDrag.ts` | 178 | El arrastre del recuadro y sus cuatro tiradores. |
| **`preferences/`** | | Los ajustes. |
| `preferences/preferences.ts` | 93 | Lo que la aplicación recuerda. |
| `preferences/PreferencesDialog.tsx` | 401 | La pantalla completa de ajustes, con su índice de secciones. |
| `preferences/Switch.tsx` | 62 | El interruptor. |
| `preferences/Select.tsx` | 189 | El desplegable. |
| `preferences/theme.ts` | 40 | El tema de la ventana. |
| **`i18n/`** | | Catálogo propio, cinco idiomas, generado desde `po/` (ADR-0009 enmendado). Los dieciséis bloques de comentario que explican el mecanismo —no para quien traduce— están indexados en `i18n/AGENTS.md`. |
| `i18n/catalog.ts` | 28 | La forma del catálogo. |
| `i18n/i18n.ts` | 40 | La traducción. |
| `i18n/LanguageProvider.tsx` | 81 | El contexto. |
| `i18n/languages.ts` | 30 | Los idiomas publicados: reexporta lo que generó `po-import`. |
| `i18n/i18next.d.ts` | 23 | Las claves, para `tsc` y el editor. Se versiona; `resources.d.ts` no. |
| `i18n/preference.ts` | 34 | De dónde sale y a dónde vuelve el idioma. |
| `i18n/locales/*.ts` | — | **Generados, no versionados.** Salen de `po/`. No se leen ni se editan. |
| **`errors/`** | | Los fallos que ve el usuario. |
| `errors/classify.ts` | 57 | Un fallo con la forma del ID-29: una situación, no un mensaje. |
| `errors/ErrorNotice.tsx` | 55 | El aviso. |
| **`design-system/`** | | `design-system/icons.tsx` (211), copiados en línea de los artboards. |
| **`about/`** | | `about/AboutDialog.tsx` (78). |
| **Andamiaje** | | `test-setup.ts` (10), `testing/render.tsx` (25), `vite-env.d.ts` (5). No son la aplicación. |

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
