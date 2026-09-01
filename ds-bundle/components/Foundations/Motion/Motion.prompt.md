# Motion

`--rf-duration-fast` 150ms · `--rf-duration-base` 300ms · `--rf-duration-slow`
300ms · `--rf-easing` `cubic-bezier(0.4, 0, 0.2, 1)`.

`slow` es idéntico a `base`; se conserva por compatibilidad, pero **no hay
tres escalones reales, hay dos**. Anima solo `opacity`, `transform`, `color`,
`background-color` y `border-color`.

`tokens/motion.css` anula las tres duraciones a 1ms bajo
`prefers-reduced-motion`, así que no necesitas repetir esa consulta.
