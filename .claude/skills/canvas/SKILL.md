---
name: canvas
description: Prototipar interfaz de rFirma en el lienzo de Claude Design. Úsala cuando prototype, grill-with-docs o wayfinder lleguen a una rama de UI.
---

# El lienzo de rFirma

Este proyecto tiene **un solo** proyecto de Claude Design y **un artboard por
pantalla-estado**. Las dos cosas se olvidan siempre, y las dos se pagan caras:
un proyecto nuevo parte el historial de decisiones, y un artboard duplicado deja
dos sitios donde mirar la misma pantalla.

`docs/agents/prototyping.md` es el contrato completo. Esta skill es el
procedimiento, y **manda sobre cualquier instinto de crear algo nuevo**.

## Antes de nada: quién dibuja

Por defecto dibuja el usuario, en Claude Design (`docs/agents/prototyping.md`,
«Quién dibuja»). Tú dibujas solo si el dibujo sale del repositorio —un issue,
la spec, el código— o es el mismo cambio en muchos artboards. Si no es ninguno
de los dos, díselo: que lo dibuje allí y lance `/canvas-pull` al terminar. Si
lo es, sigue.

## El trabajo son tres fases, y dos de ellas no van aquí

1. **Dibujar** el prototipo con el estilo que ya existe → [PROTOTIPAR.md](PROTOTIPAR.md).
2. **Validar** con el usuario, iterando sobre lo dibujado. Esta fase es la
   conversación: se queda en la sesión principal, con el usuario delante.
3. **Unificar** —fundir lo de usar y tirar, dejar la copia 1-1 y escribir las
   fichas— → [UNIFICAR.md](UNIFICAR.md).

Las fases 1 y 3 son mecánicas, largas y se comen el contexto en lecturas de
HTML que a la conversación no le sirven de nada. **Van en un contexto propio**:
lánzalas con `Agent` (`general-purpose`), pasándole como prompt «lee
`.claude/skills/canvas/PROTOTIPAR.md` y ejecútalo para \<encargo\>» más el
encargo concreto, y en la fase 3 la lista de ficheros del proyecto. Lo que
vuelve a la sesión principal es qué decide cada palanca (fase 1), o el resumen
de qué se fundió y qué fichas cambiaron (fase 3), y en las dos las rutas que
subir y borrar. Nunca el HTML. Con esas rutas sincronizas tú.

La fase 2 **no** se delega: el veredicto es del usuario y las correcciones
llegan en tandas numeradas sobre lo que está viendo.

### Sincronizar es cosa tuya

**Un subagente no tiene `DesignSync`**: trabaja solo sobre ficheros locales. La
sesión principal es la única que habla con el proyecto:

- **Bajar**: `get_file` de cada ruta y su contenido a un directorio temporal
  fuera del repositorio. La salida grande se guarda en un fichero; extráela con
  `jq -j .content`, porque `jq -r` añade un salto de línea que rompe la
  igualdad byte a byte.
- **Subir**: el subagente te devuelve las rutas que escribir y las que borrar.
  Antes de pisar un artboard que ya existe, bájalo y compáralo fuera del
  `<helmet>` con su versión de `HEAD`: si difieren, el usuario lo retocó a
  mano; para y pide `/canvas-pull` antes de subir nada. Después,
  `finalize_plan` (con `writes` **y** `deletes`, obligatorio aunque vaya vacío)
  con `localDir` en `docs/design/artboards/`, `write_files` con `localPath` y
  `delete_files`.
- **Comprobar**: `get_file` de cada ruta subida y `cmp` con la copia local.

## La fase 2, que es la que te toca a ti

Iterar sobre lo dibujado, con el usuario delante:

- Las correcciones llegan **en tandas numeradas** sobre el mismo artboard. Se
  aplican ahí, sin crear nada nuevo, y se vuelve a enseñar.
- Si preguntas, **de cinco en cinco como mucho**, numeradas, con tu
  recomendación en cada una: más que eso obliga a hacer scroll para contestar
  (03/09).
- **Lo decidido no se vuelve a preguntar.** «Vamos a hacer una cosa pq estas
  entrando en bucle» (03/09) es lo que pasa cuando se reformula una pregunta ya
  contestada.
- Cuando el usuario dice que algo no se ve o que dos opciones se ven iguales,
  el fallo es del dibujo: arréglalo, no lo expliques.
