# Fase 3 · Unificar y dejarlo escrito

Este fichero se ejecuta **en contexto propio**, cuando el usuario ya ha
elegido. Recibes las decisiones tomadas; tu trabajo es que no quede rastro de
lo provisional y que la documentación describa lo elegido.

Antes de nada, lee las reglas que no se negocian de [SKILL.md](SKILL.md).

## 1. Fundir

Por cada artboard de trabajo:

- Lleva la decisión **al artboard de la pantalla**, el que ya existía. Si el de
  trabajo era más completo, gana su contenido pero **manda el nombre viejo**:
  `ResumenFirmado` se fundió en `EstadoExito` y desapareció, no al revés.
- La variante elegida pasa a ser el `default` de la palanca. **La palanca se
  queda**: es el registro de la alternativa. El porqué —incluido por qué se
  descartaron las otras— va a la anotación de la página.
- Borra el artboard de trabajo, su página de `canvas.json` y su fila del
  registro. Del repositorio **y** del proyecto, con `delete_files` en el mismo
  `finalize_plan`.

Al terminar, en el proyecto no puede haber dos sitios donde mirar la misma
pantalla. Es el motivo entero de esta fase.

## 2. Dejar la copia 1-1

- `docs/design/artboards/` refleja exactamente lo que hay en el proyecto:
  mismos ficheros, mismo `canvas.json`.
- `./docs/design/artboards/comprueba.sh` en verde.
- Actualiza `docs/design/artboards/README.md`: la tabla de artboards y el
  apartado «Lo que cambió en vX» de la versión en curso.

## 3. Las fichas

Una ficha **por pantalla**, no por flujo: `docs/design/<pantalla>.md`, con la
estructura que fija `docs/agents/prototyping.md` (qué resuelve, casos de uso
que la usan, estructura, estados, componentes y tokens, decisiones). Una
pantalla que aparece en varios flujos tiene una sola ficha que los lista.

- El enlace al lienzo va en «Decisiones», junto a lo que se descartó y por qué.
- Un flujo validado toca **varias** fichas. Repásalas todas, no solo la del
  nudo.
- Si la decisión introduce vocabulario o una regla visual transversal, actualiza
  `docs/design/design-system.md`; si es de arquitectura, `docs/adr/`.
- Marca el canvas como `validado (YYYY-MM-DD)` y **borra su fila** del registro
  de `prototyping.md`: el enlace ya vive en las fichas.

## 4. Cerrar

- Comenta en el ticket o issue la **respuesta**: qué gana, por qué, y enlaces
  al lienzo y a las fichas.
- Commit y PR (o commit directo en `main` si el usuario lo pidió así). Prosa en
  castellano; **sin atribución a Claude ni a ninguna IA** en el mensaje, el
  título ni la descripción.
- Si el usuario pidió dejar fuera algún fichero de la tanda —la propia skill,
  por ejemplo—, respétalo.

## 5. Lo que devuelves

Qué se fundió en qué, qué se borró, qué fichas cambiaron y el enlace a la PR.
En diez líneas.
