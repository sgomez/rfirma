# Diálogo de páginas sin firma visible

Avisa, **justo antes de firmar**, de que el recuadro de la firma visible no cabe
en algunas de las páginas elegidas y que esas se quedarán sin él. Es el único
aviso que hay de una degradación que, sin él, ocurriría en silencio.

Componente: `rfirma-app/src/signing/UnsealedPagesDialog.tsx`. Historias:
`UnsealedPagesDialog.stories.tsx`, en «Flujos/Firma/UnsealedPagesDialog».

## Casos de uso que lo usan

- Firmar un PDF en local, entre pulsar «Firmar» y el [diálogo del
  PIN](dialogo-pin.md), **solo** si la firma visible está en «Varias» o «Todas»
  y el conjunto de páginas incluye alguna donde el recuadro no cabe.

## Flujo

Se abre desde el [panel de firma](panel-de-firma.md) al pulsar «Firmar». Tiene
dos salidas y las dos están siempre:

- **Cancelar** (o Escape) vuelve al panel sin firmar, tal como estaba.
- **Firmar de todos modos** sigue el recorrido de siempre, hacia el secreto del
  almacén y la firma.

No es un paso del recorrido: aparece solo cuando hay páginas que se caen.

## Variantes

Las historias recorren lo único que varía, cuántas páginas se caen:

- `SeveralPages`: varias, el caso por defecto.
- `OnePage`: la forma singular del titular, que es la que hay que cuidar.
- `ManyPages`: muchas, para comprobar que el diálogo no crece, porque no las
  nombra.

## Textos

Claves del catálogo: `sealLoss.title` (con plural por `count`), `sealLoss.body`,
`actions.cancel` y `actions.signAnyway`. Los textos no se copian aquí: se leen
en `po/es.po`.

## Componentes y tokens

El primitivo `Dialog` (con `Escape`, foco y trampa de foco), `Button` y `Row`,
`AlertIcon`, `.rf-title`, `.rf-prose` y `.rf-divider`. El ancho de 460 px es de
`UnsealedPagesDialog.css`.

## Decisiones

- **Modal y no un aviso permanente en el panel.** Se pensó una tira bajo el
  visor que marcara las páginas que se caen, sin bloquear, más este modal. La
  tira se descartó (ver [visor de documento](visor-de-documento.md)) y el modal
  quedó como único aviso. Se acepta: es el momento en que la información
  importa, y un aviso fijo para un caso poco frecuente es ruido los demás días.
- **Por qué hace falta.** `correctPositionSignature` recorta el recuadro contra
  la primera página de la lista y descarta en silencio aquellas donde no cabe su
  esquina inferior izquierda
  ([medido aquí](../research/ancla-y-paginas-en-el-puente.md)). No se puede
  impedir sin más, porque firmar el documento entero sigue siendo lo que la
  persona ha pedido; lo honesto es contarlo y dejar decidir.
- **Sigue haciendo falta con la firma visible apagada por defecto**: con «Varias»
  o «Todas» la biblioteca descarta igual las páginas donde no cabe.
- **«Sin firma visible», nunca «recortadas».** Recortar sugiere que algo se
  estampa a medias; lo que ocurre es que en esas páginas no se estampa nada. La
  firma criptográfica no se recorta jamás: cubre el documento entero, y
  confundir las dos cosas es lo peor que puede hacer este diálogo.
- **No dice «error».** Es una consecuencia de la geometría del documento, y la
  salida principal es seguir.
- **Las páginas se cuentan, no se nombran.** Se descartaron un bloque de fichas
  numeradas, una fila de miniaturas y nombrarlas en la frase: con doce, una
  lista de números es una pared que no ayuda a decidir, porque la decisión no
  depende de cuáles son sino de cuántas. Mirar páginas se hace antes, en el
  visor.
- **El denominador es el conjunto elegido, no el documento.** Con 13 páginas
  elegidas de un PDF de 27 se habla de las 13, nunca de las 27: lo otro solo
  sería cierto con todas elegidas.
- **«Firmar de todos modos» es la acción principal.** La persona ha llegado
  hasta aquí para firmar; el diálogo informa, no disuade.
- **Escape equivale a Cancelar**, como en todos los diálogos con salida.
