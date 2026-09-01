# Typography

Una sola familia: **Inter** (OFL), servida desde Google Fonts. La especificación
de partida nombraba `InterDisplay` e `InterVariable`, compilaciones de
distribución privada de esta misma tipografía. Los tokens `--rf-font-display` y
`--rf-font-body` existen como puntos de extensión, pero hoy resuelven a la
misma pila. Los roles se distinguen por tamaño, peso y tracking, no por familia.

| Clase | Tamaño / interlineado | Uso |
| --- | --- | --- |
| `.rf-display` | 96 / 1.0, -2.4px | héroes, portada |
| `.rf-heading` | 48 / 1.15 | títulos de sección mayores |
| `.rf-title` | 20 / 1.3 | **añadido** — la escala saltaba de 48px a 12px sin escalón intermedio |
| `.rf-body` | 12 / 1.0 | etiquetas, ayuda, metadatos |
| `.rf-prose` | 14 / 1.6 | **añadido** — texto corrido; 12/1.0 apelmaza los párrafos |

Solo `display` lleva tracking. No apliques `letter-spacing` a los tamaños pequeños.
