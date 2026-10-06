# Panel de firma

Columna derecha de 380 px. Reúne lo que hay que decidir antes de firmar —con
qué certificado, la firma visible y dónde se guarda— y termina en el botón que
firma. Tras firmar, o al abrir un documento solo para verlo, la misma columna
pasa a ser el resumen de las firmas.

**La verdad del dibujo es el código y sus historias.** Esta ficha cuenta el
flujo y el porqué; los textos salen del catálogo y no se copian aquí, solo se
citan sus claves.

| Pieza | Componente | Historias |
| --- | --- | --- |
| Panel antes de firmar | `signing/SigningPanel.tsx` | «Panel de firma/1 · Antes de firmar» |
| Panel de firmas: acuse y lectura de firmas | `signing/SignaturesPanel.tsx` | «Panel de firma/2 · Firmado» |
| Selector de certificado | `signing/CertificateSelect.tsx` | «Firma/CertificateSelect» |
| Tarjeta de certificado | `signing/CertificateCard.tsx` | «Firma/CertificateCard» |
| Aviso de firmas previas | `signing/PreviousSignaturesNotice.tsx` | las de «Antes de firmar» que empiezan por `PreviousSignatures` |
| Pie fijo | `signing/PanelFooter.tsx` | las dos carpetas de arriba |

`SignaturesPanel` y `CertificateSelect` se exportan en la entrada de `/design-sync`:
no reciben puertos, solo datos y órdenes. `SigningPanel` no se exporta porque
lo conecta `App.tsx` a la vista previa del visor y a la colocación.

## Casos de uso que la usan

- Firmar un PDF en local — desde que hay un documento abierto hasta «firmado».
- Ver las firmas de un documento desde la terminal — `rfirma verify -i <doc>
  -gui` abre directamente el resumen.

## Estructura

**Zona que se desliza** arriba y **pie fijo** abajo. No hay cabecera de
documento: el nombre ya está en la [pestaña](pestanas-de-documentos.md).

Zona que se desliza, de arriba abajo, cada bloque solo cuando toca:

1. **Certificado**: el selector, siempre el primero mientras se puede firmar.
2. **Aviso de firmas previas**, si el PDF ya trae firmas.
3. **Sin certificados**, **el resumen** o **el error de firma**, según el
   estado.
4. **Firma visible**: rótulo e interruptor; encendida, el segmentado de páginas
   y su línea o campo.
5. **Modelo**, **rúbrica** y, con Personalizada, **la frase**.

Pie fijo: **el destino** con su `Cambiar` y **el botón de firmar**, a secas.

### Geometría

- Panel de 380 px, borde izquierdo de 1 px en `--rf-border-subtle`, fondo
  `--rf-bg`. Las historias lo enmarcan a 380 × 608: los 446 px de la zona que
  se desliza más el pie.
- **Zona que se desliza**: relleno `14px 24px 12px`, 12 px entre bloques. Los
  bloques se separan por el aire y su rótulo, sin divisores ni bordes
  decorativos. Si no cabe, se desplaza y el pie no se mueve.
- **Pie**: **162 px en todos los estados**, relleno `10px 24px 16px`, 10 px
  entre sus dos filas, borde superior de 1 px en `--rf-border-subtle`.
- **Rótulos**: `.rf-label`.
- **Segmentado de páginas**: 32 px de alto, 2 px de relleno, borde
  `--rf-border-strong`, `--rf-radius-md`; el elegido en `--rf-primary` /
  `--rf-on-primary` y peso 700.
- **Tarjetas de modelo**: rejilla de tres columnas con 6 px entre ellas; la
  elegida, borde `--rf-primary` doble y fondo `--rf-surface`. Modelo, rúbrica y
  frase van a 8 px entre sí.
- **Caja del destino**: relleno `7px 10px`, `--rf-radius-md`, `--rf-surface`,
  borde `--rf-border-subtle`; arriba la carpeta en `--rf-text-muted`, debajo el
  nombre en negrita.
- **Botón de firmar**: fila de 44 px, un botón primario a todo lo ancho, al 55 %
  mientras no se puede pulsar.

## Estados

Cada uno tiene su historia en «Antes de firmar», salvo que se diga otra cosa:

