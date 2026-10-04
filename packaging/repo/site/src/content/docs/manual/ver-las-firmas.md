---
title: Ver las firmas
description: "Qué muestra el diálogo «Ver firmas» de rFirma y qué significa cada validez (válida, caducada, no válida) y cada hallazgo del documento."
---

El diálogo **Ver firmas** enseña las firmas que ya trae un PDF, con su validez, antes
de que firmes. Solo se mira y se cierra: no firma ni pregunta nada.

## Cómo se abre

- Al abrir un PDF que ya está firmado, el panel de firma muestra una línea con el
  número de firmas y, si los hay, los problemas. Pulsa **Ver firmas →**.
- En una sede electrónica, el consentimiento de firma ofrece el mismo botón.

Se cierra con **Cerrar** o con `Escape`, y vuelves a donde estabas.

## Qué enseña

![PDF firmado en PAdES, con el resumen de la firma](../../../assets/how-step4.png)

Arriba, el título con el formato y el recuento (por ejemplo, «PAdES · 3 firmas»). Debajo,
una zona que se desplaza:

1. **Los hallazgos del documento**, si los hay, uno por línea. No son de ninguna firma
   concreta.
2. **Una ficha por firma**, con el número y su validez, y estas filas:
   - **Firmante**.
   - **En nombre de**, solo si el certificado es de representación.
   - **Emisor** del certificado.
   - **Fecha**, la que declara la firma, o **Sellada** si lleva un sello de tiempo.
   - **Motivo**, solo cuando la firma no es válida o está caducada.

   La firma que cierra el documento lo dice con **No admite más firmas**, y las
   contrafirmas se anidan dentro de su firma con su propia validez.

## Qué significa cada validez

Una firma tiene siempre una de estas tres valideces. Se distinguen por el icono, la palabra y el
peso del texto, no por el color.

| Validez | Qué significa |
| --- | --- |
| **Válida** | La firma está íntegra, el documento no se ha tocado tras ella y el certificado era utilizable. |
| **Caducada** | El certificado ha caducado. El motivo dice cuándo. |
| **No válida** | La firma tiene un problema, que el motivo explica. |

Los motivos de una firma caducada o no válida son:

- El certificado caducó el día indicado.
- El certificado no se podía usar antes del día indicado.
- Se ha modificado después de firmarse.
- La firma está dañada.
- rFirma no conoce este tipo de firma.
- Usa un algoritmo de resumen no admitido (MD5 o MD2).
- La firma anterior no admitía más firmas.

Una firma que rFirma no reconoce sale como una ficha más, no válida.

## Los hallazgos del documento

Un hallazgo es algo que se encuentra en el documento sin poder atribuirlo a una firma.
Pesa como una firma no válida:

- Se ha modificado después de la última firma.
- Se ha rellenado el formulario después de firmar.
- Se ha añadido contenido encima de lo firmado.

## El resumen de `verify -gui` y tras firmar

Desde la terminal, `rfirma verify -i documento.pdf -gui` abre la ventana con el
documento y el resumen de sus firmas (véase [Línea de órdenes](/manual/linea-de-ordenes/)).
El panel enseña también ese resumen justo después de firmar. Usa las mismas fichas,
valideces y hallazgos que el diálogo; tras firmar, un documento modificado antes de tu
firma se muestra como **Se modificó antes de tu firma**.

Si no hay firmas que mostrar, el resumen lo dice con **Sin firmas**. Si el fichero no
es un PDF ni una firma CAdES o XAdES, dice **Formato no reconocido**, y si no se han
podido leer, **No se han podido leer las firmas**.

## Firmar con problemas

Ver las firmas no impide firmar. Si hay alguna firma caducada, no válida o algún hallazgo,
al pulsar **Firmar** rFirma pregunta **¿Firmar de todos modos?** y enumera los problemas.
Es una confirmación, no un bloqueo.
