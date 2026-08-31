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
2. **Recientes**: lista de documentos ya vistos. Cada fila lleva el nombre,
   una insignia de estado y la fecha.

La fila seleccionada se marca con `--rf-border-strong` y fondo `--rf-bg`;
las demás son `.rf-card--interactive` sin borde.

## Vocabulario de las insignias

Dos valores, y solo dos: **`Firmado`** y **`Sin firmar`**.

Un PDF que ya trae la firma de otra persona es, sencillamente, `Firmado`.
Quién la puso y cuándo se consulta desplegando el aviso de cofirma del
[panel de firma](panel-de-firma.md), que es donde ese detalle tiene sitio.

Consecuencia asumida: un documento que llegó firmado por otro sale como
`Firmado` antes y después de firmarlo; en la fila solo cambia la hora. Si
alguna vez hace falta distinguir «firmado por mí» de «firmado por otro»,
habrá que decidirlo como vocabulario, no improvisarlo en la fila.

## Estados

- **Vacía** (primera ejecución): solo la zona de soltar, más
  «Aquí aparecerán los documentos que vayas firmando».
- **Con recientes**: la lista, con el documento activo seleccionado.
- **Arrastrando** (por definir): la zona de soltar debe acusar el arrastre.

## Componentes y tokens

`.rf-card--interactive`, `.rf-badge`, `.rf-label`, `.rf-prose`, `.rf-body`,
`--rf-surface` de fondo, `--rf-border-strong` para la zona de soltar y la fila
seleccionada.

## Lo que esta pantalla deja abierto

Los recientes son **estado persistido entre sesiones**, y eso no estaba en el
inventario de capacidades del prototipo: qué se guarda, dónde, cuánto tiempo y
cómo se borra son decisiones de alcance, no de interfaz. Están pendientes en el
mapa.

## Decisiones

La bandeja fue la variante ganadora (D) precisamente por esto: reconocer el
documento y reutilizar la configuración anterior es lo que la aplicación
original obliga a repetir cada vez.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards «1 · Vacío» y «5 · Configurando la firma
visible».
