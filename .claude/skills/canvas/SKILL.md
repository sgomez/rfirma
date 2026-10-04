---
name: canvas
description: Explorar interfaz de rFirma en Claude Design con los componentes reales y llevar lo validado a código. Úsala cuando prototype, grill-with-docs o wayfinder lleguen a una rama de UI.
---

# Explorar en Claude Design

`docs/agents/prototyping.md` es el contrato. Esta skill es el procedimiento.

## Cuándo merece la pena explorar

Solo cuando hay una decisión de aspecto que mirar antes de escribir código: una
pantalla nueva, o una variante radicalmente distinta de una existente. Un ajuste
con la respuesta clara en la ficha o en el sistema de diseño va directo a
código.

## Antes de explorar

Lanza `/design-sync`. Sube a «rFirma Components» (`312bca0c-2f94-494a-820a-e947e03f9ade`)
los componentes y las historias tal como están en `main`; sin eso se explora
sobre piezas viejas. Se explora allí, con los componentes reales, no dibujando
HTML a mano.

## De lo validado a código

Cuando la persona usuaria da por buena una variante:

1. **Componente**: se escribe o se cambia en `rfirma-app/src/`, con los tokens y
   las clases de `docs/design/design-system.md`.
2. **Historias**: una por variante, junto al componente (ADR-0045).
3. **Ficha**: `docs/design/<pantalla>.md` cuenta el flujo entre estados y el
   porqué de cada decisión, y cita historias y claves i18n.

El prototipo de Claude Design no se versiona ni se copia al repositorio.
