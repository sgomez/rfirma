---
title: Preferencias
description: "Qué cambia cada opción de las Preferencias de rFirma: privacidad, firma, certificados, tema e idioma."
---

Las **Preferencias** se abren desde el menú de la cabecera y ocupan la ventana bajo
ella. Tienen cuatro secciones: **General**, **Firma**, **Certificados** y
**Apariencia**. Se cierran con **Cerrar** o con `Escape`.

**Los cambios se aplican al hacerlos**: no hay botón de guardar. Si un ajuste no se
puede guardar, el aviso sale en la sección donde lo cambiaste y el control vuelve al
valor anterior.

## General

El grupo **Privacidad**:

- **Recordar mi actividad**: guarda los documentos recientes y el último certificado
  usado. Apágalo si compartes el ordenador. Al apagarlo, rFirma pide confirmación y
  **borra** lo guardado.
- **Vaciar la lista**, junto al interruptor, borra los recientes sin apagar el recuerdo.
- **Avisar de versiones nuevas**: la comprobación de versión es la única conexión
  saliente de rFirma. Apagada, no se hace.

## Firma

- **Recordar la firma visible**: encendida, rFirma guarda cómo configuraste la firma
  visible. Apagada, cada documento arranca con los valores por omisión.
- **Dónde guardar**: el destino del documento firmado.
  - **Junto al original**: en la misma carpeta que el PDF.
  - **En esta carpeta**: la que elijas con **Cambiar carpeta…**.

  Bajo flatpak solo está la carpeta elegida, y un documento que llega por el portal del
  sistema se guarda también ahí.
- **Esperar 3 segundos antes de firmar**: la pausa de la ventana de la sede electrónica
  antes de poder consentir la firma. Solo afecta a las firmas pedidas desde una sede.
- **Si solo sirve un certificado y la sede lo permite, usarlo sin preguntar**: apagada
  por omisión. Encendida, una sede que lo pida firma con el único certificado válido sin
  enseñar la lista.

## Certificados

La lista de los ficheros `.p12` que has instalado en rFirma. Tiene dos acciones:

- **Añadir…** instala un certificado. rFirma guarda lo necesario y no recuerda de dónde
  vino. Se rechaza al instalar una clave que no sea RSA ni de curva elíptica.
- **Quitar** en cada fila lo elimina.

Cada fila muestra el titular, el NIF, el almacén y la caducidad. Un certificado
caducado se queda en la lista, con el motivo por el que no puede firmar. Los
certificados de otros almacenes (el sistema, los navegadores, los tokens) no aparecen
aquí: rFirma los encuentra sin que los gestiones.

## Apariencia

- **Tema**: **El del sistema**, **Claro** u **Oscuro**. El del sistema no fuerza nada y
  sigue lo que tengas en el escritorio.
- **Idioma**: castellano, catalán, euskera, gallego e inglés. Solo se ofrecen los que
  están completamente traducidos.
