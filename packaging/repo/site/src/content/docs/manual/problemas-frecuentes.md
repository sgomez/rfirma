---
title: Problemas frecuentes
description: Soluciones cuando la sede electrónica no abre rFirma ni AutoFirma, la petición no llega, no aparece tu certificado o la firma no se completa.
---

Síntomas habituales al firmar con rFirma y cómo resolverlos. Si el tuyo no está
aquí, cuéntalo en [el repositorio](https://github.com/sgomez/rfirma/issues) o
desde **Comentarios y ayuda**, en el menú de rFirma.

## La sede no abre rFirma

**Al pulsar «Firmar» en la sede no pasa nada, o la sede dice que no encuentra
AutoFirma.**

- Comprueba en **Estado de rFirma**, desde el menú, que «Firma en sedes» está
  correcto. Si no lo está, pulsa **Usar rFirma**.
- Si el navegador preguntó si abrir la aplicación externa y lo rechazaste, vuelve
  a pulsar «Firmar» en la sede y acepta.
- Recarga la página de la sede y vuelve a empezar el trámite.

**Se abre AutoFirma en lugar de rFirma.** Las sedes abren la aplicación que el
sistema tiene elegida para los enlaces `afirma://`. En **Estado de rFirma**,
«Firma en sedes» ofrece **Usar rFirma** para que las abra rFirma.

## La petición no ha llegado

La ventana de sede se abre, se queda en «Conectando con la sede» y termina en
**«La petición no ha llegado»**. La página de la sede no ha podido hablar con
rFirma, casi siempre por una de estas dos causas, que la propia ventana explica:

- **Falta el permiso de red local del navegador.**
  - En Chrome, pulsa **Permitir** en la franja bajo la barra de direcciones. Si
    ya no está: candado › Configuración de sitios › Acceso a la red local.
  - En Firefox, pulsa **Permitir** en el panel junto a la barra de direcciones.
    Si ya no está: candado › Conexión segura › Más información › Permisos ›
    Acceso a la red local.
- **Falta la CA local de rFirma** en el navegador. En **Estado de rFirma**,
  «Certificado de rFirma» dice en qué navegadores está instalada y ofrece
  **Instalar**. Firefox necesita reiniciarse después.

Tras arreglarlo, vuelve a la sede y pulsa su botón de reintentar.

## Mi certificado no aparece

- **Los certificados caducados no se ofrecen en las sedes.** Si el tuyo ha
  caducado, renuévalo con su emisora.
- **La sede puede limitar los certificados válidos**: la ventana lo avisa, y si
  no queda ninguno dice que la sede no acepta tus certificados. Instala otro que
  la sede admita.
- **Si es una tarjeta o el DNIe**, comprueba que está en el lector antes de
  pulsar «Firmar» en la sede. Si la ventana ya estaba abierta, pulsa **Volver a
  buscar** si te lo ofrece; si no, cancela y vuelve a empezar desde la sede.
- **Si no tienes ninguno**, la ventana dice «No tienes ningún certificado» y
  ofrece **Instalar un certificado…** con un fichero `.p12` o `.pfx`.

## El PIN no funciona

- **«El PIN no es correcto»**: la tarjeta se bloquea tras varios intentos
  fallidos, así que comprueba el PIN antes de volver a probar.
- **«La tarjeta está bloqueada»**: se desbloquea con el PUK.

## La firma no se completa

**La ventana termina en «No se ha completado la petición».** Debajo dice qué
hacer:

- **«Vuelve a la sede e inténtalo de nuevo»**: un fallo pasajero.
- **«Cierra el otro trámite o la otra aplicación de firma»**: hay otro trámite
  abierto, o hay otra aplicación de firma, como AutoFirma, ocupando la conexión
  con la sede. Ciérralos y vuelve a intentarlo.
- **«No se puede firmar con ese certificado»**: vuelve a la sede y elige otro.
- **«Contacta con la sede para terminar el trámite»**: el fallo está en la sede.
  Copia el detalle técnico de la ventana y dáselo a quien mantiene la sede.

Si la ventana dice que la sede ha pedido un tipo de firma que rFirma no hace, no
es un fallo tuyo ni de tu certificado: el detalle trae una nota para quien
mantiene la sede. Más en [Si vienes de AutoFirma](/manual/si-vienes-de-autofirma/).

Si la ventana pregunta **«¿Te parece un fallo de rFirma?»**, cuéntalo en
el repositorio de rFirma, no al equipo de AutoFirma.

## Avisos antes de firmar

- **«Esta página está desactualizada»**: la sede usa una versión antigua de la
  pieza que habla con rFirma. Pulsa Continuar; si algo falla, avisa a quien
  mantiene la sede.
- **«El PDF que quieres firmar está certificado»**: el autor lo cerró a nuevas
  firmas, y la tuya invalidaría la anterior. Si no estás seguro, cancela y pide al
  autor una copia sin certificar.
- **El botón de firmar tarda tres segundos en activarse**: evita que un Intro
  pulsado por descuido firme. Se apaga en Preferencias, con «Esperar 3 segundos
  antes de firmar».

## Después de firmar

**No encuentro el documento que firmé en la sede.** rFirma no guarda copia de lo
que firma para una sede ni lo añade a los recientes: el documento firmado está en
la sede. Para guardar una copia, descárgala desde la propia sede.
