# Elevation

`--rf-shadow-card` (reposo) y `--rf-shadow-elevated` (flotante). Son las dos
únicas elevaciones del sistema.

En un lienzo casi negro las sombras apenas se leen: la profundidad la aporta el
contraste entre `--rf-color-background` (#18181b) y `--rf-color-surface`
(#15171a) más el borde. Empieza por `.rf-surface`; sube a `.rf-card` cuando el
elemento sea una unidad de contenido, y a `.rf-card--elevated` solo si flota
sobre lo demás.
