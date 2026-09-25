# Ventana principal

La única ventana de rFirma. Aloja el recorrido completo de firmar un PDF, de
abrir el documento a guardarlo firmado, sin navegar a otra pantalla.

## Casos de uso que la usan

- Firmar un PDF en local — de principio a fin.

## Estructura

Cabecera, tira de pestañas y, debajo, el visor y el panel de firma.

```
┌─────────────────────────────────────────────────────────┐
│ cabecera                                                │  52 px
├─────────────────────────────────────────────────────────┤
│ tira de pestañas                                        │  40 px
├─────────────────────────────────────────────────────────┤
│ franja de notificación (solo si hay algo que notificar)  │  41 px
├──────────────────────────────────────┬──────────────────┤
│ visor                                │ panel de firma   │
│ flexible                             │ 380 px           │
└──────────────────────────────────────┴──────────────────┘
```

**Sin documento no hay panel.** El visor ocupa todo el ancho con la zona de
soltar y los recientes, y la tira solo lleva el «+». El panel no se monta: no
hay nada que firmar.

- [Cabecera](cabecera.md) — identidad y menú principal.
- [Pestañas de documentos](pestanas-de-documentos.md) — qué documentos hay
  abiertos, cuál se firma y los recientes.
- [Visor de documento](visor-de-documento.md) — cómo va a quedar.
- [Panel de firma](panel-de-firma.md) — la firma visible, el destino y el botón
  que firma.

La acción principal vive **al pie del panel**: como mucho un botón primario en
pantalla.

Sobre la ventana se abren diálogos con velo, que la oscurecen sin desmontarla:
[el del secreto del almacén](dialogo-pin.md),
[progreso de firma](dialogo-progreso-firma.md),
[páginas sin firma visible](dialogo-paginas-sin-firma-visible.md) y
[acerca de](acerca-de.md). [Preferencias](preferencias.md) y el
[panel de estado](panel-de-estado.md) son vistas que tapan todo lo que hay bajo
la cabecera, tira incluida.

El velo lo coloca `rfirma-app/src/app.css` (`position: fixed`, `inset: 0`,
diálogo centrado): `.rf-scrim` en el bundle es solo el color. Sin esa regla los
diálogos se pintan en flujo, detrás de la ventana.

### Capas

| z-index | Qué |
| --- | --- |
| 5 | cabecera (11 con su menú abierto, para quedar sobre la tira) |
| 6 | desplegable de certificados, bloque de modelos y menú «+ Dato» del panel |
| 8 | menú del «+» |
| 10 | tira de pestañas |
| 20 | velo de cualquier diálogo |

### Geometría

- Cabecera de 52 px sin borde propio: la raya la pone la tira.
- Tira de 40 px sobre `--rf-surface`, borde inferior de 1 px en
  `--rf-border-subtle`.
- Visor flexible sobre `--rf-bg`. Panel de 380 px fijos, borde izquierdo de 1 px
  en `--rf-border-subtle`.
- Solo el papel del documento fuerza `data-theme="light"`.
- **La ventana abre a 1280×720 y no baja de 1100×560.** El mínimo de ancho
  protege al visor, que es la región principal; el de alto deja caber la ventana
  en un portátil de 1366×768. No hay lógica de monitores: el gestor de ventanas
  coloca. El tamaño se recuerda entre sesiones
  ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).

## La franja de notificación

**Es el sitio donde notifica rFirma**, y va **bajo la tira de pestañas**, a
ancho completo, encima del visor y del panel: fondo `--rf-surface`, borde
inferior de 1 px en `--rf-border-subtle`, 41 px de alto. Cuando no hay nada que
notificar no se monta y el contenido sube.

Lleva icono, una frase, **una sola acción** secundaria y una `×` para
descartarla. Su único inquilino es el aviso de versión nueva: «Hay una versión
nueva de rFirma: **0.4.1**», con «Cómo actualizar», que lleva a
[Acerca de](acerca-de.md).

No es un sitio para errores del recorrido: el error de firma va en el panel, y
los fallos de Preferencias dentro de su sección. Los diagnósticos tampoco
avisan aquí: los dice la [ventana de sede](ventana-de-sede.md) cuando duelen, y
el triángulo del menú de la [cabecera](cabecera.md) llama a mirar el panel de
estado.

## La secuencia no es negociable

