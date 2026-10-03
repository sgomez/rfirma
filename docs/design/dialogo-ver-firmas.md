# Diálogo «Ver firmas»

Enseña las firmas que ya trae el documento, con su validez, **antes de
firmar**. Se mira y se cierra: no firma ni pregunta nada.

## Casos de uso que la usan

- Firmar un PDF en local — al pulsar «Ver firmas →» en el aviso de firmas
  previas del [panel de firma](panel-de-firma.md#el-aviso-de-firmas-previas).
- Firmar a petición de una sede — al pulsar «Ver firmas →» en la caja del
  documento del consentimiento de la [ventana de sede](ventana-de-sede.md). Es
  el mismo diálogo: no se dibuja otro sobre la sede.

## Estructura

`.rf-dialog` de 420 px sobre `--rf-scrim`, con `z-index:20` para tapar también
la cabecera y sus pestañas, y un alto máximo del de la ventana menos 48 px.
Debajo, la ventana principal en «listo», con el aviso compacto en el panel. De
arriba abajo:

1. **«Firmas del documento»** en `.rf-title` y, debajo, el formato y el
   recuento en `.rf-body` `--rf-text-muted`: «PAdES · 3 firmas».
2. **La zona que se desplaza**, la única que lo hace, con 12 px entre bloques:
   - **Los hallazgos del documento**, uno por línea y encima de las firmas:
     el círculo con aspa y el texto a 13 px y peso 700, con borde de 2 px en
     `--rf-border-strong` y `--rf-radius-md`. No se cuelgan de ninguna firma.
   - **Una ficha por firma**, las mismas del
     [resumen](panel-de-firma.md#el-resumen-tras-firmar-y-con-verify---gui),
     apiladas a 6 px. Las describe el panel de firma: cabecera «FIRMA N» con la
     validez a la derecha y filas de Firmante, En nombre de, Emisor,
     «Fecha» o «Sellada» y, si no es válida, Motivo.
3. Un `.rf-divider` y **una sola salida**: «Cerrar», `.rf-btn--primary` de
   96 px de ancho mínimo, a la derecha, como en [Acerca de](acerca-de.md).

No hay «Firmar» ni pie con el documento: quien quiere firmar cierra y pulsa
«Firmar» en el panel.

## Estados

En el artboard `EstadoVerFirmas`, palanca «Firmas previas». Por defecto, el
ejemplo «valida, caducada, no admitia mas firmas y hallazgo (3)»: una firma
válida que no admite más firmas, una caducada, una no válida con el motivo «Ana
López García no admitía más firmas» y el hallazgo «Se ha modificado después de
la última firma».

- **Todas válidas**: solo fichas, cada una con la marca en gris.
- **Con caducadas, no válidas o hallazgos**: los hallazgos encima y cada ficha
  con su validez y su motivo.
- **Formato desconocido**: la firma que rFirma no reconoce es una ficha más,
  «No válida», con el motivo «rFirma no conoce este tipo de firma».
- **Ya la firmaste tú**: la ficha es como las demás; la franja «Ya lo firmaste
  tú» es del aviso del panel, no del diálogo.
- **Extremo** («extremo · todos los motivos y hallazgos (7)»): los seis
  motivos y los tres hallazgos. La zona central se desplaza; el título y
  «Cerrar» no se mueven.

«Cerrar» devuelve a la pantalla de la que se vino, tal como estaba.

## Componentes y tokens

`.rf-scrim`, `.rf-dialog`, `.rf-title`, `.rf-body`, `.rf-text-muted`,
`.rf-card`, `.rf-label`, `.rf-badge--primary`, `.rf-divider`,
`.rf-btn--primary`, `--rf-text`, `--rf-text-muted`, `--rf-border-strong`,
`--rf-radius-md`. Las fichas y los hallazgos salen de
`docs/design/artboards/_fichas-firmas.part`, el mismo fragmento del resumen.

## Decisiones

- **Un diálogo y no un aviso desplegable.** El aviso de firmas previas se
  desplegaba en el panel con una fila por firma y su motivo; ahora es una
  línea, y el detalle vive aquí. En un panel de 380 px la lista desplegada
  empujaba la firma visible fuera de la vista, y el motivo no cabía junto a
  quién y cuándo.
- **Las mismas fichas que el resumen.** Una firma se ve igual antes y después
  de firmar: la validez es la misma en todas partes (ADR-0043).
- **Solo «Cerrar».** Se mira y se cierra; la confirmación antes de firmar con
  algún problema la hace [«¿Firmar de todos
  modos?»](dialogo-firmar-de-todos-modos.md).
- **Monocromo.** Ni ámbar para la caducada ni rojo para la no válida: la
  paleta no tiene tokens para eso ([design-system.md](design-system.md#2-color)).
  Lo que distingue las tres valideces es la silueta, la palabra y el peso.

Validado el **03/10/2026** en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `EstadoVerFirmas`; el razonamiento, en
la anotación `nota-validez`.
