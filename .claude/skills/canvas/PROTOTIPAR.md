# Fase 1 · Dibujar con el estilo que ya existe

Este fichero se ejecuta **en contexto propio**. Lo que devuelves a quien te
lanzó es corto: la URL del proyecto, qué artboard toca cada palanca y qué
decide cada opción. Nada de HTML, nada de volcados de fichero.

Antes de nada, lee las reglas que no se negocian de [SKILL.md](SKILL.md).

## 1. Sitúate sin leer de más

- `docs/design/artboards/README.md` tiene la tabla de los catorce artboards con
  su estado. Ahí se decide si la pantalla **ya existe**.
- La ficha `docs/design/<pantalla>.md` dice qué es normativo hoy.
- Del `.dc.html` abre **solo el tramo que vas a tocar** (`grep -n` del texto que
  buscas). Un artboard entero son miles de tokens y no los necesitas.

## 2. El estilo no se inventa

- El `<helmet>` se copia **de `docs/design/artboards/_helmet.part`**, nunca de
  un `get_file` del proyecto: la copia remota se queda atrás. El 02/09/2026
  entraron tres artboards con dos tokens de sombra desfasados por mirar al
  proyecto.
- **Nada de colores literales.** Tokens `--rf-*` y clases del bundle
  (`rfirma-app/src/design-system/bundle/`, que es lo normativo).
- Copia el lenguaje visual del artboard vecino: mismos gaps, mismos tamaños de
  pastilla, mismos rótulos. Un gap distinto del resto se nota y molesta («da
  TOC», 31/08).
- Prosa en castellano, identificadores y nombres de fichero en inglés.

## 3. Las variantes van en palancas, no en artboards

- Palancas (`data-props`), como `Main` y `EstadoElegirCertificado`. **Máximo
  dos por tanda, una por pregunta abierta** («no 4 no, usa tweaks, maximo 2 uno
  para cada pregunta», 03/09).
- Las opciones son **radicalmente distintas**: otra jerarquía y otra acción
  principal, no otro color. Tres por defecto, tope cinco.
- **Incluye siempre la opción «hoy · …»** cuando estés arreglando algo. Sin
  ella el usuario no puede ver el fallo, solo tu propuesta.
- Añade las palancas que hagan **visible el caso que decide**: el zoom que
  esconde el elemento, el pie lleno hasta arriba, el recuadro al tamaño mínimo,
  el nombre de fichero larguísimo. Una variante en reposo no decide nada.

## 4. Comprueba que se ve algo antes de enseñarlo

El fallo más repetido de esta fase no es dibujar mal, es dibujar nada: «no veo
la variante B», «no veo nada», «fuera no se ve nada (la pagina en blanco)», «si
se ve igual pq hay dos opciones». Antes de subir:

- Recorre **cada combinación de palancas** y confirma que cambia algo visible.
  Dos opciones que se ven igual son una opción y sobra la otra.
- Rellena con **contenido realista**: la página del PDF pintada, nombres de
  fichero largos, la lista con suficientes filas para que se vea el scroll.
- Los estados del mismo componente ocupan **lo mismo**: sin botón y con botón,
  el mismo alto, o la interfaz pega saltos (03/09).

## 5. La redacción: menos texto del que te pide el cuerpo

El usuario corrige esto en cada tanda; adelántate.

- **Nada de verborrea.** «Sin firma aún» → «Sin firmar». Fuera las frases que
  explican lo evidente («tu PDF original no se ha tocado») y las instrucciones
  de uso dentro del panel: si hace falta, tooltip, porque el texto come alto.
- **No muestres el estado interno de un componente** («vista previa al día»,
  «refrescando»): no le aporta nada a quien firma.
- **Agrega en vez de enumerar**: no una línea por página, sino «n de m».
- **No nombres nada por su posición relativa** («esta página») si luego el
  usuario puede cambiar de página: di cuál es.
- **No inventes funcionalidad** para rellenar el dibujo («¿pq metes cosas
  nuevas?», 31/08) **ni quites opciones** que ya estaban sin avisar de por qué.

## 6. Publicar

1. Redacta en el repositorio, en `docs/design/artboards/`.
2. `./docs/design/artboards/comprueba.sh`.
3. Sube con `DesignSync`: `list_files` → `finalize_plan` (con `writes` **y**
   `deletes`, obligatorio aunque vaya vacío) → `write_files` / `delete_files`.
4. `canvas.json`: artboards, páginas y anotaciones. La anotación es donde vive
   el razonamiento; escríbela larga y con los números medidos. La página de
   trabajo se llama `trabajo-<tema>` y ya nace condenada.
5. Anota `en revisión` en el registro de `docs/agents/prototyping.md`.

## 7. Lo que devuelves

La URL, la lista de artboards tocados, y **por cada palanca, qué pregunta
responde y qué defiende cada opción, en una línea**. Ese texto es lo que el
usuario va a leer para decidir en la fase 2.
