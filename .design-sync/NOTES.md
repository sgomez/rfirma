# design-sync: notas de rFirma

Proyecto: «rFirma Components» (`312bca0c-2f94-494a-820a-e947e03f9ade`), forma `storybook`. Spec: #1483 (fase 0, #1486).

## Forma del repositorio

- rfirma-app es una aplicación, no una biblioteca: no hay `dist/`. Lo que se compila lo dice `rfirma-app/design-sync.entry.ts` (`cfg.entry` y `cfg.extraEntries`). Un componente nuevo para Claude Design se exporta ahí y lleva su historia.
- El escaneo de exportaciones solo ve los nombres a través de `extraEntries`: con `entry` a secas sale «0/1 storybook components are public exports».
- `titleMap` usa el último segmento del título sin espacios (`1·Espera`), no el título entero.

## Arreglos

- [GENERAL] El decorador de `.storybook/preview.tsx` no se empaqueta: importa el CSS del sistema de diseño, que trae `.woff2`, y el empaquetador de decoradores no tiene cargador para fuentes. Por eso `cfg.provider` es `DesignRoot`, la misma raíz que usa el decorador.
- [GENERAL] Ni los tokens ni las clases `rf-*` llegaban a `_ds_bundle.css`. El convertidor solo toma el CSS que importan los componentes de la entrada, y `index.css` lo importa el preview de Storybook, no un componente. Ahora la entrada importa `index.css` y `app.css`. `cfg.tokensGlob` no sirve sin `cfg.tokensPkg`.
- [GENERAL] La ventana de sede usa `position: fixed; inset: 0` y en Storybook medía 0 px (`sb-error`: «no storybook root content»). Las historias de sede llevan el decorador `inSedeWindow` (520 × 420, el `DIALOG_SIZE` de Tauri), y `SedeView` va con `cardMode: "single"`.

- [GENERAL] Las piezas de dominio que exporta la entrada solo llegan con su `titleMap`: sin él salen como `[TITLE_UNMAPPED]` y se descartan. `Ventana principal/4 · Recientes` va a `RecentsSection`, el `component` de su historia, no a `RecentRows`.
- [GENERAL] `Dialog` y los cuatro diálogos de firma son `position: fixed` y daban `sb-error` (raíz de 0 px). Sus historias llevan el decorador `inDialogWindow` (`.storybook/decorators/dialogWindow.tsx`, 1280 × 720, el tamaño inicial de la ventana principal).
- [GENERAL] La captura es de 900 × 700. Los marcos más anchos se cortaban por la derecha en la vista previa, mientras que la referencia captura el desbordamiento. Por eso los diálogos llevan `viewport: "1340x780"`; `StatusView` y `WithdrawCertificateView`, `"1240x760"`; y `MainWindow`, `"1160x620"`.
- `cardMode`: `single` en `Dialog`, `Popover`, los diálogos de firma y `WithdrawCertificateView`; `column` en `StatusView`, `DocumentViewer`, `ErrorNotice`, `MainWindow` y `RecentsSection` (`[GRID_OVERFLOW]`).
- `SedeView` tiene 63 historias: el driver se corre con `--max-stories 63`, o la principal (`Consent`) se queda sin capturar.

## Artefactos de la referencia, ya graduados

- `Card` › `Elevated` y `Popover` › `In Portal`: la referencia recorta a la raíz de la historia, así que pierde la sombra exterior y el panel en portal. El componente es el mismo.
- `SignaturesDialog`, en las historias altas: el diálogo desborda el marco en los dos lados porque `rf-dialog` no tiene `max-height`. Es fiel al componente.
- `DocumentViewer`: su historia es estrecha y la barra sale cortada por la izquierda en los dos lados.

## Riesgos al volver a sincronizar

- `conventions.md` solo presenta `SedeView` como pantalla; no nombra los diálogos de firma, `StatusView`, `MainWindow` ni el resto de piezas de dominio que ya se sincronizan.

- Hay que recompilar la referencia (`npx storybook build -c .storybook -o ../.design-sync/sb-reference` desde rfirma-app) cuando cambien historias, componentes o CSS.
- Si una pantalla nueva también usa `position: fixed`, necesita un marco como `inSedeWindow`.
- Las piezas componibles son los primitivos de `design-sync.entry.ts` (`Button`, `Card`, `Field`, `Badge`, `Dialog`, `Popover`, `Stack`, `Row`), y las convenciones enseñan a componer con ellos. Un primitivo nuevo se exporta en la entrada, lleva historia y entra en la tabla de las convenciones.
- El `_ds_bundle.css` pesa unos 310 KB porque incluye `app.css` entero.
- Herramientas probadas: Storybook 10.6.1, Vite 8, React 19.3 y Node 24.

## Re-sync de octubre 2026

- `CertificateSelect` entró en la entrada: `cardMode: "column"` (`[GRID_OVERFLOW]`). Su historia `Open` se abre con un `play()` que el arnés no ejecuta; el preview propio `.design-sync/previews/CertificateSelect.tsx` hace clic en el combobox al montar. Si cambia el `play()` de la historia, hay que actualizar ese preview. El compare sigue avisando `[PORTAL?]` (el listbox va en portal); se dejó `column` para conservar todos los estados.
- `SedeView` › `Marking` y `Unreadable Document` (ventana de 1080 × 660) salen cortadas por la derecha en la captura de 900 px. Están graduadas `close`: el recorte es del marco, no del componente. Darle `viewport` a `SedeView` lo arreglaría, pero re-gradúa las 63 historias y agranda la tarjeta `single` de `Consent`.
- `SedeView` tiene ya 66 historias; el driver se corre con `--max-stories 63` y las tres últimas quedan sin graduar (subir el tope las deja pendientes de nota).
- `Header`: las pestañas se truncan algo antes que en la referencia porque el marco del preview lleva 24 px de margen de cuerpo. Aceptado.
- Títulos sin mapear (`TITLE_UNMAPPED`), fuera de la sincronización a propósito: `Menu`, `ProgressBar`, `Select`, y las pantallas `1·Diálogo`, `2·Instalaractualización`, `Colocación`, `1·Antesdefirmar`, `2·Firmado`, `Pantalla`, `Primerarranque`.
- `conventions.md` no menciona `CertificateSelect`.
