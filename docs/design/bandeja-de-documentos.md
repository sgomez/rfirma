# Bandeja de documentos

Columna izquierda de 300 px. Responde a una sola pregunta: **qué documento se
firma**. Es el único sitio de la aplicación donde se abre o se cambia de
documento.

## Casos de uso que la usan

- Firmar un PDF en local — en todos los estados salvo el diálogo modal.

## Estructura

De arriba abajo:

1. **Zona de soltar**: recuadro con borde discontinuo,
   «Arrastra un PDF o pulsa para abrirlo». Pulsarla abre el explorador de
   archivos del sistema.
2. **Recientes**: lista de documentos ya vistos, **diez como máximo**, con
   desalojo por último uso —reabrir uno viejo lo rescata—. Cada fila lleva el
   nombre, una insignia de estado y la fecha.

La fila seleccionada se marca con `--rf-border-strong` y fondo `--rf-bg`;
las demás son `.rf-card--interactive` sin borde.

### Geometría

- La columna entera lleva 16 px (`--rf-space-sm`) de relleno y 16 px entre la
  zona de soltar, el rótulo y la lista.
- **Zona de soltar**: columna centrada con 16 px de relleno y 8 px
  (`--rf-space-xs`) entre el icono y el texto; borde de **1 px discontinuo** en
  `--rf-border-strong` y `--rf-radius-md`. No fija alto: lo dan el icono, el
  texto y el relleno. El texto va en `.rf-prose` centrado.
- **Icono de la zona de soltar**: la flecha de subir, `<svg>` en línea de 28×28
  px sobre lienzo `0 0 24 24`, trazo de 1.5 en `currentColor`, teñida con
  `--rf-text-muted`. Dos trazados: `M12 16V4M8 8l4-4 4 4` y
  `M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3`.
- **«RECIENTES»**: `.rf-label` en **versalitas** (`text-transform: uppercase`)
  con `letter-spacing: .6px`. Es el mismo tratamiento que los rótulos de
  sección del [panel](panel-de-firma.md).
- **Fila**: 16 px de relleno, `--rf-radius-md`, borde de 1 px, y **6 px** entre
  el nombre y su línea de metadatos. El nombre va en `.rf-prose` a peso **600**
  con elipsis; debajo, la insignia y la fecha en `.rf-body rf-text-muted`
  separadas 8 px.
- La fila sin seleccionar tiene borde y fondo **transparentes**, no ausentes:
  así no salta de tamaño al seleccionarse.

## Vocabulario de las insignias

Tres valores: **`Firmado`**, **`Sin firmar`** y **`No disponible`**.

Un PDF que ya trae la firma de otra persona es, sencillamente, `Firmado`.
Quién la puso y cuándo se consulta desplegando el aviso de cofirma del
[panel de firma](panel-de-firma.md), que es donde ese detalle tiene sitio.

Consecuencia asumida: un documento que llegó firmado por otro sale como
`Firmado` antes y después de firmarlo; en la fila solo cambia la hora. Si
alguna vez hace falta distinguir «firmado por mí» de «firmado por otro»,
habrá que decidirlo como vocabulario, no improvisarlo en la fila.

`No disponible` es distinto de los otros dos: no describe el documento sino que
**la ruta no responde**. La fila se atenúa y al pulsarla se ofrece quitarla de
la lista, pero no se purga sola: un PDF en un USB desmontado o en un disco de
red caído no está borrado, y la fila revive cuando la ruta reaparece.

Al firmar aparecen **dos filas**, el original y el firmado, y el firmado pasa a
ser el documento activo. No se fusionan: hay dos ficheros en el disco y la
bandeja lo dice.

## Estados

- **Vacía** (primera ejecución, o con «Recordar mi actividad» apagado en
  [Preferencias](preferencias.md)): solo la zona de soltar, más «Aquí
  aparecerán los documentos que vayas firmando». **Aquí la ficha y el canvas
  no coinciden**, y manda la ficha: el artboard del estado vacío enseña además
  el rótulo `RECIENTES` sobre el mensaje, y un encabezado sobre una lista que
  no existe promete una sección vacía donde no hay ninguna. El rótulo aparece
  con el primer documento (ID-44).
- **Con recientes**: la lista, con el documento activo seleccionado.
- **Arrastrando** (por definir): la zona de soltar debe acusar el arrastre.

## Componentes y tokens

`.rf-card--interactive`, `.rf-badge`, `.rf-label`, `.rf-prose`, `.rf-body`,
`--rf-surface` de fondo, `--rf-border-strong` para la zona de soltar y la fila
seleccionada.

## Lo que esta pantalla deja abierto

Nada. Los recientes son **estado persistido entre sesiones**, y qué se guarda,
dónde, cuánto dura y cómo se borra quedó decidido en el
[ADR-0010](../adr/0010-memoria-entre-sesiones.md): se cachean nombre, insignia,
`mtime` y fecha de último uso para poder pintar la fila sin abrir el fichero, y
se revalida solo el documento que se selecciona.

## Decisiones

La bandeja fue la variante ganadora (D) precisamente por esto: reconocer el
documento y reutilizar la configuración anterior es lo que la aplicación
original obliga a repetir cada vez.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards «1 · Vacío» y «5 · Configurando la firma
visible».
