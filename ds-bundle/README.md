# rFirma Design System

Sistema **solo CSS**. **No hay componentes React que
importar**: `window.RFirma` únicamente expone los nombres de los tokens. Se
construye con elementos HTML normales y las clases `rf-*` de `_ds_bundle.css`.

Tipografía: **Inter** (OFL), una sola familia para todos los roles.

## Envoltura obligatoria

Toda pantalla empieza por `.rf-root`. Establece el lienzo, la familia de texto
y el color heredado. **Sin ella los componentes caen sobre el blanco por
defecto del navegador y el sistema se ve roto.**

```html
<body class="rf-root">
  <section class="rf-section rf-stack rf-gap-md">
    <h1 class="rf-title">Certificados disponibles</h1>
    <div class="rf-card">
      <p class="rf-title">DNIe</p>
      <p class="rf-body rf-text-muted">Caduca el 14/03/2029</p>
      <button class="rf-btn rf-btn--primary">Firmar documento</button>
    </div>
  </section>
</body>
```

## Claro y oscuro

El tema lo decide el sistema operativo vía `prefers-color-scheme`. **No escribas
nada para que eso funcione**: usa los roles y ya cambian solos.

Para forzarlo, `data-theme="light"` o `data-theme="dark"` sobre **cualquier**
elemento, no solo la raíz — así una zona clara dentro de una pantalla oscura no
necesita ni una regla de componente:

```html
<body class="rf-root">           <!-- sigue al sistema -->
  <aside data-theme="dark">…</aside>   <!-- siempre oscuro -->
  <article data-theme="light">…</article> <!-- siempre claro -->
</body>
```

`.rf-on-light` es un alias de `data-theme="light"`, conservado por compatibilidad.

**Nunca fijes un color a mano.** Un `color: #fff` o un `background: #18181b`
escritos directamente rompen el modo contrario, y es el error más fácil de
cometer aquí.

## Roles de color

Los tokens `--rf-color-*` son los literales de la paleta y **no cambian con el
tema**; están como referencia. Lo que se consume son los roles:

| Rol | Oscuro | Claro |
| --- | --- | --- |
| `--rf-bg` | #18181b | #ffffff |
| `--rf-surface` | #15171a | #fafbfc |
| `--rf-text` | #ffffff · 17,72:1 | #000000 · 21:1 |
| `--rf-text-muted` | #94a3b8 · 6,91:1 | #64748b · 4,76:1 |
| `--rf-primary` | #94a3b8 | #64748b |
| `--rf-on-primary` | #18181b · 6,91:1 | #ffffff · 4,76:1 |
| `--rf-primary-hover` | #b6c0cd | #475569 |
| `--rf-border-subtle` | rgba(229,229,229,.12) | #e5e5e5 |
| `--rf-border-strong` | rgba(229,229,229,.45) · 3,80:1 | #64748b · 4,76:1 |
| `--rf-focus-ring` | #94a3b8 | #64748b |
| `--rf-scrim` | rgba(0,0,0,.6) | igual |

`--rf-border-subtle` es decorativo (divisores, borde de tarjeta).
`--rf-border-strong` es el contorno de control (campos, botón secundario) y
cumple el 3:1 de WCAG 1.4.11 en ambos temas: no lo cambies por el sutil.

## Vocabulario de clases

| Familia | Clases |
| --- | --- |
| Raíz y tema | `.rf-root`, `.rf-on-light`, `[data-theme]` |
| Texto | `.rf-display`, `.rf-heading`, `.rf-title`, `.rf-body`, `.rf-prose`, `.rf-text-muted`, `.rf-text-primary` |
| Disposición | `.rf-stack`, `.rf-row`, `.rf-section`, `.rf-divider`, `.rf-gap-xs\|sm\|md\|lg` |
| Superficies | `.rf-surface`, `.rf-card`, `.rf-card--elevated`, `.rf-card--interactive` |
| Botones | `.rf-btn` + `--primary\|--secondary\|--ghost\|--pill` |
| Formularios | `.rf-field`, `.rf-field--error`, `.rf-label`, `.rf-input`, `.rf-hint` |
| Otros | `.rf-badge`, `.rf-badge--primary`, `.rf-dialog`, `.rf-scrim` |

