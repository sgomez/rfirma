# Card

```html
<div class="rf-card">
  <p class="rf-title">DNIe</p>
  <p class="rf-body rf-text-muted">Caduca el 14/03/2029</p>
</div>
```

`.rf-card` ya es un `flex` en columna con `gap: --rf-space-1` y
`padding: --rf-space-3`: no le añadas relleno propio.

Modificadores: `--elevated` (sombra flotante) · `--interactive` (cursor y
transición de hover; úsalo solo si toda la tarjeta es pulsable).

Para un contenedor sin sombra ni relleno usa `.rf-surface`.
