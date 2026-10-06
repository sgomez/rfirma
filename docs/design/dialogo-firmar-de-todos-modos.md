# Diálogo «¿Firmar de todos modos?»

Pide confirmación, **justo antes de firmar**, cuando el documento ya trae algún
problema: una firma caducada, una no válida o un hallazgo del documento. Es un
paso de confirmación, no un bloqueo: las dos salidas están siempre.

Componente: `rfirma-app/src/signing/SignAnywayDialog.tsx`. Historias:
`SignAnywayDialog.stories.tsx`, en «Flujos/Firma/SignAnywayDialog». Los ejemplos de firmas previas que usan las historias están en
`rfirma-app/src/signing/testing/storyReports.ts`.

## Casos de uso que lo usan

- Firmar un PDF en local, al pulsar «Firmar» en el [panel de
  firma](panel-de-firma.md), **solo** si hay algún ⚠ o ✗: una firma caducada,
  una no válida —también la que rFirma no reconoce— o un hallazgo del
  documento.

No lo abre la [ventana de sede](ventana-de-sede.md), que ya es un
consentimiento: informa y no pide confirmación.

## Flujo

- **Cancelar** (o Escape) vuelve al panel sin firmar.
- **Firmar de todos modos** sigue el recorrido de siempre.

El botón del pie del panel no cambia: sigue diciendo «Firmar», y es este diálogo
el que pregunta.

## Qué enseña

De arriba abajo: el título, una fila por problema, una advertencia para cerrar,
el filete y las dos salidas.

- **Primero los hallazgos del documento**, con el icono de no válida y sin
  segunda línea.
- **Después cada firma caducada o no válida**, con el icono de su validez, su
  número y su firmante, y debajo el motivo, atenuado y sangrado para que caiga
  bajo el texto.
- **Las válidas no salen.** Tampoco hay párrafo con el recuento: la lista ya lo
  cuenta.
- La lista tiene un alto máximo y se desplaza por dentro; el diálogo no crece
  más. La última fila visible queda cortada, que es lo que dice que hay más.

## Variantes

Las historias cubren cada forma de problema:

- `FindingAndSignatures`: un hallazgo, una caducada y una no válida.
- `OnlyExpired`: solo caducadas, con el triángulo.
- `UnrecognizedFormat`: la firma que rFirma no reconoce es una fila más.
- `EveryReasonAndFinding`: los siete motivos y los tres hallazgos, que enseña el
  desplazamiento.

## Textos

Claves: `signAnyway.title`, `signAnyway.signature`, `signAnyway.warning`,
`actions.cancel` y `actions.signAnyway`; los hallazgos, `documentFinding.*`, y
los motivos, `signatureReason.*`. Los textos no se copian aquí: se leen en
`po/es.po`.

## Componentes y tokens

El primitivo `Dialog` (con `Escape`, foco y trampa de foco), `Button` y `Row`,
`CrossCircleIcon` y `AlertIcon` (los iconos de validez de
[design-system.md](design-system.md#8-accesibilidad)), `.rf-title`, `.rf-prose`,
`.rf-body`, `.rf-text-muted` y `.rf-divider`. El ancho y el alto máximo de la
lista son de `SignAnywayDialog.css`.

## Decisiones

- **Lo abre cualquier problema**, no solo una firma no válida: la caducada y el
  hallazgo del documento también. «Ha cambiado después de la última firma» era
  solo un tono del aviso y ahora es un hallazgo que pesa como una firma no
  válida (ADR-0043).
- **Absorbe el diálogo de las firmas que rFirma no reconoce.** Una firma
  ilegible o de formato desconocido es *no válida*, con su motivo, y no tiene
  pregunta propia: la cofirma pide el consentimiento aquí, en una fila más.
- **Una fila por problema y sin párrafo de recuento.** Repetía lo que la lista
  ya cuenta y no servía para hallazgos ni caducadas.
- **Las válidas no se repiten**: están en [«Ver firmas»](dialogo-ver-firmas.md).
- **La advertencia es corta**: dice que el receptor podría rechazarlo, que es lo
  que la persona necesita para decidir.
- **«Firmar de todos modos» es la acción principal** y «Cancelar» va en
  fantasma.
- **Monocromo**: ni ámbar ni rojo. La caducada y la no válida se distinguen por
  la silueta, la palabra y el peso, porque la paleta no tiene token para color
  de estado.
