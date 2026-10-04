---
title: Si vienes de AutoFirma
description: Alternativa a AutoFirma sin Java. Qué funciona igual en rFirma (las sedes electrónicas y la línea de órdenes) y qué cambia al pasar de AutoFirma a rFirma.
---

rFirma es una alternativa a AutoFirma que no necesita Java. Los formatos de firma
salen del código de AutoFirma, compilado dentro de rFirma, así que las firmas son
las mismas; cambian la ventana, la forma de elegir certificado y algunas órdenes.

rFirma es un proyecto independiente: no está relacionada con AutoFirma ni con la
Administración General del Estado. Los fallos de rFirma se cuentan en
[su repositorio](https://github.com/sgomez/rfirma/issues), no al equipo de
AutoFirma.

## Qué funciona igual

### Las sedes electrónicas

rFirma atiende el mismo protocolo que AutoFirma: las sedes que piden AutoFirma la
llaman con los mismos enlaces `afirma://`, sin cambiar nada en la sede. Atiende
estas peticiones:

- **Firmar, cofirmar y contrafirmar** un documento que envía la sede, o uno que
  eliges tú.
- **Ceder los datos de identidad** del certificado.
- **Guardar** un fichero de la sede en tu equipo y **cargar** ficheros tuyos para
  enviarlos.
- **Firmar y guardar** el resultado.
- **Firmar un lote**, remoto o local.

Firma en CAdES, PAdES, XAdES y FacturaE, como AutoFirma. Cómo es el trámite, en
[Firmar en una sede electrónica](/manual/firmar-en-una-sede/).

### La línea de órdenes

`rfirma` atiende `sign`, `cosign`, `listaliases` y `verify`, con los mismos
parámetros y escritos también con un solo guion, como en AutoFirma (`-i`,
`-alias`, `-store`). La referencia completa está en
[Línea de órdenes](/manual/linea-de-ordenes/).

## Qué cambia

### En las sedes

- **Siempre ves quién pide la firma y qué se firma.** Antes de firmar, la ventana
  de sede dice la dirección de la página, qué tipo de documento es y si ya trae
  firmas. Una sede no puede firmar sin que lo consientas, aunque pida elegir el
  certificado sola; eso solo pasa si lo permites en Preferencias.
- **Al terminar, la ventana dice cómo ha ido**: firmado y enviado, cancelado o
  rechazado, con un detalle que se puede copiar para la sede.
- **Los certificados caducados no se ofrecen.**
- **rFirma no guarda copia** de lo que firmas para una sede.
- **Algunas peticiones se rechazan**, con una explicación en la ventana:
  - firmas con SHA-1 y XAdES explícita;
  - XMLDSig;
  - cofirma y contrafirma de una factura electrónica;
  - contrafirma en un formato que no sea CAdES, CMS o XAdES;
  - el lote local en XML (el lote local en JSON sí se atiende);
  - un almacén de certificados concreto que rFirma no abre.

### En la línea de órdenes

- **`countersign` y `batchsign` no existen**: la contrafirma y los lotes se
  atienden desde las sedes, no desde la terminal.
- **La contraseña no se escribe en la orden**: `--password` se rechaza siempre.
  El PIN se escribe en la terminal sin eco, se lee de un descriptor con
  `--password-fd` o se pide en la ventana de `--certgui`.
- **`--certtui`** elige el certificado en la terminal. Es propio de rFirma.
- **Algunos almacenes de AutoFirma no se abren** (`pkcs12`, `dni`, `dnie`,
  `mac`): la orden se rechaza con un mensaje que lo dice.

Cada rechazo y su alternativa están en
[Línea de órdenes](/manual/linea-de-ordenes/#lo-que-rfirma-rechaza).

## Tener las dos instaladas

AutoFirma y rFirma pueden estar instaladas a la vez, pero solo una atiende las
sedes. Para que sea rFirma, ve a **Estado de rFirma**, desde el menú, y en
«Firma en sedes» pulsa **Usar rFirma**. Si las dos están abiertas a la vez, una
sede puede no llegar a rFirma: cierra AutoFirma y vuelve a intentarlo.