Fuera de esta tabla no hay clases. Para la maquetación propia usa CSS con
`var(--rf-*)`; **nunca escribas un valor literal** de color, espacio, radio,
sombra ni duración.

## Tres cosas que no son evidentes

1. **El par primario intuitivo falla WCAG AA.** Blanco sobre `#94a3b8` da
   **2,56:1**, por debajo del 4,5:1 de AA. El par se invierte en oscuro
   (`--rf-on-primary` = #18181b → 6,91:1) y en claro el relleno pasa al slate
   más oscuro (#64748b con blanco → 4,76:1). La paleta no crece: son los dos
   slates de siempre, repartidos según el tema.
2. **`text` y `text-muted` no son roles de tema.** `text: #000000` sobre el
   lienzo oscuro da 1,17:1 y `text-muted: #ffffff` es más brillante que el texto
   principal. Usa `--rf-text` y `--rf-text-muted`.
3. **Interlineado.** `body` es 12px/1.0, una medida de etiqueta. Para texto de
   más de una línea usa `.rf-prose` (14/1.6). `.rf-title` (20/1.3) cubre el
   hueco entre `heading` (48px) y `body` (12px), que en el documento no existe.

## Reglas de accesibilidad

- Altura mínima 44px en botones y campos: ya viene en `.rf-btn` y `.rf-input`.
- Foco: contorno de 2px en `--rf-focus-ring` con 2px de desplazamiento; lo
  aplica `.rf-root :focus-visible`, no lo reimplementes.
- El color nunca es el único indicador: acompáñalo de borde, peso o glifo
  (`.rf-field--error` antepone un "!" al texto de ayuda). No hay color de error
  en la paleta a propósito.
- Las duraciones caen a 1ms bajo `prefers-reduced-motion` desde
  `tokens/motion.css`; no repitas la media query.
- Puntos de ruptura: 400, 640, 768, 1024, 1280, 1400px. Escríbelos literales en
  `@media`; las custom properties no funcionan ahí.

## Dónde está la verdad

`styles.css` y su cierre de imports: `tokens/color.css` (los dos temas),
`typography.css`, `spacing.css`, `radius.css`, `shadow.css`, `motion.css`,
`breakpoint.css`, `fonts/fonts.css` y `_ds_bundle.css`. Léelos antes de dar
estilo. Cada componente tiene su
`components/<grupo>/<Nombre>/<Nombre>.prompt.md` con el fragmento canónico.

Las tarjetas de preview están fijadas a `data-theme="dark"`: muestran la
identidad del sistema, no el modo del visitante.


---

## Índice de componentes

### Components

- **Badge** — `components/Components/Badge/Badge.prompt.md`
- **Button** — `components/Components/Button/Button.prompt.md`
- **Card** — `components/Components/Card/Card.prompt.md`
- **Dialog** — `components/Components/Dialog/Dialog.prompt.md`
- **Input** — `components/Components/Input/Input.prompt.md`

### Foundations

- **Colors** — `components/Foundations/Colors/Colors.prompt.md`
- **Elevation** — `components/Foundations/Elevation/Elevation.prompt.md`
- **Motion** — `components/Foundations/Motion/Motion.prompt.md`
- **Radius** — `components/Foundations/Radius/Radius.prompt.md`
- **Spacing** — `components/Foundations/Spacing/Spacing.prompt.md`
- **Typography** — `components/Foundations/Typography/Typography.prompt.md`

## Ficheros

- `styles.css` — hoja raíz. Todo lo que recibe un diseño llega por su cierre de imports.
- `tokens/color.css` — los dos temas (claro por defecto, oscuro bajo `prefers-color-scheme`, ambos forzables con `data-theme`).
- `tokens/*.css` — tipografía, espaciado, radio, sombra, movimiento, breakpoints.
- `_ds_bundle.css` — capa de componentes y utilidades `rf-*`.
- `_ds_bundle.js` — expone `window.RFirma` (solo nombres de tokens; **no hay componentes React**).
- `fonts/fonts.css` — carga Inter (OFL) desde Google Fonts.
- `components/<grupo>/<Nombre>/` — `<Nombre>.html` (tarjeta de preview) y `<Nombre>.prompt.md` (uso).
