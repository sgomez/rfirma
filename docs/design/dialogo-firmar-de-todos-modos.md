# Diálogo «¿Firmar de todos modos?»

Pide confirmación, **justo antes de firmar**, cuando el documento ya trae alguna
firma que no es válida. Es un paso de confirmación, no un bloqueo: las dos
salidas están siempre.

## Casos de uso que la usan

- Firmar un PDF en local — al pulsar «Firmar como …» en el
  [panel de firma](panel-de-firma.md), y **solo** si alguna firma previa no es
  válida: certificado caducado, certificado aún no válido, firma rota o no se
  puede validar.

No lo abren el cambio después de la última firma ni la firma que no se ha podido
comprobar del todo: suben el tono del aviso de firmas previas, pero no son
firmas no válidas. Tampoco lo abre la [ventana de sede](ventana-de-sede.md), que
informa y no pide confirmación.

## Estructura

`.rf-dialog` de 420 px sobre `--rf-scrim`. El velo tapa la ventana principal
entera, cabecera y tira de pestañas incluidas, y por eso lleva `z-index:20`.
Debajo, la ventana en «listo» con el aviso de firmas previas desplegado y el
botón del pie normal. De arriba abajo:

1. **Título** en `.rf-title`: «¿Firmar de todos modos?».
2. **La frase con el recuento**, en `.rf-prose`: «Este documento lleva **N**
   firmas que no son válidas» («1 firma que no es válida» en singular).
3. **La lista de las firmas no válidas**, en un bloque con borde de 1 px en
   `--rf-border-subtle` y `--rf-radius-md`: una fila por firma con quién, cuándo,
   el veredicto con su icono —los mismos del aviso de firmas previas— y el motivo
   debajo en `--rf-text-muted`. Las válidas no se repiten: ya están en el aviso.
4. **La frase que no se negocia**, en `.rf-prose`: «Si lo firmas, la sede u
   organismo que reciba el documento **puede rechazarlo**».
5. Un `.rf-divider` y **dos salidas** alineadas a la derecha: `Cancelar` en
   `--ghost` y `Firmar de todos modos` en `--primary`, como en el
   [diálogo del PIN](dialogo-pin.md).

## Estados

En el artboard `EstadoFirmarDeTodosModos`, palanca «motivo»:

- **Una firma no válida** (certificado caducado, aún no válido, rota o no se
  puede validar): una fila.
- **Varias** («todo a la vez»): una fila por cada una.
- **Muchas** («muchas no validas (7)»): la lista tiene un **alto máximo de
  264 px**, unas tres filas y media, y se desplaza por dentro; el diálogo no
  crece más. La cuarta fila queda cortada a media altura, que es lo que dice que
  hay más, sin sombras ni rótulos añadidos.

`Cancelar` vuelve al panel sin firmar. `Firmar de todos modos` sigue el
recorrido de siempre.

## Componentes y tokens

`.rf-scrim`, `.rf-dialog`, `.rf-title`, `.rf-prose`, `.rf-body`,
`.rf-divider`, `.rf-btn--primary|--ghost`, `--rf-border-subtle`,
`--rf-text`, `--rf-text-muted`, `--rf-radius-md`.

## Decisiones

- **La frase y solo las firmas no válidas.** Las válidas ya están en el aviso
  del panel.
- **«Firmar de todos modos» es la acción principal** y `Cancelar` va en
  fantasma.
- **El botón del pie del panel no cambia**: sigue siendo «Firmar como …», y es
  este diálogo el que pregunta.
- **No sustituye** a la pregunta de las firmas que rFirma no reconoce, que sigue
  como está.

Validado el **26/09/2026** en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `EstadoFirmarDeTodosModos`.
