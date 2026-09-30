# Prototipo: abrir y el menú en la barra de título nativa de GTK

Desechable. Responde a la pregunta de `docs/research/barra-de-titulo-en-linux.md`,
donde están el veredicto y lo medido. Tauri fijado a `=2.12.0`.

```
PATH="$HOME/.cargo/bin:$PATH" cargo build
POC_VARIANT=native ./target/debug/titlebar-poc
POC_VARIANT=native GDK_BACKEND=x11 ./target/debug/titlebar-poc
```

`POC_VARIANT`: `native` (el diseño elegido), `baseline`, `html` y `html-runtime`
(las opciones descartadas).

Los scripts `nested.sh`, `run.sh`, `ev.sh`, `click.sh`, `drag.sh` y `shot.sh`
son el arnés del `gnome-shell` anidado de las mediciones sobre 2.11. **Esa sesión
anidada colgó la sesión gráfica del anfitrión**: solo dentro de una máquina
virtual.
