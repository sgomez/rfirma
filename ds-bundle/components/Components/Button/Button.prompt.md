# Button

```html
<button class="rf-btn rf-btn--primary">Firmar documento</button>
<button class="rf-btn rf-btn--secondary">Elegir certificado</button>
<button class="rf-btn rf-btn--ghost">Cancelar</button>
```

Modificadores: `--primary` (relleno `--rf-primary`, texto `--rf-on-primary`;
6,91:1 en oscuro, 4,76:1 en claro) · `--secondary` (contorno) · `--ghost` (sin
caja) · `--pill` (radio completo). Desactivado con el atributo `disabled`.

El par relleno/texto se invierte con el tema, y por eso **nunca** fijes el color
del texto de un botón primario a mano: blanco sobre #94a3b8 da 2,56:1 y
falla AA.

Altura mínima 44px por el requisito de área táctil — no la
reduzcas. `--secondary` cambia **borde y color** en hover, nunca solo el color:
el color no puede ser el único diferenciador.

Como máximo un `--primary` por vista.
