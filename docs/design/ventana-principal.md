# Ventana principal

**La verdad del dibujo es el código y sus historias:**
`shell/MainWindow.stories.tsx` («Flujos/Ventana principal/MainWindow»), con las piezas
en las historias que su número ordena en la misma carpeta de la barra lateral
(cabecera, pestañas, recientes, visor, franja de versión y aviso de error). Esta
ficha cuenta el flujo y el porqué; los textos salen del catálogo y no se copian.

La única ventana de rFirma. Aloja el recorrido completo de firmar un PDF, de
abrir el documento a guardarlo firmado, sin navegar a otra pantalla.

## Casos de uso que la usan

- Firmar un PDF en local — de principio a fin.
- Ver las firmas de un documento desde la terminal — `rfirma verify -i <doc>
  -gui` abre la ventana con el documento en el resumen.

## Estructura

Una sola barra —la cabecera con las pestañas— y, debajo, el visor y el panel de firma.
Así en Windows y macOS; en Linux, la barra de título nativa de GTK y, debajo, una
tira con solo las pestañas (ver la [cabecera](cabecera.md#en-linux)).

De arriba abajo: la barra (la cabecera con las pestañas), la franja de
notificación **solo si hay algo que notificar**, y debajo, en dos columnas, el
visor —flexible— y el panel de firma, de ancho fijo.

**Sin documento no hay panel.** El visor ocupa todo el ancho con la zona de
soltar y los recientes, y la cabecera, el botón de abrir sin ninguna pestaña. El panel no se monta: no
hay nada que firmar.

- [Cabecera](cabecera.md) — la barra única, o la barra de título GTK en Linux: identidad, aviso y menú principal.
- [Pestañas de documentos](pestanas-de-documentos.md) — lo que la barra lleva
  de los documentos: abrir, los recientes, cuáles hay abiertos y cuál se firma.
- [Visor de documento](visor-de-documento.md) — cómo va a quedar.
- [Panel de firma](panel-de-firma.md) — el certificado, la firma visible, el
  destino y el botón que firma.

La acción principal vive **al pie del panel**: como mucho un botón primario en
pantalla.

Sobre la ventana se abren diálogos con velo, que la oscurecen sin desmontarla:
[el del secreto del almacén](dialogo-pin.md),
[progreso de firma](dialogo-progreso-firma.md),
[páginas sin firma visible](dialogo-paginas-sin-firma-visible.md),
[¿firmar de todos modos?](dialogo-firmar-de-todos-modos.md) y
[acerca de](acerca-de.md). [Preferencias](preferencias.md) y el
[panel de estado](panel-de-estado.md) son vistas que tapan todo lo que hay bajo
la cabecera, y la dejan en su variante sin documentos: solo `rFirma` y el menú.

El velo lo coloca `rfirma-app/src/app.css` (`position: fixed`, `inset: 0`,
diálogo centrado): `.rf-scrim` en el bundle es solo el color. Sin esa regla los
diálogos se pintan en flujo, detrás de la ventana.

### Capas

| z-index | Qué |
| --- | --- |
| 6 | bloque de modelos y menú «+ Dato» del panel |
| 7 | selector de certificado y su lista |
| 10 | cabecera (11 con cualquiera de sus menús abierto: el principal, los recientes o las pestañas ocultas) |
| 20 | velo de cualquier diálogo |

### Geometría

- Cabecera de 44 px sobre `--rf-bg`, borde inferior de 1 px en
  `--rf-border-subtle`.
- Visor flexible sobre `--rf-bg`. Panel de 380 px fijos, borde izquierdo de 1 px
  en `--rf-border-subtle`. Las historias enmarcan la ventana a su tamaño mínimo.
- Solo el papel del documento fuerza `data-theme="light"`.
- **La ventana abre a 1280×720 y no baja de 1100×560.** El mínimo de ancho
  protege al visor, que es la región principal; el de alto deja caber la ventana
  en un portátil de 1366×768. No hay lógica de monitores: el gestor de ventanas
  coloca. El tamaño se recuerda entre sesiones
  ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).

## La franja de notificación

**Es el sitio donde notifica rFirma**, y va **bajo la cabecera**, a
ancho completo, encima del visor y del panel: fondo `--rf-surface`, borde
inferior de 1 px en `--rf-border-subtle`, 41 px de alto. Cuando no hay nada que
notificar no se monta y el contenido sube.

Lleva icono, una frase, **una sola acción** secundaria y una `×` para
descartarla (`actions.dismiss`). Su único inquilino es el aviso de versión nueva
(`updates.newVersion`). Si la versión se puede instalar desde la aplicación, la
acción es la de instalar (`updates.install.action`); si no, es el paso a
[Acerca de](acerca-de.md) (`notifications.newVersion.action`), donde están las
órdenes de alta del repositorio. Una historia por caso: `Installable` y
`NotInstallable` de la franja, y `NewVersionInstallable` y
`NewVersionNotInstallable` sobre la ventana.

No es un sitio para errores del recorrido: el error de firma va en el panel, y
los fallos de Preferencias dentro de su sección. Los diagnósticos tampoco
avisan aquí: los dice la [ventana de sede](ventana-de-sede.md) cuando duelen, y
el botón de aviso de la [cabecera](cabecera.md) llama a mirar el panel de
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

El recorrido nunca cambia de pantalla: los estados son combinaciones del
contenido de las regiones. Los de la **ventana** y los del **visor** tienen
historia, y también los del **panel** y los **diálogos**, en sus propias
historias.

| Estado | Pestañas | Visor | Panel | Historia |
| --- | --- | --- | --- | --- |
| Vacío | ninguna; el botón de abrir sí | zona de soltar y recientes | no se monta | `Empty` |
| Vacío con aviso | ídem | ídem | no se monta | `WithAttention` |
| Versión nueva | ídem | ídem | no se monta | `NewVersionInstallable`, `NewVersionNotInstallable` |
| Linux | solo la tira bajo la barra GTK | ídem | ídem | `NativeTitlebar` |
| Con documento | el documento | documento, con o sin firma visible | editable | visor: `WithDocument`, `WithSignatureBox` |
| Buscando certificados | el documento | documento | el selector dice que busca, y firmar está inactivo. Encima, el diálogo de secreto si el almacén lo pide para listar | `Searching` de «Flujos/Firma/SigningPanel» |
| Sin certificados | ídem | documento | el aviso de que no hay certificados arriba; el pie ofrece añadir uno y volver a buscar | `NoCertificates` de «Flujos/Firma/SigningPanel» |
| Sin certificado elegido | ídem | documento, sin firma visible: su interruptor está desactivado hasta elegir | el selector pide elegir | `Unchosen` de «Flujos/Firma/SigningPanel» |
| Listo | ídem | documento, con la firma visible si está encendida | el selector con el certificado elegido, y firmar | `Ready` de «Flujos/Firma/SigningPanel» |
| Certificados abiertos | ídem | ídem | el buscador en el selector y la lista flotando sobre el panel | `Open` de «Dominio/Firma/CertificateSelect» |
| Pidiendo el secreto / secreto incorrecto | ídem | bajo el velo | bajo el velo | [ficha del secreto](dialogo-pin.md): ventana nativa, sin historia |
| Viendo las firmas previas | ídem | bajo el velo | bajo el velo; encima, [Ver firmas](dialogo-ver-firmas.md) | las historias de «Flujos/Firma/SignaturesDialog» |
| ¿Firmar de todos modos? | ídem | bajo el velo | bajo el velo; encima, [el diálogo](dialogo-firmar-de-todos-modos.md) | las historias de «Flujos/Firma/SignAnywayDialog» |
| Firmando | ídem | bajo el velo, hoja atenuada | bajo el velo; el diálogo de progreso encima | `Signing` de «Flujos/Firma/SigningPanel» y «Flujos/Firma/SigningProgressDialog» |
| Firmado | la pestaña pasa al documento firmado, con su marca | documento firmado | la franja de hora, los hallazgos y el resumen con todas las firmas y su validez, la tuya como nueva | `JustSigned` de «Flujos/Firma/SignaturesPanel» |
| `verify --gui` | el documento | el documento; si no es PDF, el icono y el nombre sin vista previa | el mismo resumen sin franja, o el aviso de que no hay firmas, de formato no reconocido o del fallo al leer las firmas | visor: `WithoutPreview` |
| Error al firmar | sin marca | documento sin tocar | el error sustituye al panel; el pie ofrece reintentar | aviso: `SigningFailed`, `SigningFailedWithoutRetry` |

**El pie del panel mide lo mismo en todos**: 162 px. Lo que cambia es su fila de
botones, de 44 px.

## Componentes y tokens

`.rf-scrim` + `.rf-dialog` para los diálogos; `--rf-bg` en cabecera, visor y panel;
`--rf-surface` en la franja; `--rf-border-subtle` entre regiones. Tema
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

**Una sola barra arriba** (27/09/2026): la cabecera de 52 px y la tira de
pestañas de 40 px se funden en una barra de 44 px, y los 48 px que sobran vuelven
al visor ([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)). La
franja de notificación va debajo de ella, como ya iba debajo de la tira desde el
25/09/2026: las pestañas son parte de la cabecera y una notificación no se mete
entre las dos.

**Firmando es un diálogo con velo**, no un estado del pie: el secreto, el
progreso y el resultado se suceden en el mismo sitio. **El error de firma es un
estado del panel**, como firmado: lo que pasó y que el documento sigue intacto
arriba, «Reintentar» en el pie, sin que el pie crezca.

**Se borró «Documento cargado, sin certificado»**: el certificado se elige en el
panel, y la firma visible ya no depende de él.

**El certificado se elige en un selector al principio del panel, separado del
botón de firmar** (27/09/2026), como en la ventana de sede. Sustituye al botón
partido de V4 D: quien pulsaba «Firmar como…» en lugar de la flecha firmaba con
un certificado que no quería. El detalle y el resto de lo que se decidió están
en [panel-de-firma.md](panel-de-firma.md#certificado).

Validado el 25/09/2026; el selector de certificado y la barra única, el
27/09/2026; la validez de las firmas en el resumen y el diálogo «Ver firmas», el
03/10/2026. Desde entonces la verdad de la ventana, la cabecera, las pestañas, el
visor, el panel y los diálogos son sus historias, y el artboard `Main` ya no
existe.
