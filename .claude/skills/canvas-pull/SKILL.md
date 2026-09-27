---
name: canvas-pull
description: Traer al repositorio los retoques hechos a mano en el lienzo de Claude Design de rFirma.
disable-model-invocation: true
---

# Traer el lienzo al repositorio

Es la dirección contraria de `/canvas`: el usuario ya ha retocado el lienzo en
Claude Design, a mano, y el repositorio se ha quedado atrás. El trabajo es que
`docs/design/artboards/` vuelva a ser la copia 1-1 del proyecto y que las fichas
de `docs/design/` describan lo que ahora se ve.

Las reglas que no se negocian de `.claude/skills/canvas/SKILL.md` valen aquí
enteras —el proyecto del repositorio, un artboard por pantalla-estado, lo de
usar y tirar se funde—. Léelas antes de empezar.

## Lo que baja y lo que no

El proyecto manda sobre **el cuerpo** de cada artboard y sobre `canvas.json`.
Sobre el estilo, no:

- **El `<helmet>` sale siempre de `docs/design/artboards/_helmet.part`.** Al
  traer un artboard se le sustituye el `<helmet>` remoto por ese, porque la
  copia remota se queda atrás y `comprueba.sh` lo exige.
- **`_ds/` no baja nunca.** El sistema de diseño normativo es el bundle, y viaja
  solo del repositorio al proyecto (ADR-0033).
- Un retoque que solo se explica cambiando un token —un color, una sombra, un
  radio nuevos— es un cambio del sistema de diseño: se para y se le dice al
  usuario, que decide si entra por el bundle.

## Tres fases, como en `/canvas`

Los subagentes no tienen `DesignSync`: bajar y subir es tuyo, como dice
«Sincronizar es cosa tuya» en `.claude/skills/canvas/SKILL.md`.

1. **Traer y leer el cambio.** Tú haces `list_files` y bajas a un directorio
   temporal `canvas.json` y los artboards que el usuario diga haber tocado, o,
   si no lo dice, todos los `.dc.html` que están en los dos sitios. Después,
   [TRAER.md](TRAER.md) en un `Agent` (`general-purpose`) con el prompt «lee
   `.claude/skills/canvas-pull/TRAER.md` y ejecútalo» más la ruta del
   directorio y la lista de `list_files`. Devuelve, por artboard, qué cambió
   **en palabras de interfaz**, nunca HTML.
2. **Confirmar con el usuario**, en la sesión principal. Enséñale la lista y
   pregúntale **el porqué** de cada cambio que no se explique solo: el diff dice
   qué, y la ficha necesita por qué. De cinco en cinco como mucho, numeradas, con
   tu lectura propuesta en cada una. Decide también lo que TRAER.md deje marcado
   como dudoso: artboards nuevos, borrados y colores literales.
3. **Consolidar** → otro `Agent` (`general-purpose`) con el prompt «lee
   `.claude/skills/canvas/UNIFICAR.md` y ejecuta sus apartados 2 a 5» más la
   lista confirmada de cambios y sus porqués, la lista de `list_files`, y
   estos añadidos:
   - Si el usuario creó en Claude Design un artboard para una pantalla que ya
     tenía el suyo, ejecuta también el apartado 1, fundir.
   - En la PR, un apartado «Lo que falta en la aplicación»: qué debe cambiar en
     `rfirma-app/src/` para seguir al artboard. La skill no toca código.

Al volver, **sube tú** los artboards traídos, ya con el `<helmet>` de
`_helmet.part`, y lo que devuelva la fase 3, para que proyecto y repositorio
queden idénticos. En la subida de este flujo no hay comprobación de retoque a
mano: lo retocado es justo lo que se ha traído.

La fase 2 no se delega: el porqué solo lo sabe el usuario.

## Hecho es

`comprueba.sh` en verde; `list_files` del proyecto y `ls docs/design/artboards/`
con los mismos `.dc.html`; cada artboard traído, igual byte a byte a su copia
remota; cada pantalla tocada con su ficha al día y su porqué en «Decisiones»; y
la PR abierta.