```
configurar la firma visible → prefirma → firma → postfirma → guardar
```

La firma visible forma parte del PDF cuyo hash se firma, así que tiene que estar
decidida antes de la prefirma
([ADR-0001](../adr/0001-firma-trifasica-clave-privada-solo-en-rust.md)).

**El secreto del almacén no es un eslabón de esa cadena**: es requisito del
almacén. Sin necesidad de sesión no hay diálogo; un módulo PKCS#11 o un perfil
de Firefox con contraseña maestra lo piden **al buscar certificados**, antes de
que haya lista; un `.p12` instalado lo pide **al firmar**. Ver
[dialogo-pin.md](dialogo-pin.md).

## Estados

Todos viven en el artboard `Main`, palanca «Estado». El recorrido nunca cambia
de pantalla.

| Estado | Pestañas | Visor | Panel |
| --- | --- | --- | --- |
| Vacío | solo el «+» | zona de soltar y recientes | no se monta |
| Buscando certificados | el documento | documento | editable; el botón dice «Buscando certificados…» con indicador, al 55 %. Encima, el diálogo de secreto si el almacén lo pide para listar |
| Sin certificados | ídem | documento | «Sin certificados» arriba; el pie ofrece «Añadir un certificado…» y «Volver a buscar» |
| Sin certificado elegido | ídem | documento; la firma visible, si está encendida, con el recuadro vacío | «Elegir certificado ▾», un solo botón que abre la lista |
| Listo | ídem | documento, con la firma visible si está encendida | «Firmar como <nombre> ▾» |
| Certificados abiertos | ídem | ídem | la lista flota sobre el pie, hacia arriba |
| Pidiendo el secreto / secreto incorrecto | ídem | bajo el velo | bajo el velo |
| Firmando | ídem | bajo el velo, hoja al 45 % | bajo el velo; el diálogo de progreso encima |
| Firmado | la pestaña pasa a `…-firmado.pdf` con ✓ | documento firmado | «Firmado a las 11:04» y el resumen; el pie ofrece abrir el PDF, la carpeta o volver a firmar |
| Error al firmar | sin ✓ | documento sin tocar | el error sustituye al panel; el pie ofrece «Reintentar» |

**El pie del panel mide lo mismo en todos**: 162 px. Lo que cambia es su fila de
botones, de 44 px.

## Componentes y tokens

`.rf-scrim` + `.rf-dialog` para los diálogos; `--rf-surface` en cabecera, tira y
franja; `--rf-bg` en visor y panel; `--rf-border-subtle` entre regiones. Tema
claro y oscuro, según el sistema operativo; el papel siempre claro.

## Decisiones

**Pestañas en lugar de bandeja lateral.** Hasta Main v4 la ventana eran tres
columnas —bandeja de 300 px, visor y panel de 360 px— y las dos laterales fijas
se comían el visor. Se compararon cuatro composiciones y ganó una quinta que las
combina:

| | Composición | Por qué no |
| - | --- | --- |
| v2 | Pestañas sobre la ventana de entonces | resolvía la bandeja, pero el panel seguía con cabecera de documento, fila de certificado y botón sin nombre |
| V3 A | Documento primero, miniaturas a la izquierda, acción en barra flotante | el titular y la carpeta se recortaban mucho en la barra; las miniaturas no escalan a 200 páginas |
| V3 B | Panel tipo recibo con «Firmar como», «Firma visible» y «Guardar en» | el recibo repetía el certificado que ya puede decir el botón |
| V3 C | Flujo en cuatro pasos | cuatro pasos para firmar una vez; la barra recortaba titular y documento |
| **V4 D** | **Pestañas de v2, visor y paginación de antes, panel de B sin el renglón del certificado, botón partido de A al pie** | **elegida** |

**La franja baja de la cabecera a debajo de la tira** (25/09/2026). Las
pestañas son parte de la cabecera de la ventana; una notificación entre las dos
las separaría.

**Firmando es un diálogo con velo**, no un estado del pie: el secreto, el
progreso y el resultado se suceden en el mismo sitio. **El error de firma es un
estado del panel**, como firmado: lo que pasó y que el documento sigue intacto
arriba, «Reintentar» en el pie, sin que el pie crezca.

**Se borró «Documento cargado, sin certificado»**: el certificado se elige en el
botón de firmar, y la firma visible ya no depende de él.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`, el 25/09/2026.
