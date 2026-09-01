# Diálogo de progreso de firma

Acompaña las tres etapas de la firma trifásica mientras se ejecutan. Bloquea la
ventana porque no hay nada que hacer hasta que termine, y porque retirar la
tarjeta a mitad rompe la firma.

## Casos de uso que la usan

- Firmar un PDF en local — tras aceptar el PIN.

## Estructura

`.rf-dialog` de 420 px sobre `.rf-scrim`:

1. Título «Firmando el documento…».
2. Lista de tres etapas, cada una con su marca de estado.
3. Barra de progreso fina.
4. «No retires la tarjeta hasta que termine».

## Las tres etapas

Se nombran en lenguaje llano, con el término del dominio entre paréntesis y
atenuado. El usuario no necesita el vocabulario, pero el que lea un informe de
error sí:

| Etapa | Texto |
| ----- | ----- |
| Prefirma | Preparando la firma *(prefirma)* |
| Firma | Firmando en la tarjeta |
| Postfirma | Ensamblando el PDF *(postfirma)* |

La etapa de **firma** no lleva paréntesis: «firmando en la tarjeta» ya dice
exactamente lo que pasa, y es la única de las tres que toca la clave privada.

Marcas de estado: hecha (✓), en curso (círculo relleno en `--rf-primary` y
texto en negrita), pendiente (círculo hueco y texto atenuado).

### Geometría

- Diálogo de 420 px. Cada etapa es una fila con una casilla de marca de 20 px,
  el texto y su palabra de estado.
- **La marca**: la etapa cumplida lleva la verificación en `<svg>` de 20 px con
  trazo 2; la etapa en curso, un disco macizo de 10 px en `--rf-primary`; la
  pendiente, un aro de 10 px con borde `--rf-border-strong`. Son `<svg>` y
  formas maquetadas, no los glifos `✓ ● ○`.
- La etapa en curso va a peso 700 y en `--rf-primary`; la pendiente, en
  `--rf-text-muted`.
- **Barra**: 4 px de alto, `--rf-radius-pill`, canal `--rf-border-subtle` y
  relleno `--rf-primary`.

**La palabra de estado se queda, aunque el artboard no la dibuje.** El artboard
distingue las tres etapas solo por la forma de su marca, y la sección 8 del
[sistema de diseño](design-system.md) prohíbe que la forma o el color sean el
único indicador. «Hecha», «En curso» y «Pendiente» al final de cada fila son lo
que convierte tres glifos parecidos en tres filas que se leen.

## Estados

Tres, uno por etapa en curso. El diálogo no se puede cancelar una vez empezada
la firma en la tarjeta.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-prose`, `.rf-text-muted`, `--rf-primary`,
`--rf-border-subtle`, `--rf-radius-pill`.

## Por qué se enseñan las tres etapas

La postfirma **regenera el PDF entero** y puede tardar; sin desglose, una
espera larga tras teclear el PIN parece un cuelgue. Además, cuando algo falla,
saber en qué fase fue es lo primero que hace falta — el
[panel de firma](panel-de-firma.md) lo repite en el detalle técnico del error.

## Decisiones

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «8 · Firmando».
