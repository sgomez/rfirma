# Diálogo de páginas sin sello

Avisa, **justo antes de firmar**, de que el recuadro no cabe en algunas de las
páginas elegidas y que esas se quedarán sin sello. Es la única defensa contra
una degradación que ocurriría en silencio.

## Casos de uso que la usan

- Firmar un PDF en local — entre pulsar «Firmar documento» y el diálogo de PIN,
  y **solo** si el conjunto de páginas incluye alguna donde el recuadro no cabe.

## Por qué existe

`correctPositionSignature` recorta el recuadro contra la **primera** página de
la lista y **descarta en silencio** aquellas donde no cabe la esquina inferior
izquierda. Medido en
[ancla-y-paginas-en-el-puente.md](../research/ancla-y-paginas-en-el-puente.md).

O sea que «dejar pasar» no significa no hacer nada: significa **perder páginas
sin decirlo**, que es exactamente la degradación que el ID-22 rechaza. Y no se
puede impedir sin más, porque firmar el documento entero sigue siendo lo que la
persona ha pedido y la firma será válida en todo él. Lo único honesto es
contarlo y dejar decidir.

## Estructura

Diálogo de 460 px sobre `--rf-scrim`, `--rf-radius-xl`, `--rf-shadow-elevated`.
De arriba abajo:

1. **Titular** con el triángulo de aviso de 24 px y la cifra dentro: «3 páginas
   se quedarán sin sello», o «Una página se quedará sin sello».
2. **Cuerpo** en `.rf-prose`: qué pasa, por qué, y qué **no** pasa —«El recuadro
   no cabe en 3 de las 13 páginas que has elegido, más pequeñas que aquella
   sobre la que lo colocaste. El documento se firmará igual y la firma será
   válida en todo él, pero en esas páginas no aparecerá el sello».
3. **La vuelta positiva**, en un bloque con borde `--rf-border-subtle` y fondo
   `--rf-bg`, con el icono del sello: «El sello aparecerá en 10 de las 13
   páginas elegidas». Es el dato que de verdad decide, y por eso no va escondido
   en la prosa.
4. **Dos salidas**, alineadas a la derecha: `Cancelar` fantasma y
   `Firmar de todos modos` primario.

## Vocabulario

**«Sin sello», nunca «recortadas».** Recortar sugiere que algo se estampa a
medias; lo que ocurre es que en esas páginas no se estampa nada. La firma
criptográfica no se recorta jamás: cubre el documento entero pase lo que pase, y
confundir las dos cosas es lo peor que puede hacer este diálogo.

**No dice «error».** No lo es: es una consecuencia de la geometría del
documento, y la salida principal es seguir adelante.

## Las páginas no se nombran: se cuentan

**Nunca una a una. Siempre el total, *n* de *m*.**

Se probaron las tres salidas: un bloque de fichas numeradas, una fila de
miniaturas de las páginas que se caen, y nombrarlas dentro de la frase. Las tres
se descartan por el mismo motivo, que se ve con doce: una lista de doce números
no es información, es una pared que nadie lee y que además no ayuda a decidir,
porque la decisión —seguir o cancelar— no depende de *cuáles* son sino de
*cuántas*. El bloque de fichas y la fila de miniaturas añaden encima una región
al diálogo para repetir lo que la frase ya dice, y las miniaturas prometen una
inspección que aquí no toca: el momento de mirar páginas es antes, en el visor.

### El denominador es el conjunto elegido, no el documento

*m* son **las páginas que la persona ha elegido**, no las que tiene el PDF. Si
el documento tiene 27, se han elegido 13 y se caen 3, el diálogo dice «3 de las
13 páginas que has elegido» y «el sello aparecerá en 10 de las 13» — nunca «24
de las 27», que sería cierto solo cuando se han elegido todas y en los demás
casos es sencillamente falso.

## Estados

Uno solo. La variación es cuántas páginas se caen y de cuántas elegidas, y
afecta al titular («Una página» / «3 páginas»), al cuerpo y al recuento. La
única forma singular que hay que cuidar es la de una sola página, en el titular
y en el recuento.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-title`, `.rf-prose`, `.rf-body`, `.rf-label`,
`.rf-btn--primary|--ghost`, `--rf-radius-xl`, `--rf-radius-md`,
`--rf-border-subtle`, `--rf-bg`, `--rf-shadow-elevated`.

## Decisiones

- **Modal y no un aviso en el panel.** Se decidió en el
  [#152](https://github.com/sgomez/rfirma/issues/152) que las páginas donde no
  cabe se marcarían en una tira bajo el visor **sin bloquear**, más este modal.
  Al descartarse la tira (ver [visor de documento](visor-de-documento.md)) se
  cayó la marca, y este diálogo pasó a ser el **único** aviso. Se acepta: es el
  momento en que la información importa, y un aviso permanente en el panel para
  un caso poco frecuente es ruido los otros días.
- **Aparece solo cuando hay páginas que se caen.** No es un paso del recorrido.
- **`Firmar de todos modos` es el primario.** La persona ha llegado hasta aquí
  para firmar; el diálogo informa, no disuade.

Validado en el canvas
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard «5b · Antes de firmar · páginas sin
sello», con la palanca «Cuántas se caen» (1, 3 y 12 de 13 elegidas, más 3 de 27
con todas elegidas). Decidido el 02/09/2026 en el
[#155](https://github.com/sgomez/rfirma/issues/155).
