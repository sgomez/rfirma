# Sistema de diseño de rFirma

Referencia normativa y única de la capa visual de rFirma. Es autosuficiente:
quien implemente una pantalla debería poder hacerlo sin más contexto que este
fichero.

El sistema es CSS puro — custom properties y clases — sin dependencia de
framework. El frontend de rFirma es React ([ADR-0013](../adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md)), pero nada de lo que sigue lo
presupone.

---

## 1. Temas

Dos temas, claro y oscuro. **El tema lo decide el sistema operativo** vía
`prefers-color-scheme`; no hay que escribir nada para que eso funcione, basta
con usar los roles de la sección 2.

Para forzarlo, `data-theme="light"` o `data-theme="dark"` sobre **cualquier**
elemento, no solo la raíz. Eso permite una zona clara dentro de una pantalla
oscura sin escribir ni una regla de componente:

```html
<body class="rf-root">              <!-- sigue al sistema -->
  <aside data-theme="dark">…</aside>       <!-- siempre oscuro -->
  <article data-theme="light">…</article>  <!-- siempre claro -->
</body>
```

La estructura CSS que lo sostiene es la de tres estados:

```css
:root, [data-theme="light"], .rf-on-light { /* claro: define TODOS los roles */ }
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) { /* oscuro */ }
}
[data-theme="dark"] { /* oscuro, forzado */ }
```

Tres invariantes al tocarlo:

1. El bloque claro es **el bloque base**. Ningún rol puede existir únicamente
   dentro de una media query.
2. Los selectores `[data-theme]` valen en cualquier elemento, no solo `:root`.
3. El repintado de fondo (`background: var(--rf-bg)`) va en `:where([data-theme])`,
   con especificidad 0, para que un contenedor que sea a la vez superficie y
   ámbito de tema conserve su `--rf-surface` en lugar de recibir el `--rf-bg`
   del lienzo.

**Nunca fijes un color a mano.** Un `color: #fff` o un `background: #18181b`
escritos directamente rompen el modo contrario. Es el error más fácil de cometer.

---

## 2. Color

Los roles semánticos son lo único que se consume. Cambian con el tema.

| Rol | Oscuro | Claro |
| --- | --- | --- |
| `--rf-bg` | `#18181b` | `#ffffff` |
| `--rf-surface` | `#15171a` | `#fafbfc` |
| `--rf-text` | `#ffffff` · 17,72:1 | `#000000` · 21:1 |
| `--rf-text-muted` | `#94a3b8` · 6,91:1 | `#64748b` · 4,76:1 |
| `--rf-primary` | `#94a3b8` | `#64748b` |
| `--rf-on-primary` | `#18181b` · 6,91:1 | `#ffffff` · 4,76:1 |
| `--rf-primary-hover` | `#b6c0cd` | `#475569` |
| `--rf-accent` | `#64748b` | `#94a3b8` |
| `--rf-border-subtle` | `rgba(229,229,229,.12)` | `#e5e5e5` |
| `--rf-border-strong` | `rgba(229,229,229,.45)` · 3,80:1 | `#64748b` · 4,76:1 |
| `--rf-focus-ring` | `#94a3b8` | `#64748b` |
| `--rf-scrim` | `rgba(0,0,0,.6)` | igual |

Los ratios son contra `--rf-bg` de su tema, calculados con la fórmula WCAG 2.x.

**Los dos bordes no son intercambiables.** `--rf-border-subtle` es decorativo:
divisores y borde de tarjeta. `--rf-border-strong` es el contorno de control
—campos, botón secundario— y es el que cumple el 3:1 que exige WCAG 1.4.11 para
identificar un componente de interfaz. Usar el sutil en un control es un fallo
de accesibilidad.

La paleta es fría, desaturada y **monocroma a propósito**: no hay verde de
éxito ni rojo de error. Los estados se señalan con forma, peso y texto, no con
matiz (ver sección 8).

---

## 3. Tipografía

Familia única: **Inter**, licencia OFL. Los roles se distinguen por tamaño,
peso y tracking, nunca por familia.

| Clase | Tamaño / interlineado | Peso | Tracking | Uso |
| --- | --- | --- | --- | --- |
| `.rf-display` | 96 / 1.0 | 700 | −2.4px | portada, héroe |
| `.rf-heading` | 48 / 1.15 | 700 | 0 | título de sección mayor |
| `.rf-title` | 20 / 1.3 | 700 | 0 | título de tarjeta, diálogo, panel |
| `.rf-body` | 12 / 1.0 | 400 | 0 | etiquetas, ayuda, metadatos |
| `.rf-prose` | 14 / 1.6 | 400 | 0 | texto corrido |