| Estado | Historia | Qué cambia |
| --- | --- | --- |
| Listo | `Ready` | Selector con el certificado elegido; firmar disponible |
| Sin certificado elegido | `Unchosen` | Selector sin elegido, firmar al 55 %, firma visible apagada y desactivada con su ayuda |
| Buscando certificados | `Searching` | La caja lo dice con un indicador; firmar al 55 % |
| Sin certificados | `NoCertificates` | Aviso arriba; el pie ofrece añadir un certificado y volver a buscar. No hay selector |
| Búsqueda fallida | `SearchFailed` | El aviso cuenta que no se pudo buscar, no que no haya ninguno |
| Varios certificados | `SeveralCertificates` | El desplegable los lista todos al abrirse |
| Certificados abiertos | `Open` de «Firma/CertificateSelect» | El buscador en lugar de la caja y la lista flotando |
| Firma visible | `VisibleSignature*` | Una página, varias, todas, otra página a la vista y sin colocar |
| Rango con error | `RangeOutOfDocument` | Campo en error y firmar al 55 %. Es lo único que apaga firmar |
| Modelos y rúbrica | `CompleteModelWithRubric`, `RubricOnlyModel`, `RubricWithoutImage`, `RubricFailed`, `CustomModel` | Ver «El modelo» y «La rúbrica» |
| Firmas previas | `PreviousSignatures*`, `ClosedDocument` | Ver «El aviso de firmas previas» |
| Destino no disponible | `UnwritableDestination` | La caja avisa; firmar no se apaga y `Cambiar` sigue ahí |
| Firmando | `Signing` | Selector, interruptor, controles y `Cambiar` al 35 %; firmar al 55 % con su texto de avance; encima, el [diálogo de progreso](dialogo-progreso-firma.md) |
| Error al firmar | `SigningFailed`, `KeyringPinMissing` | La zona que se desliza se sustituye por la tarjeta del fallo; el pie ofrece reintentar y volver |

**El error de firma sustituye a la zona que se desliza** y no se mete en el pie:
el pie tiene 162 px fijos en todos los estados. La tarjeta es `ErrorNotice` con
borde de 2 px en `--rf-border-strong` y el documento intacto. Si la causa es que
el llavero perdió el PIN del Almacén de rFirma, ofrece además vaciarlo, con su
misma confirmación (ADR-0034).

## El aviso de firmas previas

Si el PDF ya trae firmas, es el primer bloque de la zona que se desliza después
del certificado. Firmar junto a ellas es lo normal; el aviso dice cuántas hay y
si alguna tiene un problema, **en una sola línea**. El detalle está en el
diálogo [«Ver firmas»](dialogo-ver-firmas.md).

- **La línea** es de 36 px de alto, con un relleno de 10 px a la izquierda y 2 a
  la derecha: el icono de la **peor validez**, el recuento y, si hay problemas,
  cuántos. A la derecha, la única acción, un botón fantasma que abre el diálogo.
  No se despliega, no hay lista ni motivos. En 380 px caben tres firmas con tres
  problemas con 5 px de holgura, y por eso el relleno es de 10/2 y no de 12/4.
- **El recuento de problemas** habla de caducadas solo cuando todos los
  problemas lo son; en cuanto hay una no válida o un hallazgo del documento,
  habla de problemas.
- **El tono** lo dan el icono y el borde, sin color (ver «Decisiones»):

| Lo peor que hay | Icono | Borde |
| --------------- | ----- | ----- |
| Todo válido | información | 1 px `--rf-border-subtle` |
| Alguna caducada, ninguna no válida ni hallazgo | triángulo | 2 px `--rf-border-strong` |
| Alguna no válida, o un hallazgo del documento | círculo con aspa | 2 px `--rf-border-strong` |

- **La franja del pie del aviso** (fondo `--rf-surface`, borde superior de 1 px)
  dice si el certificado elegido ya firmó el documento: mismo NIF de titular y
  misma entidad representada (`organizationIdentifier`; «ninguna» cuenta como
  valor). Dos casos, el mismo certificado u otro del mismo titular; no
  bloquea. Solo con un certificado elegido. Si el documento está cerrado a más
  firmas, la franja lo dice en su lugar.
