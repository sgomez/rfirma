# Dialog

```html
<div class="rf-dialog">
  <p class="rf-title">Confirmar firma</p>
  <p class="rf-prose rf-text-muted">…</p>
  <hr class="rf-divider">
  <div class="rf-row rf-gap-sm" style="justify-content:flex-end">
    <button class="rf-btn rf-btn--ghost">Cancelar</button>
    <button class="rf-btn rf-btn--primary">Firmar</button>
  </div>
</div>
```

`.rf-dialog` es la caja: radio `xl`, sombra `elevated`, ancho máximo 480px,
apilado en columna con `gap: --rf-space-2`. Envuélvela en `.rf-scrim` para el
velo de fondo.

Acciones abajo a la derecha, la destructiva o de descarte como `--ghost` a la
izquierda de la primaria. Nunca dos botones `--primary`. Título en `.rf-title`
(20px), no en `.rf-heading` (48px): un diálogo no es una portada.
