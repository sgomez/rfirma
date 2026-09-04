# Input

```html
<div class="rf-field">
  <label class="rf-label" for="ruta">Ruta del documento</label>
  <input class="rf-input" id="ruta">
  <span class="rf-hint">Formatos admitidos: PDF, XML, FacturaE.</span>
</div>
```

`.rf-field` apila etiqueta, control y ayuda con `gap: --rf-space-1`. Entre
campos deja `--rf-space-3` (24px).

Error: añade `.rf-field--error` al contenedor. Engorda el borde **y** pone en
negrita el texto de ayuda — el color nunca es el único indicador. Nada de glifo
antepuesto: un "!" suelto se lee mal en castellano. Siempre
`<label>` asociado por `for`/`id`; un placeholder no es una etiqueta.

Altura mínima 44px, igual que los botones.