- **El botón del pie no cambia** con problemas. Pulsarlo abre
  [«¿Firmar de todos modos?»](dialogo-firmar-de-todos-modos.md) si hay algún ⚠ o
  ✗: una caducada, una no válida o un hallazgo.

Claves: `panel.previousSignatures.*`. Historias: `PreviousSignaturesAllValid`,
`PreviousSignaturesExpired`, `PreviousSignaturesWithProblems`,
`PreviousSignaturesSameCertificate`, `PreviousSignaturesOtherCertificate` y
`ClosedDocument`.

### La validez de cada firma

La regla es la del ADR-0043, y la misma en el aviso, en «Ver firmas», en el
resumen y en «¿Firmar de todos modos?». Una firma tiene una **validez** de tres,
con la misma silueta, palabra y peso en todas partes
([design-system.md](design-system.md#8-accesibilidad)):

- **Válida**: círculo con marca, en `--rf-text-muted` y peso normal.
- **Caducada**: triángulo, en `--rf-text` a peso 700. El motivo dice cuándo
  caducó el certificado.
- **No válida**: círculo con aspa, en `--rf-text` a peso 700, con su motivo
  (`signatureReason.*`).

**Un hallazgo del documento** (`documentFinding.*`) es lo que se encuentra en
el documento sin poder atribuirlo a una firma. Va en su propia línea, encima de
las firmas, con el círculo con aspa, peso 700 y borde de 2 px en
`--rf-border-strong`. Pesa como una no válida y no se cuelga de ninguna.

## Firma visible

**Apagada por defecto**, y firmar sigue permitido. **Encenderla la coloca**: el
recuadro aparece en la página a la vista, abajo a la derecha, y se despliega la
configuración. No existe «encendida y sin colocar». Apagar y volver a encender
recupera todo lo que había: modo, páginas, rango, modelo, rúbrica y posición.
Sin certificado elegido el interruptor está apagado y desactivado, y una ayuda
dice por qué (`panel.visibleSignature.needsCertificate`).

### En qué páginas

| Modo | Debajo |
| --- | --- |
| Una página | La página colocada; si se mira otra, un botón que la mueve a la vista |
| Varias | El campo de rango y, a su derecha, el botón que añade o quita la página a la vista |
| Todas | Nada |

Claves: `panel.placement.*`.

**Cada modo guarda su conjunto.** Cambiar de modo no reescribe el que dejas; la
primera vez que se elige uno se siembra del anterior, y a partir de ahí es suyo.
El recuadro es uno solo: cambiar de modo no lo mueve.

**El campo de «Varias»** usa el formato de impresión (`1,6` o `1,60-70,184`).
Nada se recorta ni se ignora: lo que no se puede resolver se dice en negrita con
el triángulo, el campo pasa a borde de 2 px en `--rf-text` y el botón de firmar
se apaga al 55 %. Los cinco errores —más allá del documento, al revés, cero,
mal formado y vacío— son `panel.placement.errors.*`. Los botones de añadir y
quitar reescriben el campo en forma comprimida: los dos caminos no pueden
discrepar.

**La posición estándar**, la de encender y la del botón que mueve el recuadro,
es abajo a la derecha, a un margen del borde. Solo el trazo o el arrastre en el
[visor](visor-de-documento.md) eligen sitio.

Con varias páginas, **el recuadro está en el mismo sitio en todas**: es un solo
campo de firma con el widget replicado, validado por VALIDe como PAdES B-Level
con un firmante
([recuadro-replicado-pdfsig.md](../research/recuadro-replicado-pdfsig.md)).

### El modelo

Tres tarjetas iguales, cada una con un esbozo de su firma visible (claves
`panel.visibleSignature.model.*`):

- **Completa**: la frase que AutoFirma estampa por omisión. La miniatura enseña
  firmante, fecha y emisor, una línea cada uno, y la rúbrica a la izquierda si
  está encendida.
- **Solo rúbrica**: la imagen ocupando el recuadro.
- **Personalizada**: la miniatura no pinta la frase, que no cabe; la esboza con
  dos renglones de barras de texto y pastillas de dato.

**No hay fuente, tamaño ni color.** El texto se ajusta al recuadro: rFirma lo
compone, lo envía resuelto en `layer2Text` y con `layer2FontSize = 0`, que es lo
que hace que iText reparta la letra según el alto.

**La frase de Personalizada** se ve tal como saldrá, con los datos del
certificado como pastillas dentro del texto (fondo `--rf-border-subtle`, radio
`sm`, parten de línea como el texto). Se mueven y se borran como una palabra; el
resto es texto libre. El botón de añadir dato, dentro del campo abajo a la
derecha, abre un menú con **Firmante**, **Emisor** y **Fecha**, cada uno con su
valor de muestra (`panel.visibleSignature.phrase.*`).

**El DNI no es un dato aparte**: va dentro de Firmante, que es el `CN` del
certificado, enmascarado como lo hace AutoFirma (`*`, tres ocultos y cuatro
visibles). La máscara protege de la lectura casual, no del documento: el
certificado viaja entero dentro de la firma. Los certificados de seudónimo
quedan exentos.

### La rúbrica

Una fila siempre visible bajo las tarjetas: interruptor, miniatura y botón para
cambiarla o cargarla (`panel.visibleSignature.rubric.*`).

- **Encendida**: las tarjetas y el recuadro del visor la llevan, y «Solo
  rúbrica» se puede elegir.
- **Apagada**: ninguna la lleva y «Solo rúbrica» queda al 45 %, sin poder
  elegirse.
- **Con «Solo rúbrica» elegida**, el interruptor queda encendido y bloqueado al
  45 %.
- **Encendida sin imagen**: miniatura punteada, y un hueco punteado en las
  tarjetas y en el recuadro donde irá.

La miniatura enseña **el fichero que se va a firmar**, ya normalizado: la rúbrica
viaja como JPEG sin alfa, así que un PNG recortado sale sobre blanco y así se ve.
El selector filtra PNG y JPEG; una imagen demasiado grande se reduce en silencio,
y los fallos se dicen al elegir, nunca al firmar
([ADR-0012](../adr/0012-normalizacion-de-la-rubrica-en-rust.md)).

## Guardar en

La carpeta y el nombre del fichero que se va a escribir, **antes** de firmar
([ADR-0011](../adr/0011-destino-del-documento-firmado.md)). La carpeta se recorta
por la cola y el nombre por el medio, conservando siempre `-firmado`, su número
de desempate y la extensión: es el componente **ruta de destino** de
[design-system.md](design-system.md), y en el pie cada línea va en una sola fila
con elipsis y entera en el `title`. Claves: `panel.footer.*`.

- `Cambiar` abre **el diálogo de guardar**, que fija carpeta y nombre a la vez, y
  vale **solo para esta firma**: no toca la preferencia.
- **Segunda firma** del mismo original: el nombre lleva su número de desempate.
  La aplicación no borra nada del usuario, así que no hay «reemplazar».
- **Destino no disponible**: la caja se cambia por el aviso con el triángulo y
  borde de 2 px, a la misma altura, con la carpeta en negrita. El botón de firmar
  **no se apaga** ni se degrada a otro destino: `Cambiar` sigue ahí.

## Certificado

**El primer bloque del panel.** Es **el mismo componente** que el de la
[ventana de sede](ventana-de-sede.md), y se describe solo aquí. Historias:
«Firma/CertificateSelect». Cada fila es una
[`CertificateCard`](#componentes-y-tokens), la misma que usa la lista de
certificados de [Preferencias](preferencias.md).

**Cerrado**, una caja de dos líneas con un chevron: 52 px de alto mínimo,
relleno `6px 12px`, borde de 1 px en `--rf-border-strong`, `--rf-radius-md`,
fondo `--rf-bg`, cada línea en una sola fila con elipsis. Dice **el certificado
elegido**: el titular o, si es de representante, la entidad, y debajo la
capacidad. Sin elegir, `panel.certificate.chooseOne`; buscando,
`panel.certificate.loading` con el indicador.

**Abierto**, la caja se convierte en el buscador —lupa, borde de 2 px en
`--rf-primary`— y la lista cuelga justo debajo, **hacia abajo**, a 4 px y a todo
el ancho del selector: 480 px de alto máximo, relleno 6 px, `--rf-radius-lg`,
borde `--rf-border-subtle`, `--rf-bg`, `--rf-shadow-elevated`. Flota sobre el
resto del panel y el pie, que no se mueven.

- **Una fila por certificado.** El mismo certificado —el mismo número de serie—
  en varios almacenes es **una** fila con todas sus etiquetas.
- **Empresa primero**: en un certificado de representante, la entidad es la
  primera línea.
- **El DNI en claro**: la lista no sale del ordenador. La máscara es solo de la
  firma visible.
- **Agrupada** en disponibles y no disponibles
  (`panel.certificate.groups.*`), y dentro de cada grupo orden alfabético por la
  primera línea con `localeCompare` del idioma.
- **Un certificado caducado, aún no vigente o revocado se lista, dice por qué y
  no se deja elegir**: nombre en `--rf-text-muted`, sin cursor, con su icono y su
  motivo. Esconderlo dejaría a quien viene a firmar con él mirando una lista
  donde falta. Hoy no se comprueba la revocación, solo las fechas.
- **El buscador** filtra por titular, entidad, NIF o DNI, emisor y almacén. Con
  texto encabeza la lista el recuento, y sin coincidencias, el aviso de que no
  hay ninguna (`panel.certificate.matches`, `panel.certificate.noMatch`).
  `Escape` cierra y vacía la búsqueda, y elegir también la vacía.
- **Se recuerda al firmar con él**, no al elegirlo, y la próxima sesión sale ya
  puesto ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).
- La lista se cierra al elegir, al pulsar fuera y con `Escape`.

**La primera vez no hay certificado elegido.** No se preselecciona ninguno,
ni siquiera cuando hay uno solo —la identidad con que se firma no la elige la
aplicación—, y nunca uno que no sirva.

## El resumen: tras firmar y con `verify --gui`

Un solo resumen, y se ve igual llegues como llegues: después de firmar, o
abierto por `rfirma verify -i <doc> -gui`. Lista **todas las firmas del
documento tal como ha quedado**, con lo mismo que `rfirma verify -v` y nada más.
Sustituye a la zona que se desliza: no hay selector, ni aviso de firmas
previas, ni firma visible, ni modelo. Historias: «Panel de firma/2 · Firmado».

| Situación | Historia |
| --- | --- |
| Recién firmado, con la tuya marcada como nueva | `JustSigned` |
| Recién firmado sobre un documento modificado antes | `JustSignedAfterChangedDocument` |
| `verify`, con firmas | `VerifyWithSignatures` |
| `verify`, con firmas caducadas, no válidas y hallazgos | `VerifyWithProblems` |
| CAdES con contrafirmas | `VerifyCadesWithCountersignatures` |
| XAdES | `VerifyXades` |
| No se puede firmar desde aquí | `VerifyNotSignable` |
| Sin firmas | `VerifyWithoutSignatures` |
| Formato desconocido | `VerifyUnrecognizedFormat` |
| Leyendo | `VerifyReading` |
| Fallo al leer las firmas | `VerifyReadFailed` |
| Fallo al abrir lo que se pidió | `OpenFailed` |

De arriba abajo, cada bloque cuando toca:

- **Solo tras firmar**, una franja con el círculo con marca y la hora
  (`panel.signed.signedAt`): fondo `--rf-surface`, borde de 1 px en
  `--rf-border-subtle`, `--rf-radius-md`. Con `verify` no hay franja.
- **El título** con el icono de documento, y dos insignias `.rf-badge`: el
  formato y el recuento, con las contrafirmas si las hay.
- **Los hallazgos del documento**, uno por línea y encima de las fichas.
  **Tras firmar, «modificado después de la última firma» se lee desde tu firma**
  (`documentFinding.*`): la última firma es ya la tuya, y el cambio es anterior
  a ella.
- **Una ficha por firma**, apiladas y todo a la vista; las mismas del diálogo
  [«Ver firmas»](dialogo-ver-firmas.md) (`SignatureCards.tsx`). Cabecera con el
  número y la **validez**; tras firmar, la tuya va la última y lleva la insignia
  de nueva. Debajo, una fila por dato —firmante, en nombre de, emisor, fecha o
  sello, y motivo si no es válida— con el rótulo a 104 px fijos
  (`panel.signed.field.*`). **Un campo ausente no se pinta.**
- **Las contrafirmas van dentro de la firma que contrafirman** (CAdES y XAdES),
  con un filete de 2 px en `--rf-border-strong` a la izquierda y 12 px de
  sangría y su propia validez. En PDF no hay árbol.

**Sin algoritmo ni número de serie**: son de `verify -vv`, y la interfaz enseña
lo de `verify -v`, que ahora incluye la validez. **Tu ficha tiene las mismas
filas que las demás**: lo propio de la firma recién hecha —la firma visible, sus
páginas, el recuadro— no sale en el resumen; la firma estampada se sigue viendo
en la hoja del visor.

Con `verify`, cuando no hay lista:

- **Sin firmas**: el icono de documento y el título, con el patrón de «sin
  certificados» (`panel.signed.none.title`).
- **Formato desconocido**: el triángulo, el título y el cuerpo
  (`panel.signed.unrecognized.*`).
- **Fallo al leer las firmas**: la tarjeta del error de firma, con el título
  propio (`panel.signed.readFailed.title`) y el detalle técnico.

**El pie, rotulado siempre como documento**, con la caja de la carpeta y el
nombre: tras firmar, el fichero firmado y `Cambiar`, que mueve el destino
(ADR-0011); con `verify`, el fichero abierto y `Cambiar` oculto. La fila de 44 px
es la misma en los dos: abrir el documento (primario), abrir la carpeta
(secundario) y firmar (fantasma). Los dos primeros usan el portal `OpenURI`: bajo
el sandbox son la única forma de llegar al fichero sin saberse la ruta.

**Firmar vuelve al panel con el documento releído del disco**, como
`rfirma doc.pdf`. Si el fichero no es PDF, queda al 55 % con su `title`
explicándolo. El acuse es de un documento concreto: al cambiar de pestaña o
cerrarla, se va.

## Componentes y tokens

Primitivos: `Button`, `Row`, `Badge`, `Popover` y `Card`. Pieza de dominio:
`CertificateCard`, que usan el selector y la lista de Preferencias. El
interruptor es el de [design-system.md](design-system.md). Clases: `.rf-label`,
`.rf-input`, `.rf-body`, `.rf-prose`. Tokens: `--rf-surface`, `--rf-bg`,
`--rf-primary`, `--rf-on-primary`, `--rf-border-strong`, `--rf-border-subtle`,
`--rf-text-muted`, `--rf-radius-sm|md|lg` y `--rf-shadow-elevated`. Las medidas
son de `SigningPanel.css`, `SignaturesPanel.css`, `SignatureCards.css` y
`CertificateSelect.css`.

## Decisiones

- **El selector de certificado se separa del botón de firmar** y sube al
  principio del panel; el botón queda en firmar a secas. Es el mismo mecanismo
  que ya tenía la [ventana de sede](ventana-de-sede.md), y así las dos pantallas
  quedan iguales. Se descartó el botón partido «Firmar como <nombre> ▾»: no
  estaba claro cómo cambiar de certificado, y quien pulsaba el nombre en lugar
  de la flecha firmaba con un certificado que no quería. Separados, no se firma
  sin querer.
- **La lista se abre hacia abajo desde arriba del todo.** En Tauri un
  desplegable no puede salir de la ventana, y pegado arriba es donde tiene el
  espacio. Sigue flotando: la firma visible y el botón no se mueven al abrirla.
- **El selector recuerda el último certificado con el que se firmó** y la primera
  vez no hay preselección. Cambian el aspecto, los datos —la entidad primero en
  los de representante, las etiquetas de almacén, la caducidad— y la agrupación,
  que marca el que no se puede usar con su icono y el nombre en gris en lugar de
  atenuar la fila al 45 %.
- **Una fila por certificado, con etiquetas de almacén.** El mismo certificado
  en varios almacenes salía en filas duplicadas que no había forma de
  distinguir.
- **El buscador es lo único nuevo**: gestorías y representantes manejan muchos
  certificados.
- **El DNI va en claro** en el selector, sin la máscara de la firma visible: el
  listado no sale del ordenador y el número ya se ve en los almacenes del
  sistema y de los navegadores; el texto de la firma visible, en cambio, viaja
  dentro del documento.
- **Sin cabecera de documento**: nombre en la pestaña, páginas en el visor.
- **Modelos en lugar de casillas por dato.** La v0.3.1 tenía cinco casillas que
  componían un párrafo. Tres tarjetas con la firma real en miniatura dicen el
  resultado sin leer; el caso de quien quiere otra cosa lo cubre Personalizada.
  Se descartaron un modelo «Breve» (lo hace Personalizada) y la frase editable
  como alternativa suelta.
- **Sin comodines ni motivo.** AutoFirma compone el texto con comodines entre
  `$$` que se escriben a mano y no se ven resueltos hasta firmar. Las pastillas
  son esos datos, pero se insertan desde un menú, no se escriben mal y se ven ya
  sustituidos. El motivo era un segundo campo de texto libre junto a una frase
  que ya lo es.
- **La rúbrica, una fila siempre visible.** Se descartó añadirla y quitarla desde
  un hueco en el propio recuadro del visor: escondía la opción dentro del objeto
  que se arrastra. Y se descartó la casilla con un bloque de imagen aparte: dos
  sitios para una decisión.
- **Firma visible apagada por defecto y encendida = colocada.** «Colocado» dejó
  de ser una bandera: con recuadro y sin páginas era un estado que nadie sabía
  dibujar.
- **La elección de páginas vive en el panel**, no en miniaturas: una tira se
  rompe a las 200 páginas.
- **El error de firma sustituye al panel**, como el resumen, en lugar de meter la
  tarjeta en el pie.
- **El pie enseña carpeta y nombre**, en una caja, bajo un solo `Cambiar`: el
  nombre lo elige la aplicación y hasta firmar no se veía.
- **«Firmar otro documento» no existe**: abrir desde la cabecera ya abre.
- **El aviso de firmas previas dice la validez, no solo el número, y es una
  línea.** Se descartó el aviso desplegable, con una fila por firma, su
  veredicto y su motivo: en 380 px empujaba la firma visible fuera de la vista.
  El diálogo es el mismo desde el panel y desde la sede.
- **Tres valideces, y la misma en todas partes** (ADR-0043). Se descartó el
  cuarto valor «no se ha podido comprobar del todo», que describía una
  limitación del validador y no algo de la firma.
- **El hallazgo del documento va aparte**, en su línea, y no se cuelga de la
  última firma: la pintaría no válida sin serlo.
- **Monocromo.** Ni ámbar para la caducada ni rojo para la no válida: la paleta
  no tiene token de color para eso y los colores literales no se admiten
  ([design-system.md](design-system.md#2-color)).
- **Fecha o sellada**, no «fecha declarada»: el sello de una TSA es lo único que
  fecha una firma, y cuando lo hay se dice con su nombre.
- **Un solo resumen para después de firmar y para `verify --gui`.** `verify
  -gui` abría el escritorio con el fichero cargado para firmarlo; ahora abre el
  resumen. Se descartó una pantalla propia de verificación: dos resúmenes del
  mismo documento que dijeran cosas distintas. Por eso el resumen de después de
  firmar dejó de ser un acuse de tu firma y pasó a listar todas, con la tuya
  marcada como nueva.
- **Fichas apiladas, todo a la vista.** Se descartaron las fichas plegables y la
  lista compacta con la ficha de la elegida debajo: con las una a cuatro firmas
  de un documento normal, serían un clic por firma para leer lo que `verify -v`
  da sin pedirlo.
- **El pie conserva firmar.** Se descartaron el pie sin firmar y quitar el pie
  entero: tras ver las firmas, lo siguiente suele ser firmar, y el resumen no
  debe ser un callejón.
- **Sin número de serie** en las fichas: es de `verify -vv`.

Validado en el lienzo original, hoy congelado, entre el 25/09/2026 y el
03/10/2026. Desde entonces su verdad son el código y las historias.
