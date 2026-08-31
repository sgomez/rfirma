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
