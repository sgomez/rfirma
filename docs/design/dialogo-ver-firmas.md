# Diálogo «Ver firmas»

Enseña las firmas que ya trae el documento, con su validez, **antes de
firmar**. Se mira y se cierra: no firma ni pregunta nada.

Componente: `rfirma-app/src/signing/SignaturesDialog.tsx`, con las fichas de
`SignatureCards.tsx`. Historias: `SignaturesDialog.stories.tsx`, en «Diálogos de
firma/3 · Ver firmas».

## Casos de uso que lo usan

- Firmar un PDF en local, al pulsar «Ver firmas →» en el aviso de firmas
  previas del [panel de firma](panel-de-firma.md#el-aviso-de-firmas-previas).
- Firmar a petición de una sede, al pulsar «Ver firmas →» en la caja del
  documento del consentimiento de la [ventana de sede](ventana-de-sede.md). Es
  el mismo diálogo: no se dibuja otro sobre la sede.

## Flujo

Una sola salida, «Cerrar» (o Escape), que devuelve a la pantalla de la que se
vino, tal como estaba. No hay «Firmar» ni pie con el documento: quien quiere
firmar cierra y pulsa «Firmar» en el panel.

## Qué enseña

De arriba abajo:

1. **El título y, debajo, el formato y el recuento**, atenuados («PAdES · 3
   firmas»; sin formato cuando no se reconoce).
2. **La zona que se desplaza**, la única que lo hace:
   - **Los hallazgos del documento**, uno por línea y encima de las firmas, con
     el icono de no válida y borde fuerte. No se cuelgan de ninguna firma.
   - **Una ficha por firma**, las mismas del
     [resumen](panel-de-firma.md#el-resumen-tras-firmar-y-con-verify---gui),
     apiladas: cabecera con el número y la validez, y filas de firmante, en
     nombre de (solo con representación), emisor, fecha declarada o sellada y,
     si no es válida, motivo. La que cierra el documento lo dice, y las
     contrafirmas van anidadas con su propia validez.
3. **El filete y «Cerrar»**, primario, a la derecha.

El diálogo tiene un alto máximo del de la ventana menos un margen; el título y
«Cerrar» no se mueven cuando la zona central se desplaza.

## Variantes

- `FindingAndSignatures`: una válida que cierra el documento, una caducada, una
  no válida con el motivo de que la anterior no admitía más firmas, y un
  hallazgo.
- `AllValid`: solo fichas, sin hallazgos.
- `UnrecognizedFormat`: la firma que rFirma no reconoce es una ficha más, no
  válida, y el título lleva solo el recuento.
- `EveryReasonAndFinding`: los siete motivos y los tres hallazgos, que enseña el
  desplazamiento.

«Ya la firmaste tú» no es una variante de este diálogo: esa franja es del aviso
del panel.

## Textos

Claves: `panel.signed.title`, `panel.signed.count`, `panel.signed.*` de las
fichas, `actions.close`, `documentFinding.*` y `signatureReason.*`. Los textos no
se copian aquí: se leen en `po/es.po`.

## Componentes y tokens

El primitivo `Dialog` (con `Escape`, foco y trampa de foco), `Button` y `Row`,
`.rf-title`, `.rf-body`, `.rf-text-muted`, `.rf-card`, `.rf-label`,
`.rf-divider`, `--rf-border-strong` y `--rf-radius-md`. Las medidas son de
`SignaturesDialog.css` y `SignatureCards.css`.

## Decisiones

- **Un diálogo y no un aviso desplegable.** El aviso del panel se desplegaba con
  una fila por firma y su motivo; ahora es una línea y el detalle vive aquí. En
  un panel estrecho la lista desplegada empujaba la firma visible fuera de la
  vista, y el motivo no cabía junto a quién y cuándo.
- **Las mismas fichas que el resumen.** Una firma se ve igual antes y después de
  firmar: la validez es la misma en todas partes (ADR-0043).
- **Solo «Cerrar».** Se mira y se cierra; la confirmación antes de firmar con
  algún problema la hace [«¿Firmar de todos
  modos?»](dialogo-firmar-de-todos-modos.md).
- **Monocromo.** Ni ámbar para la caducada ni rojo para la no válida: la paleta
  no tiene tokens para eso ([design-system.md](design-system.md#2-color)). Lo
  que distingue las tres valideces es la silueta, la palabra y el peso.
