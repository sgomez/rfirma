# Diálogo de progreso de firma

Acompaña las tres etapas de la firma trifásica mientras se ejecutan. Bloquea la
ventana porque no hay nada que hacer hasta que termine, y porque interrumpir a
mitad rompe la firma.

## Casos de uso que la usan

- Firmar un PDF en local — al pulsar «Firmar como …», con la sesión del almacén
  ya abierta. Ocupa el mismo sitio que el [diálogo de secreto](dialogo-pin.md),
  que lo precede cuando hace falta.

## Estructura

`.rf-dialog` de 420 px sobre `.rf-scrim` con `z-index: 20`, por encima de la
tira de pestañas:

1. Título «Firmando el documento…».
2. Las tres etapas, cada una con su marca.
3. Barra de progreso fina.

Nada más: no queda ningún gesto que pedirle a quien mira una barra de progreso.

Debajo, la ventana en «firmando»: la hoja al 45 %, los controles de la firma
visible al 35 % y el botón al 55 % diciendo «Firmando como <nombre>».

## Las tres etapas

Lenguaje llano, con el término del dominio entre paréntesis y atenuado para quien
lea un informe de error:

| Etapa | Texto |
| ----- | ----- |
| Prefirma | Preparando la firma *(prefirma)* |
| Firma | Firmando |
| Postfirma | Ensamblando el PDF *(postfirma)* |

«Firmando» no lleva paréntesis: es a la vez la palabra llana y la del dominio.

### Geometría

- Cada etapa es una fila con una casilla de marca de 20 px y su texto.
- **Marcas**: la cumplida, la verificación en `<svg>` de 20 px con trazo 2; la
  en curso, un disco macizo de 10 px en `--rf-primary`; la pendiente, un aro de
  10 px con borde `--rf-border-strong`.
- La etapa en curso va a **peso 700 sin teñir el texto**; la pendiente, con el
  texto en `--rf-text-muted`.
- **Barra**: 4 px, `--rf-radius-pill`, canal `--rf-border-subtle` y relleno
  `--rf-primary`.
- **La palabra de estado** —«Hecha», «En curso», «Pendiente»— va al final de
  cada fila aunque el artboard no la dibuje: la sección 8 del
  [sistema de diseño](design-system.md) prohíbe que la forma sea el único
  indicador.

## Estados

Tres, uno por etapa en curso. No se puede cancelar una vez empezada la firma. Al
terminar, el velo se va y el [panel](panel-de-firma.md) enseña «Firmado» o el
error.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-prose`, `.rf-text-muted`, `--rf-primary`,
`--rf-border-subtle`, `--rf-border-strong`, `--rf-radius-pill`.

## Decisiones

- **Se enseñan las tres etapas** porque la postfirma regenera el PDF entero y
  puede tardar: sin desglose, una espera larga parece un cuelgue. Y cuando falla,
  la fase es lo primero que hace falta; el error del panel la repite en su
  detalle.
- **Diálogo con velo, no etapas en el pie** (25/09/2026). Main v4 D atenuaba la
  ventana sin diálogo; se mantiene el velo para que el secreto, el progreso y el
  paso al resultado ocurran en el mismo sitio.
- **Sin tarjeta.** «Firmando en la tarjeta» y «No retires la tarjeta hasta que
  termine» se fueron con las tarjetas en la v0.4.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`, palanca «Estado: firmando».