`--rf-font-display` y `--rf-font-body` existen como puntos de extensión pero
hoy resuelven a la misma pila. Ambas declaran fallbacks del sistema, de modo
que un fallo de carga degrada a la sans nativa, no a serif.

**`.rf-body` es una medida de etiqueta**, no de lectura: 12px con interlineado
1.0 apelmaza cualquier párrafo. En cuanto un texto pueda ocupar más de una
línea, `.rf-prose`.

Nota de despliegue: Inter **está autoalojada**, no servida desde una CDN. Los
woff2 (subconjuntos `latin` y `latin-ext`) y su OFL viven junto al bundle y
entran en el paquete. No es una preferencia: la CSP hereda `default-src 'self'`
sin `font-src`, así que un `@import` a Google Fonts no cargaría nunca y toda la
aplicación caería a la sans del sistema. El `--share=network` del manifiesto no
cambia nada aquí: existe sólo para la consulta de versión a GitHub (#270), y la
ventana sigue sin poder pedir un recurso de fuera.

---

## 4. Espaciado

Base de 8px, nueve escalones: `--rf-space-1` … `--rf-space-9` =
8, 16, 24, 40, 48, 64, 72, 80, 144px.

Alias semánticos:

| Alias | Valor | Uso previsto |
| --- | --- | --- |
| `--rf-space-xs` | 8px | relleno interno de botón, separación de iconos |
| `--rf-space-sm` | 16px | relleno de bloques de texto, separación en línea |
| `--rf-space-md` | 24px | separación entre campos y entre tarjetas |
| `--rf-space-lg` | 48px | separación entre secciones |
| `--rf-space-xl` | 80px | héroes y contenido destacado |
| `--rf-space-2xl` | 144px | márgenes a sangre, cabecera de página |

Todo `padding`, `margin` y `gap` sale de la escala. **Nunca un px suelto.**

---

## 5. Radio y elevación

`--rf-radius-sm` 4px · `md` 6px · `lg` 8px · `xl` 16px · `pill` 9999px.

`md` es el valor por defecto: botones, tarjetas, campos. `xl` se reserva a
diálogos. `pill` solo a insignias.

Dos elevaciones: `--rf-shadow-card` (reposo) y `--rf-shadow-elevated`
(flotante). Ambas son sombras de cuatro capas y **su valor es el del bundle
versionado**, que es el único del repositorio: el `<helmet>` de los artboards
de `docs/design/artboards/` es una copia comprimida para previsualizar y no es
la fuente. En el tema oscuro las sombras apenas se leen; **la profundidad la
aporta el contraste entre `--rf-bg` y `--rf-surface` más el borde**. Empieza
siempre por una superficie plana y sube solo si el elemento flota de verdad.

---

## 6. Movimiento

`--rf-duration-fast` 150ms · `--rf-duration-base` 300ms · `--rf-easing`
`cubic-bezier(0.4, 0, 0.2, 1)`.

Existe `--rf-duration-slow`, pero vale 300ms igual que `base`: **hay dos
escalones reales, no tres**.

Anima solo `opacity`, `transform`, `color`, `background-color` y `border-color`.
Las tres duraciones caen a 1ms bajo `prefers-reduced-motion` desde la propia
capa de tokens; no repitas esa media query en los componentes.

---

## 7. Puntos de ruptura

400, 640, 768, 1024, 1280, 1400px. Existen como
`--rf-bp-xs|sm|md|lg|xl|2xl` para lectura desde JS, pero **las custom
properties no funcionan dentro de `@media`**: escribe el valor literal.

---

## 8. Accesibilidad

Requisitos que la capa de componentes ya satisface y que no hay que
reimplementar ni relajar:

- **Área táctil.** Mínimo 44×44px en botones y campos.
- **Foco.** Contorno de `--rf-focus-ring-width` (2px) en `--rf-focus-ring` con
  `--rf-focus-ring-offset` (2px) de desplazamiento, aplicado a todo
  `:focus-visible` bajo la raíz. Las dos medidas son tokens, no literales.
- **El color nunca es el único indicador.** El botón secundario cambia borde
  *y* color en hover; el campo con error **engorda el borde a 2 px y sube a
  negrita el texto de ayuda**.

  **Sin glifo antepuesto, y no vuelve.** `.rf-field--error .rf-hint::before`
  ponía un `"! "` delante de la ayuda; se retiró en la v0.4 del bundle, de los
  catorce artboards y de los dos documentos que lo prescribían. El motivo es de
  idioma: en castellano la exclamación **abre con `¡`**, así que un `!` suelto
  delante de la frase se lee como una exclamación mal cerrada, no como un aviso.
  El requisito se sigue cumpliendo sin él —el borde y el peso son dos
  indicadores no cromáticos—, así que no hace falta sustituirlo por otro glifo:
  si alguien quiere reponer una señal, que sea texto que se lea.
- **Contraste.** Todo par texto/fondo del sistema cumple WCAG AA como mínimo, y
  los contornos de control cumplen el 3:1 de WCAG 1.4.11, en ambos temas.
- **Movimiento reducido.** Ver sección 6.

---

## 9. Vocabulario de clases

Prefijo `rf-`. Fuera de esta tabla no hay clases; para la maquetación propia,
CSS con `var(--rf-*)`.

| Familia | Clases |
| --- | --- |
| Raíz y tema | `.rf-root`, `.rf-on-light`, `[data-theme]` |
| Texto | `.rf-display`, `.rf-heading`, `.rf-title`, `.rf-body`, `.rf-prose`, `.rf-text-muted`, `.rf-text-primary` |
| Disposición | `.rf-stack`, `.rf-row`, `.rf-section`, `.rf-divider`, `.rf-gap-xs\|sm\|md\|lg` |
| Superficies | `.rf-surface`, `.rf-card`, `.rf-card--elevated`, `.rf-card--interactive` |
| Botones | `.rf-btn` + `--primary\|--secondary\|--ghost\|--pill\|--disabled` |
| Formularios | `.rf-field`, `.rf-field--error`, `.rf-label`, `.rf-input`, `.rf-hint` |
| Otros | `.rf-badge`, `.rf-badge--primary`, `.rf-dialog`, `.rf-scrim` |

`.rf-root` es obligatoria en la raíz de toda pantalla: establece lienzo, familia
tipográfica y color heredado.

`.rf-on-light` es un alias de `data-theme="light"`, conservado por
compatibilidad: hace exactamente lo mismo y no hay motivo para preferirlo en
código nuevo. `.rf-btn--disabled` es el equivalente en clase del atributo
`[disabled]`, para un elemento que no lo admite.

---

## 10. Componentes

### Botón

```html
<button class="rf-btn rf-btn--primary">Firmar documento</button>
<button class="rf-btn rf-btn--secondary">Elegir certificado</button>
<button class="rf-btn rf-btn--ghost">Cancelar</button>
```

El par relleno/texto se invierte con el tema, así que **no fijes nunca a mano el
color del texto de un botón primario**. Como máximo un `--primary` por vista.

### Tarjeta

```html
<div class="rf-card">
  <p class="rf-title">DNIe</p>
  <p class="rf-body rf-text-muted">Caduca el 14/03/2029</p>
</div>
```

`.rf-card` ya es flex en columna con `gap: --rf-space-1` y
`padding: --rf-space-md`; no le añadas relleno propio. `--interactive` solo si
toda la tarjeta es pulsable. Para un contenedor sin sombra ni relleno,
`.rf-surface`.

### Campo

```html
<div class="rf-field">
  <label class="rf-label" for="ruta">Ruta del documento</label>
  <input class="rf-input" id="ruta">
  <span class="rf-hint">Formatos admitidos: PDF, XML, FacturaE.</span>
</div>
```

Siempre `<label>` asociado por `for`/`id`; un placeholder no es una etiqueta.
Entre campos, `--rf-space-md`. Error: `.rf-field--error` en el contenedor, que
engorda el borde y pone la ayuda en negrita — y nada más delante del texto (ver
sección 8).

### Insignia

```html
<span class="rf-badge rf-badge--primary">Activo</span>
<span class="rf-badge">Software</span>
```

Una o dos palabras. No hay variantes de éxito ni de error (sección 2).

### Ruta de destino

Dónde va a caer un fichero: **la última carpeta y el nombre**, nunca la ruta
entera. Nace en el pie del panel de firma y lo fija el
[ADR-0011](../adr/0011-destino-del-documento-firmado.md).

```html
<p class="rf-prose" style="flex:1;min-width:0;overflow-wrap:anywhere">
  <span class="rf-text-muted">…/Documentos/</span>contrato-de-arrend…-firmado.pdf
</p>
```

Tres reglas, y las tres son el componente:

1. **La carpeta va atenuada y el nombre no.** La carpeta es contexto; el nombre
   es el dato. Delante lleva `…/`, que dice que hay carpetas por encima sin
   afirmar cuáles: bajo el sandbox la aplicación no las conoce, y fuera de él no
   se enseñan igualmente.
2. **El nombre se recorta por el medio**, no por la cola. Se conservan siempre
   la extensión y el sufijo `-firmado` con su número de desempate —`-2`, `-3`—,
   porque son la respuesta a «¿voy a machacar el anterior?», que es lo que se
   mira. El `…` se come el centro del tronco. La **carpeta no se recorta nunca
   por el medio**; si hace falta, por la cola: un nombre de carpeta se reconoce
   por el principio y no tiene ninguna cola que preservar.
3. **La línea envuelve antes que cortarse.** Nada de `white-space: nowrap` con
   `overflow: hidden`: eso corta en seco lo que el `…` ya había recortado, y sin
   avisar. Envuelve con `overflow-wrap: anywhere`, y quien la acompaña —el icono
   de carpeta, el botón `Cambiar`— se alinea arriba (`align-items: flex-start`).

Se maqueta con tokens; no hay clase propia en el bundle.

### Diálogo

```html
<div class="rf-scrim">
  <div class="rf-dialog">
    <p class="rf-title">Confirmar firma</p>
    <p class="rf-prose rf-text-muted">…</p>
    <hr class="rf-divider">
    <div class="rf-row rf-gap-sm" style="justify-content:flex-end">
      <button class="rf-btn rf-btn--ghost">Cancelar</button>
      <button class="rf-btn rf-btn--primary">Firmar</button>
    </div>
  </div>
</div>
```

Acciones abajo a la derecha, la de descarte como `--ghost` a la izquierda de la
primaria. Título en `.rf-title`, nunca en `.rf-heading`.

**`.rf-scrim` en el bundle es solo el color.** La clase trae el
`background: var(--rf-scrim)` y nada más: ni `position`, ni `inset`, ni
centrado. Quien coloca el velo —`position: fixed`, `inset: 0` y el centrado del
diálogo— es `rfirma-app/src/app.css`, y por eso los diálogos se superponen a la
ventana en vez de pintarse en flujo detrás de ella. Ahí mismo están las otras
dos reglas que el bundle tampoco trae y que toda la aplicación da por hechas:
`box-sizing: border-box` para todo y `margin: 0` en el documento. **Todas las
medidas de estas fichas son de borde a borde**, con el borde y el relleno
dentro; sin ese reinicio ninguna es la que se pinta.

---

## 11. Por qué el sistema no es lo obvio

Cuatro puntos donde la especificación de partida no funcionaba y el sistema se
apartó de ella. Están aquí porque, sin la explicación, parecen arbitrarios y
alguien los "arreglará" de vuelta.

1. **`--rf-on-primary` es oscuro en el tema oscuro.** La combinación intuitiva
   —texto blanco sobre `#94a3b8`— da **2,56:1**, por debajo del 4,5:1 que exige
   WCAG AA. Invertir el par lo lleva a 6,91:1. En el tema claro el problema se
   resuelve al revés: el relleno pasa al slate oscuro (`#64748b`) con texto
   blanco, 4,76:1. **La paleta no crece por ello**: son los dos slates de
   siempre, repartidos según el tema.

2. **Recalcula siempre los ratios.** Los que traía la especificación de partida
   estaban mal, y no por poco: el par primario se documentaba como 5,8:1 cuando
   es 2,56:1. Cualquier cifra de contraste que vayas a escribir en un documento
   o a usar para justificar una decisión, compruébala con la fórmula WCAG 2.x.

3. **El color de texto no puede ser un valor fijo.** Un sistema de dos temas no
   admite un token `text` con un único valor: cualquier elección falla en el
   modo contrario. De ahí que `--rf-text` y `--rf-text-muted` se resuelvan por
   tema y que no exista forma de escribir un color de texto literal sin romper
   algo.

4. **`.rf-title` y `.rf-prose` existen porque la escala tenía un hueco.** Entre
   `heading` (48px) y `body` (12px/1.0) no había nada, y ninguno de los dos
   sirve para un título de tarjeta ni para un párrafo. No los sustituyas por
   tamaños inventados sobre la marcha.

Nota menor: `--rf-duration-slow` vale lo mismo que `--rf-duration-base`. El
token se conserva por compatibilidad, pero **hay dos escalones de movimiento,
no tres**.
