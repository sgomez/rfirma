# Spacing

Base de 8px, nueve escalones: 8, 16, 24, 40, 48, 64, 72, 80, 144.
Accede a ellos como `var(--rf-space-1)`…`var(--rf-space-9)`, o por los alias
semánticos `--rf-space-xs | sm | md | lg | xl | 2xl`.

Reglas: todo valor de `padding`, `margin` y `gap` sale de la escala. Nunca
escribas un px suelto. Para separaciones en flex usa las clases utilitarias
`.rf-gap-xs | sm | md | lg` sobre `.rf-stack` o `.rf-row`.
