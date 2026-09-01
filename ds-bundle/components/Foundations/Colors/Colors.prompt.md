# Colors

Dos capas. Los tokens **`--rf-color-*`** son los literales de la paleta y no
cambian con el tema; están ahí como referencia. Lo que consume el sistema son
los **roles semánticos**, que sí cambian:

| Rol | Tema oscuro | Tema claro |
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

Usa siempre el rol, nunca el literal. `--rf-color-text` (#000000) sobre el
lienzo oscuro da 1,17:1.

**Bordes.** `--rf-border-subtle` es decorativo (divisores, borde de tarjeta).
`--rf-border-strong` es el contorno de control (campos, botón secundario) y
cumple el 3:1 que exige WCAG 1.4.11 en ambos temas — no lo sustituyas por el
sutil en un control.

**Por qué el par se invierte.** Blanco sobre `primary` (#94a3b8) da **2,56:1**
y no alcanza el 4,5:1 de WCAG AA. En oscuro el par se invierte (texto #18181b
sobre #94a3b8 → 6,91:1); en claro el relleno pasa al slate más oscuro (#64748b
con texto blanco → 4,76:1). La paleta no se amplía: son los dos slates de
siempre, repartidos según el tema.

```html
<div class="rf-root">
  <p class="rf-body">Texto principal</p>
  <p class="rf-body rf-text-muted">Metadato secundario</p>
</div>
```