- Nada de fase 3 hasta que el usuario diga que cierra los diseños.

## Las reglas que no se negocian

1. **Siempre el proyecto del repositorio.** `projectId`
   `c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132`, nombre «Autofirma de escritorio en
   Rust», <https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132>.
   **Nunca** `create_project`, nunca un proyecto por tanda de trabajo. Si crees
   que hace falta uno nuevo, no hace falta. («te he dicho que debe ir al
   proyecto», 01/09; «recuerda que tienes que crear las paginas en nuestro
   proyecto», 03/09.)

2. **Un caso de uso, un artboard por pantalla-estado. No hay duplicados.**
   Si vas a explorar una decisión sobre una pantalla que **ya tiene** artboard,
   la exploración se hace **con palancas dentro de ese artboard**, no en uno
   nuevo al lado. Solo se crea un artboard cuando aparece una
   **pantalla-estado que no existía** (un diálogo nuevo, por ejemplo).

3. **Un artboard de trabajo nace para morir.** Si por lo que sea acabas
   creando uno de usar y tirar, **al validar la decisión se funde en el
   artboard de la pantalla y el de trabajo se borra**, junto con su página y su
   fila del registro. Ya pasó con `VistaPreviaRecuadro` y `VistaPreviaDentro`,
   y con `BotonFirmar` / `SellarEstaPagina` / `DisparoDelSello`. No lo dejes
   para «luego»: se hace en el mismo turno en que el usuario elige. Es la regla
   que más veces ha habido que repetir («no quiero tener dos paginas en el
   proyecto para las mismas interfaces», 02/09; «el punto 2 te lo he tenido que
   explicar muchas veces», 03/09).

4. **No se dibuja sin permiso.** Cuando la sesión es de preguntas —wayfinder,
   grilling— se pregunta y punto: «solo puedes preguntar, nada de crear
   interfaces sin mi permiso» (02/09). Y cuando algo hay que arreglar,
   **primero se enumeran en texto las opciones que has pensado, y solo después
   se dibuja la elegida**: «antes de rehacer el artboard, dime las opciones q
   has pensado para arreglarlo» (03/09).

5. **La copia del repo es la que se implementa.** Todo lo que se sube al
   proyecto se queda también en `docs/design/artboards/`, 1-1, porque de ahí
   sale la transcripción a JSX «fiel 1-1» y porque el repositorio es público.
   Lo que dibujas tú va **repo → proyecto**; lo que el usuario retoca a mano
   en Claude Design se trae con `/canvas-pull`, y el `<helmet>` sale siempre
   de `_helmet.part`.

## Lo que falta por definir

Preguntas abiertas de esta skill. Mientras no se cierren, hazlo como dice la
columna «por defecto» y **no preguntes en mitad del trabajo**.

| Pregunta | Por defecto hoy |
| -------- | --------------- |
| ¿La fase 1 y la 3 van en subagente o en sesión aparte del usuario? | Subagente `general-purpose` lanzado desde la sesión principal. |
| ¿Cuántas opciones por palanca? | 3; tope 5. Palancas por tanda: **2 como mucho, una por pregunta**. |
| ¿Qué nombre lleva la página de trabajo de `canvas.json`? | `trabajo-<tema>`, y desaparece en la fase 3. |
| ¿La fase 3 comitea en `main` o abre PR? | PR, salvo que el usuario diga «comitea en main». |
| ¿Quién actualiza `README.md` de artboards y su apartado «Lo que cambió en vX»? | La fase 3, en la misma PR. |
| ¿Sigue vivo el registro de canvas de `prototyping.md`? | Sí: `en revisión` al publicar, y la fila se borra al validar. |

## Dos cosas que no son ciertas aunque lo parezcan

- **`docs/design/artboards/` NO se borra al cerrar el #80.** Se mantiene hasta
  la v1.0 ([#80, 03/09/2026](https://github.com/sgomez/rfirma/issues/80#issuecomment-5521081522)).
  Es la copia legible sin cuenta de Claude, y el repositorio es público.
- **El proyecto es `PROJECT_TYPE_PROJECT`, no un design system**, y el tipo es
  inmutable. El flujo de `/design-sync` no aplica: `DesignSync` aquí es solo
  transporte de ficheros.
