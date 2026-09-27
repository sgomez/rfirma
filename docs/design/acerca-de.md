# Acerca de rFirma

Identidad de la aplicación, **estado de la versión**, aviso de independencia
respecto del cliente oficial y licencias. Se abre desde el menú de la
[cabecera](cabecera.md) y desde la acción de la franja de notificación de la
[ventana principal](ventana-principal.md).

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido.
- Actualizar rFirma — es la única pantalla que dice si hay versión nueva.

## Estructura

`.rf-dialog` de 460 px sobre `.rf-scrim`, sin relleno propio y en tres zonas:

1. **Cabecera**: `rFirma` en `.rf-heading` con la versión en un `.rf-badge` a
   su derecha; debajo, **qué hace** en una frase atenuada, y la tarjeta del
   **estado de la versión**.
2. **Ficha técnica**, en una lista de dos columnas: «Licencia», «Firma con» y
   «Código fuente».
3. **Aviso de independencia**, tras un filete, y el **pie** con «Ver las
   licencias» (`--secondary`) y «Cerrar» (`--primary`).

**La frase de qué hace se ha recortado dos veces.** Decía «con tu certificado
**o tu tarjeta criptográfica**», que deja de ser cierto en la v0.4 (ID-201 a
ID-204); y decía «el documento y la clave privada no salen de tu ordenador»,
que se retira porque tranquiliza sobre lo evidente y no cambia nada de lo que la
persona puede hacer. Lo que la garantía significa de verdad —que la clave
privada nunca cruza a Java— está donde tiene que estar, en el
[ADR-0001](../adr/0001-firma-trifasica-clave-privada-solo-en-rust.md).

## Estado de la versión

**Una tarjeta, y nada más.** Quien tiene el diálogo delante **ya ha instalado
rFirma**, así que las órdenes de alta del repositorio —que estuvieron aquí
hasta la v0.4— no le dicen nada que le sirva para actualizar: son las de
instalar lo que ya tiene. Quien se dio de alta actualiza con su gestor de
paquetes sin volver a esta pantalla, y quien no, encuentra esas órdenes donde
se instala, en `packaging/repo/index.html`.

Con ellas se fueron el selector de tres canales, el bloque de órdenes, su botón
«Copiar» y las tres cadenas del catálogo (`about.update.channel.*`).

No hay botón de descarga.

## El aviso de independencia

> **Proyecto independiente.** rFirma no está relacionada con AutoFirma ni con
> la Administración General del Estado, que publican el cliente oficial, ni
> cuenta con su respaldo. Si necesitas la aplicación oficial, descárgala de su
> web.

Va al pie del cuerpo, tras un filete `--rf-border-subtle`, con un icono de
información atenuado a la izquierda: «Proyecto independiente.» en negrita y el
resto en `.rf-text-muted`. Tiene que estar: una aplicación que firma ante la
Administración con la misma criptografía que la oficial se puede confundir con
ella, y esa confusión hay que deshacerla en el sitio donde la gente va a
preguntar qué es esto.

## Licencias y código fuente

| Término | Valor |
| --- | --- |
| Licencia | EUPL-1.2 |
| Firma con | Bibliotecas del proyecto Cliente @firma, y debajo, en `.rf-hint`, GPL-2.0+ / EUPL-1.1 |
| Código fuente | `github.com/sgomez/rfirma`, **enlace que se pulsa** y abre el navegador, con su icono de abrir fuera |

Ver [ADR-0008](../adr/0008-licencia-eupl-1-2.md) para por qué esa combinación
se sostiene.

### Geometría

- Diálogo de **460 px** sin relleno: cada zona lleva el suyo. Fueron 520
  mientras el bloque de órdenes estuvo dentro, porque las del `.deb` envolvían
  a cuatro renglones.
- **Cabecera**: `--rf-space-md` arriba y a los lados, `--rf-space-sm` entre sus
  piezas. El nombre en `.rf-heading` **bajado a 32 px**, alineado por la línea
  base con el `.rf-badge` de la versión; la frase de qué hace en
  `.rf-prose rf-text-muted`.
- **Tarjeta de versión**: `--rf-surface`, `--rf-radius-md`, 12 px ×
  `--rf-space-sm` de relleno, icono de 18 px dentro de un círculo y el texto a
  14 px.
- **Ficha técnica**: rejilla de dos columnas con `--rf-space-md` entre ellas,
  14 px entre filas y `--rf-space-md` de relleno; términos en `.rf-label`,
  valores a 13 px.
- **Aviso**: margen `--rf-space-md`, `--rf-space-sm` sobre el filete, icono de
  16 px y texto a 13 px.
- **Pie**: `--rf-surface` con filete superior, 12 px × `--rf-space-md`,
  botones a la derecha separados `--rf-space-xs`; «Cerrar» de 96 px como
  mínimo.

## Estados

**Dos, y se diferencian sólo en la tarjeta:**

- **Hay una versión nueva**: borde `--rf-border-strong`, flecha hacia arriba
  dentro de un círculo y «Hay una versión nueva: 0.4.1» en negrita.
- **Al día**: borde `--rf-border-subtle`, marca de verificación atenuada dentro
  de un círculo y «Estás en la última versión».

Sin red **no hay tercer estado**: se calla. Un «no se ha podido comprobar» sería
un fallo que no le pide nada a nadie.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-heading`, `.rf-badge`, `.rf-prose`,
`.rf-label`, `.rf-hint`, `.rf-text-muted`, `.rf-btn--secondary|--primary`;
`--rf-surface`, `--rf-border-subtle|-strong`, `--rf-radius-md`,
`--rf-space-xs|-sm|-md`.

## Decisiones

El diálogo se rehízo en el lienzo en tres zonas, con la versión en tarjeta, la
ficha técnica siempre a la vista, la dirección del repositorio como enlace que
se pulsa y el aviso de independencia con icono. Es decisión del titular sobre
el lienzo y **revoca dos criterios anteriores** de esta ficha: que el aviso
fuera un párrafo sin icono, para no darle aire de alarma, y que no hubiera
ningún enlace que se pulsara mientras `opener:deny-open-url` siguiera
denegado.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «Acerca de · desde el menú», con la palanca
**Estado de la versión**; la de **Canal** se retiró del artboard.

El canal propio y sus tres repositorios están en el
[ADR-0015](../adr/0015-canal-de-distribucion-propio.md); las órdenes de alta,
en `packaging/repo/index.html`.
