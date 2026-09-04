# Diálogo de progreso de firma

Acompaña las tres etapas de la firma trifásica mientras se ejecutan. Bloquea la
ventana porque no hay nada que hacer hasta que termine, y porque interrumpir a
mitad rompe la firma.

## Casos de uso que la usan

- Firmar un PDF en local — tras la prefirma, y con la sesión del almacén ya
  abierta.

## Estructura

`.rf-dialog` de 420 px sobre `.rf-scrim`:

1. Título «Firmando el documento…».
2. Lista de tres etapas, cada una con su marca de estado.
3. Barra de progreso fina.

**Y nada más debajo.** Había una cuarta línea, «No retires la tarjeta hasta que
termine», y se retira: la v0.4 saca tarjetas y DNIe del alcance y del dibujo
(ID-201 a ID-204), así que era una instrucción sobre un hardware que no hay. No
se sustituye por otra advertencia: no queda ningún gesto que pedirle a quien
mira una barra de progreso.

## Las tres etapas

Se nombran en lenguaje llano, con el término del dominio entre paréntesis y
atenuado. El usuario no necesita el vocabulario, pero el que lea un informe de
error sí:

| Etapa | Texto |
| ----- | ----- |
| Prefirma | Preparando la firma *(prefirma)* |
| Firma | Firmando |
| Postfirma | Ensamblando el PDF *(postfirma)* |

La etapa de **firma** no lleva paréntesis, y ahora por un motivo más simple que
el de la v0.3: entonces se llamaba «firmando en la tarjeta» y se argumentaba que
esa frase ya decía exactamente lo que pasaba. Sin tarjeta, la etapa se llama
**«Firmando»**, que es a la vez el lenguaje llano y el término del dominio, así
que el paréntesis repetiría la palabra. Sigue siendo la única de las tres que
toca la clave privada.

Marcas de estado: hecha (✓), en curso (círculo relleno en `--rf-primary` y
texto en negrita), pendiente (círculo hueco y texto atenuado).

### Geometría

- Diálogo de 420 px. Cada etapa es una fila con una casilla de marca de 20 px,
  el texto y su palabra de estado.
- **La marca**: la etapa cumplida lleva la verificación en `<svg>` de 20 px con
  trazo 2; la etapa en curso, un disco macizo de 10 px en `--rf-primary`; la
  pendiente, un aro de 10 px con borde `--rf-border-strong`. Son `<svg>` y
  formas maquetadas, no los glifos `✓ ● ○`.
- La etapa en curso va a **peso 700, y su texto no se tiñe**: el `--rf-primary`
  es solo de su marca, el disco macizo. Es lo que dibuja el artboard
  (`EstadoFirmando.dc.html:423`, `font-weight:700` y ningún color) y lo que
  hace el CSS. La pendiente sí lleva el texto en `--rf-text-muted`.
- **Barra**: 4 px de alto, `--rf-radius-pill`, canal `--rf-border-subtle` y
  relleno `--rf-primary`.

**La palabra de estado se queda, aunque el artboard no la dibuje.** El artboard
distingue las tres etapas solo por la forma de su marca, y la sección 8 del
[sistema de diseño](design-system.md) prohíbe que la forma o el color sean el
único indicador. «Hecha», «En curso» y «Pendiente» al final de cada fila son lo
que convierte tres glifos parecidos en tres filas que se leen.

## Estados

Tres, uno por etapa en curso. El diálogo no se puede cancelar una vez empezada
la etapa de firma.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-prose`, `.rf-text-muted`, `--rf-primary`,
`--rf-border-subtle`, `--rf-radius-pill`.

## Por qué se enseñan las tres etapas

La postfirma **regenera el PDF entero** y puede tardar; sin desglose, una
espera larga tras pulsar «Firmar documento» parece un cuelgue. Además, cuando
algo falla, saber en qué fase fue es lo primero que hace falta — el
[panel de firma](panel-de-firma.md) lo repite en el detalle técnico del error.

## Decisiones

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «8 · Firmando».

La retirada de la tarjeta —del rótulo de la etapa y de la línea de aviso— se
decidió en el [#250](https://github.com/sgomez/rfirma/issues/250) (ID-201 a
ID-204) y está dibujada en ese mismo artboard.
