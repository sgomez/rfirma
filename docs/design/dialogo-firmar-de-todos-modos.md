# Diálogo «¿Firmar de todos modos?»

Pide confirmación, **justo antes de firmar**, cuando el documento ya trae algún
problema: una firma caducada, una no válida o un hallazgo del documento. Es un
paso de confirmación, no un bloqueo: las dos salidas están siempre.

## Casos de uso que la usan

- Firmar un PDF en local — al pulsar «Firmar» en el
  [panel de firma](panel-de-firma.md), **solo** si hay algún ⚠ o ✗: una firma
  caducada, una no válida —también la que rFirma no reconoce— o un hallazgo del
  documento.

No lo abre la [ventana de sede](ventana-de-sede.md), que ya es un
consentimiento: informa y no pide confirmación.

## Estructura

`.rf-dialog` de 420 px sobre `--rf-scrim`. El velo tapa la ventana principal
entera, cabecera con sus pestañas incluida, y por eso lleva `z-index:20`.
Debajo, la ventana en «listo» con el aviso compacto de firmas previas y el
botón del pie normal. De arriba abajo:

1. **Título** en `.rf-title`: «¿Firmar de todos modos?».
2. **Una fila por problema y nada más**, en un bloque con borde de 1 px en
   `--rf-border-subtle` y `--rf-radius-md`, filas separadas por 1 px en
   `--rf-border-subtle`:
   - **Primero los hallazgos del documento**: el círculo con aspa y su texto,
     sin segunda línea («Se ha modificado después de la última firma»).
   - **Después cada firma caducada o no válida**: el icono de su validez y
     «Firma N · firmante» a 13 px y peso 700; debajo, su motivo en
     `--rf-text-muted`, sangrado 23 px (icono de 15 más 8 de separación) para
     que caiga bajo el texto.
   - **Las válidas no salen.** Tampoco hay párrafo con el recuento: la lista ya
     lo cuenta.
3. **«El receptor podría rechazarlo.»**, en `.rf-prose`.
4. Un `.rf-divider` y **dos salidas** alineadas a la derecha: `Cancelar` en
   `--ghost` y `Firmar igualmente` en `--primary`, como en el
   [diálogo del PIN](dialogo-pin.md).

## Estados

En el artboard `EstadoFirmarDeTodosModos`, palanca «Firmas previas», solo con
los casos que tienen algún problema. Por defecto, «valida, caducada, no admitia
mas firmas y hallazgo (3)».

- **Solo caducadas** («una caducada (3)»): una fila con el triángulo.
- **No válidas y hallazgos**: los hallazgos arriba y una fila por firma.
- **Formato desconocido** («formato desconocido (2)»): la firma que rFirma no
  reconoce es una fila más, «Firma 2 · Notaría XYZ» con «rFirma no conoce este
  tipo de firma».
- **Muchas** («extremo · todos los motivos y hallazgos (7)»): los seis motivos
  y los tres hallazgos. La lista tiene un **alto máximo de 264 px** y se
  desplaza por dentro; el diálogo no crece más. La última fila visible queda
  cortada, que es lo que dice que hay más, sin sombras ni rótulos añadidos.

`Cancelar` vuelve al panel sin firmar. `Firmar igualmente` sigue el recorrido
de siempre.

## Componentes y tokens

`.rf-scrim`, `.rf-dialog`, `.rf-title`, `.rf-prose`, `.rf-body`,
`.rf-divider`, `.rf-btn--primary|--ghost`, `--rf-border-subtle`,
`--rf-text`, `--rf-text-muted`, `--rf-radius-md`. Los iconos de validez son los
de [design-system.md](design-system.md#8-accesibilidad).

## Decisiones

- **Lo abre cualquier problema**, no solo una firma no válida: la caducada y el
  hallazgo del documento también. Antes, «ha cambiado después de la última
  firma» solo subía el tono del aviso; ahora es un hallazgo que pesa como una
  firma no válida (ADR-0043).
- **Absorbe el diálogo de las firmas que rFirma no reconoce.** Una firma
  ilegible o de formato desconocido es *no válida*, con su motivo, y no tiene
  pregunta propia: la cofirma pide el consentimiento aquí, en una fila más.
- **Una fila por problema y sin párrafo de recuento.** «Este documento lleva N
  firmas que no son válidas» repetía lo que la lista ya cuenta, y no servía
  para hallazgos ni caducadas.
- **Las válidas no se repiten**: están en «Ver firmas».
- **«El receptor podría rechazarlo.»** sustituye a «Si lo firmas, la sede u
  organismo que reciba el documento puede rechazarlo»: dice lo mismo en
  menos.
- **«Firmar igualmente» es la acción principal** y `Cancelar` va en fantasma.
- **El botón del pie del panel no cambia**: sigue siendo «Firmar», y es este
  diálogo el que pregunta.
- **Monocromo**: ni ámbar ni rojo; la caducada y la no válida se distinguen por
  la silueta y la palabra.

Validado el **26/09/2026** en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `EstadoFirmarDeTodosModos`; la fila por
problema, el cierre y la absorción de las firmas no reconocidas, el
**03/10/2026** (anotaciones `nota-firmar-de-todos-modos` y `nota-validez`).
