# design-sync: notas de rFirma

Proyecto: «rFirma Components» (`312bca0c-2f94-494a-820a-e947e03f9ade`), forma `storybook`. Spec: #1483 (fase 0, #1486).

## Forma del repositorio

- rfirma-app es una aplicación, no una biblioteca: no hay `dist/`. Lo que se compila lo dice `rfirma-app/design-sync.entry.ts` (`cfg.entry` y `cfg.extraEntries`). Un componente nuevo para Claude Design se exporta ahí y lleva su historia.
- El escaneo de exportaciones solo ve los nombres a través de `extraEntries`: con `entry` a secas sale «0/1 storybook components are public exports».
- `titleMap` usa el último segmento del título sin espacios (`1·Espera`), no el título entero.

## Arreglos

- [GENERAL] El decorador de `.storybook/preview.tsx` no se empaqueta: importa el CSS del sistema de diseño, que trae `.woff2`, y el empaquetador de decoradores no tiene cargador para fuentes. Por eso `cfg.provider` es `DesignRoot`, la misma raíz que usa el decorador.
- [GENERAL] Ni los tokens ni las clases `rf-*` llegaban a `_ds_bundle.css`. El convertidor solo toma el CSS que importan los componentes de la entrada, y `index.css` lo importa el preview de Storybook, no un componente. Ahora la entrada importa `index.css` y `app.css`. `cfg.tokensGlob` no sirve sin `cfg.tokensPkg`.
- [GENERAL] La ventana de sede usa `position: fixed; inset: 0` y en Storybook medía 0 px (`sb-error`: «no storybook root content»). Las historias de sede llevan el decorador `inSedeWindow` (520 × 420, el `DIALOG_SIZE` de Tauri), y `SedeWindow` va con `cardMode: "single"`.

## Riesgos al volver a sincronizar

- Hay que recompilar la referencia (`npx storybook build -c .storybook -o ../.design-sync/sb-reference` desde rfirma-app) cuando cambien historias, componentes o CSS.
- Si una pantalla nueva también usa `position: fixed`, necesita un marco como `inSedeWindow`.
- `SedeWindow` recibe el puerto `errands`, y las convenciones le dicen al agente que la use solo como referencia. Si se suben piezas componibles, hay que revisar esa frase.
- El `_ds_bundle.css` pesa unos 310 KB porque incluye `app.css` entero.
- Herramientas probadas: Storybook 10.6.1, Vite 8, React 19.3 y Node 24.
