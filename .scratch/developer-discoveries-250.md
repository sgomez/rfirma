### `cargo test --lib` ya no compila sin `dist`, y `dist` necesita `just po-import` primero
Doc: CLAUDE.md (sección "Herramientas y Estado de Configuración del Entorno", entrada "Cadena de Tauri")
Evidence: PR #281, #288, #293
Proposed: Borrar la frase "con una excepción útil: `cargo test --lib` sí compila sin `dist` construido, y es el bucle rápido mientras trabajas; `cargo test` a secas y el resto de recetas sí lo necesitan" — ya no es cierta: `lib.rs` llama a `tauri::generate_context!()`, que revienta con "The `frontendDist` configuration is set to `"../dist"` but this path doesn't exist" tanto en `cargo test --lib` como en `cargo clippy --lib` si `dist` no existe. Sustituir por: en un worktree limpio, `pnpm install` + `vite build` NO basta para producir `dist`, porque `src/i18n/locales/*.ts` se genera desde `po/*.po` (`tools/po-import.mjs`) y no está versionado; el arranque completo es `pnpm install` → `just po-import` (o `pnpm install` + `node tools/po-import.mjs` + `pnpm exec i18next-cli types -q` + `pnpm exec vite build`) → recién entonces cualquier receta de Rust, sin excepción para `--lib`.

### El ADR-0015 dice `nginx:alpine` y `main` corre `caddy:alpine` desde el #273
Doc: docs/adr/0015-canal-propio-tres-repositorios-en-rfirma-sgomez-me-y-releases.md
Evidence: PR #304
Proposed: Reescribir la línea del ADR-0015 que fija la imagen del contenedor de `packaging/repo/` de `nginx:alpine` a `caddy:alpine`, para que el ADR describa lo que hay realmente en el repositorio desde el #273.

### Las guardas de `.desktop` (`check-version.py`) y cualquier prueba futura sobre lanzadores deben anclarse en `desktopTemplate`, no en el patrón de fichero
Doc: CLAUDE.md o docs/adr/0018 (guardas de `.desktop`)
Evidence: PR #298
Proposed: Documentar dos cosas descubiertas al añadir el primer *servicemenu* de KDE: (1) un `.desktop` de servicemenu necesita el bit de ejecución (`chmod +x`, modo git `100755`) desde Plasma 5.85 o Dolphin pide confirmación cada vez, y `packaging/check-version.py` ya lo vigila; (2) el lanzador de escritorio del `.deb`/`.rpm` es una plantilla Handlebars (`packaging/rfirma.desktop.hbs`) generada por `desktopTemplate` de `tauri.conf.json`, así que cualquier guarda o prueba que sólo recorra `*.desktop` bajo `packaging/` la deja fuera — hay que anclar la comprobación en el `desktopTemplate`, no en el nombre de fichero.

### `just` sólo deduplica dependencias dentro de una misma invocación, y fijar una GitHub Action por SHA puede cambiar su comportamiento si depende del *ref* invocado
Doc: justfile (comentario junto a las recetas `flatpak`/`bundle`) o docs/agents/code-host-ci.md
Evidence: PR #299
Proposed: Anotar que `just flatpak` seguido de `just bundle` reconstruye la imagen nativa dos veces (los tres canales pueden acabar con bytes distintos), mientras que `just flatpak bundle` en una sola invocación la deduplica; y que fijar una acción de GitHub por SHA (p.ej. `dtolnay/rust-toolchain@<sha>`) no congela solo la versión: algunas acciones (como esa) leen el *ref* con el que se las invoca para decidir qué instalar, así que un SHA sin el parámetro `toolchain:` puede no instalar nada.

### `pnpm exec tsc --noEmit` en `rfirma-app/` no comprueba ningún fichero
Doc: rfirma-app/src/AGENTS.md (o CLAUDE.md, sección de comprobaciones)
Evidence: PR #302
Proposed: Advertir que el `tsconfig.json` de la raíz de `rfirma-app/` es `{"files": [], "references": [...]}`, así que `pnpm exec tsc --noEmit` sale en verde sin mirar un solo fichero; el typecheck real es `pnpm exec tsc -b` (lo que corre `just build-ts`). Cualquier verificación manual de tipos debe usar `tsc -b`, no `tsc --noEmit`.
