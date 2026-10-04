# Panel de firma

Columna derecha de 380 px. Reúne lo que hay que decidir antes de firmar —con
qué certificado, la firma visible y dónde se guarda— y termina en el botón que
firma.

## Casos de uso que la usan

- Firmar un PDF en local — desde que hay un documento abierto hasta «firmado».
- Ver las firmas de un documento desde la terminal — `rfirma verify -i <doc>
  -gui` abre directamente el resumen.

## Estructura

**Zona que se desliza** arriba y **pie fijo** abajo.

Zona que se desliza, de arriba abajo, cada bloque solo cuando toca:

1. **Certificado**: el selector, siempre el primero mientras se puede firmar.
2. **Aviso de firmas previas**, si el PDF ya trae firmas.
3. **«Sin certificados»**, **el resumen** («Firmas del documento») o **el error
   de firma**, según el estado.
4. **Firma visible**: rótulo e interruptor; encendida, el segmentado de páginas
   y su línea o campo.
5. **Modelo**, **Con rúbrica** y, con Personalizada, **la frase**.

Pie fijo:

1. **«Guardar en»** con `Cambiar` («Documento» en el resumen), y una caja con
   la carpeta y el nombre del fichero.
2. **«Firmar»**, a secas.

No hay cabecera de documento: el nombre ya está en la
[pestaña](pestanas-de-documentos.md).

### Geometría

- Panel de 380 px, borde izquierdo de 1 px en `--rf-border-subtle`, fondo
  `--rf-bg`.
- **Zona que se desliza**: relleno `14px 24px 12px`, 12 px entre bloques. Los
  bloques se separan por el aire y su rótulo, sin divisores ni bordes
  decorativos. Deja 446 px en la ventana de 700; si no cabe, se desplaza y el
  pie no se mueve.
- **Pie**: **162 px en todos los estados**, relleno `10px 24px 16px`, 10 px
  entre sus dos filas, borde superior de 1 px en `--rf-border-subtle`.
- **Rótulos** («Certificado», «Firma visible», «Modelo», «Guardar en»):
  `.rf-label`.
- **Interruptor**: el componente de [design-system.md](design-system.md), a la
  derecha del rótulo en «Firma visible» y delante del texto en «Con rúbrica».
- **Segmentado** «Una página | Varias | Todas»: 32 px de alto, 2 px de relleno,
  borde `--rf-border-strong`, `--rf-radius-md`; el elegido en `--rf-primary` /
  `--rf-on-primary` y peso 700.
- **Tarjetas de modelo**: rejilla de tres columnas con 6 px entre ellas; cada
  una, la miniatura de 30 px de alto y la etiqueta a 12 px. La elegida, borde
  `--rf-primary` doble (borde más `inset`) y fondo `--rf-surface`.
- **Fila de la rúbrica**: interruptor, «Con rúbrica», miniatura de 48 × 30 px y
  botón secundario de 30 px.
- **Modelo, rúbrica y frase**: 8 px entre las tarjetas, la fila de la rúbrica y
  la frase.
- **Caja del destino**: relleno `7px 10px`, `--rf-radius-md`, `--rf-surface`,
  borde `--rf-border-subtle`. Arriba la carpeta a 12 px en `--rf-text-muted` con
  icono de carpeta; debajo el nombre a 13 px en negrita con icono de PDF.
- **Botón de firmar**: fila de 44 px, un `.rf-btn--primary` a todo lo ancho.
  «Firmar», o «Firmando…» mientras firma; al 55 % sin certificado elegido,
  buscando certificados, firmando o con el rango en error.
- **Selector de certificado**: ver «Certificado», más abajo.

## El aviso de firmas previas

Si el PDF ya trae firmas, es el primer bloque de la zona que se desliza después
del certificado. Firmar junto a ellas es lo normal; el aviso dice cuántas hay y
si alguna tiene un problema, **en una sola línea**. El detalle está en el
diálogo [«Ver firmas»](dialogo-ver-firmas.md).

**La línea**, de 36 px de alto, con un relleno de 10 px a la izquierda y 2 a la
derecha:

- El icono de la **peor validez**, «Junto a **N firmas**» («**1 firma**») y, si
  hay problemas, « · **M caducadas**» cuando todos los problemas son caducadas,
  o « · **M problemas**» en cuanto hay una no válida o un hallazgo del
  documento, que también cuentan. Letra de 13 px con interlineado de 18, en una
  línea y con elipsis si no cabe.
- A la derecha, «**Ver firmas →**», `.rf-btn--ghost` de 28 px de alto y 6 px de
  relleno, que abre el diálogo. Es la única acción del aviso: no se despliega,
  no hay chevron, ni lista, ni motivos.
- Medido en el panel de 380 px: «Junto a 3 firmas · 3 problemas» y el botón
  caben con 5 px de holgura. Por eso el relleno es de 10/2 y no de 12/4.

**El tono:**

| Lo peor que hay | Icono | Borde |
| --------------- | ----- | ----- |
| Todo válido | información | 1 px `--rf-border-subtle` |
| Alguna caducada, ninguna no válida ni hallazgo | triángulo | 2 px `--rf-border-strong` |
| Alguna no válida, o un hallazgo del documento | círculo con aspa | 2 px `--rf-border-strong` |

**«Ya lo firmaste tú»** va en una franja al pie del aviso, fondo
`--rf-surface`, borde superior de 1 px, icono de persona y texto en negrita.
Solo con un certificado elegido. Dos textos: «Ya lo firmaste tú con este
certificado» y «Ya lo firmaste tú, con otro certificado tuyo». No bloquea.

**El botón del pie no cambia**: con problemas sigue siendo «Firmar», primario.
Pulsarlo abre [«¿Firmar de todos modos?»](dialogo-firmar-de-todos-modos.md) si
hay **algún ⚠ o ✗**: una caducada, una no válida o un hallazgo.

### La validez de cada firma

La regla es la del ADR-0043, y la misma en el aviso, en «Ver firmas», en el
resumen y en «¿Firmar de todos modos?». Una firma tiene una **validez** de tres,
con la misma silueta, palabra y peso en todas partes
([design-system.md](design-system.md#8-accesibilidad)):

- **✓ Válida**: círculo con marca, en `--rf-text-muted` y peso normal.
- **⚠ Caducada**: triángulo, en `--rf-text` a peso 700. El motivo dice cuándo
  caducó el certificado.
- **✗ No válida**: círculo con aspa, en `--rf-text` a peso 700, con su motivo:
  «Se ha modificado después de firmarse», «La firma está dañada», «El
  certificado no se podía usar antes del …», «<firmante> no admitía más firmas»
  o «rFirma no conoce este tipo de firma».

**Un hallazgo del documento** —«Se ha modificado después de la última firma»,
«Se ha rellenado el formulario después de firmar», «Se ha añadido contenido
encima de lo firmado»— es lo que se encuentra en el documento sin poder
atribuirlo a una firma. Va en su propia línea, encima de las firmas, con el
círculo con aspa, a 13 px y peso 700 y con borde de 2 px en
`--rf-border-strong`. Pesa como una no válida y no se cuelga de ninguna.

**«Ya lo firmaste tú»** es el mismo NIF del titular y la misma entidad
representada (`organizationIdentifier`; «ninguna» cuenta como valor). Si el
certificado es el mismo, «con este certificado»; si no, «con otro certificado
tuyo».

## Firma visible

**Apagada por defecto**, y firmar sigue permitido. **Encenderla la coloca**: el
recuadro aparece en la página a la vista, abajo a la derecha, y se despliega la
configuración. No existe «encendida y sin colocar». Apagar y volver a encender
recupera todo lo que había: modo, páginas, rango, modelo, rúbrica y posición.

### En qué páginas

| Modo | Debajo |
| --- | --- |
| Una página | «En la página 6»; si se mira otra, «Ponerla aquí», que la mueve |
| Varias | el campo de rango y, a su derecha, «Ponerla aquí» si la página a la vista no está o «Quitarla de aquí» si está |
| Todas | nada |

**Cada modo guarda su conjunto.** Cambiar de modo no reescribe el que dejas; la
primera vez que se elige uno se siembra del anterior, y a partir de ahí es suyo.
El recuadro es uno solo: cambiar de modo no lo mueve.

**El campo de «Varias»** usa el formato de impresión, `1,6` o `1,60-70,184`.
Nada se recorta ni se ignora (ID-22): lo que no se puede resolver se dice en
negrita con el triángulo, el campo pasa a borde de 2 px en `--rf-text` y el botón
de firmar se apaga al 55 %. **Es lo único que apaga firmar.**

| Lo escrito | Lo que se ve |
| --- | --- |
| `10-40` en un documento de 6 | «Solo hay 6 páginas» |
| `6-1` | ««6-1» va al revés» |
| `1;6` | «Separa las páginas con comas» |

«Ponerla aquí» y «Quitarla de aquí» reescriben el campo en forma comprimida: los
dos caminos no pueden discrepar.

**La posición estándar**, la de encender y la de «Ponerla aquí», es abajo a la
derecha, a un margen del borde. Solo el trazo o el arrastre en el
[visor](visor-de-documento.md) eligen sitio.

Con varias páginas, **el recuadro está en el mismo sitio en todas**: es un solo
campo de firma con el widget replicado, validado por VALIDe como PAdES B-Level
con un firmante
([recuadro-replicado-pdfsig.md](../research/recuadro-replicado-pdfsig.md)).

### El modelo

Tres tarjetas iguales, cada una con un esbozo de su firma visible:

- **Completa**: la frase que AutoFirma estampa por omisión, «Firmado por
  **[Firmante]** el día **[Fecha]** con un certificado emitido por
  **[Emisor]**». La miniatura enseña firmante, fecha y emisor, una línea cada
  uno, y la rúbrica a la izquierda si está encendida.
- **Solo rúbrica**: la imagen ocupando el recuadro.
- **Personalizada**: la miniatura no pinta la frase, que no cabe; la esboza con
  dos renglones de barras de texto y pastillas de dato.

**No hay fuente, tamaño ni color.** El texto se ajusta al recuadro: rFirma lo
compone, lo envía resuelto en `layer2Text` y con `layer2FontSize = 0`, que es lo
que hace que iText reparta la letra según el alto.

**La frase de Personalizada** se ve tal como saldrá: «Visto bueno de
**[Firmante]**, **[Fecha]**», con los datos del certificado como pastillas dentro
del texto (fondo `--rf-border-subtle`, radio `sm`, parten de línea como el
texto). Se mueven y se borran como una palabra; el resto es texto libre. «+ Dato»,
dentro del campo abajo a la derecha, abre un menú con **Firmante**, **Emisor** y
**Fecha**, cada uno con su valor de muestra.

**El DNI no es un dato aparte**: va dentro de Firmante, que es el `CN` del
certificado, enmascarado como lo hace AutoFirma (`*`, tres ocultos y cuatro
visibles): «MARTÍN ORTEGA LUCÍA - ***9999**». La máscara protege de la lectura
casual, no del documento: el certificado viaja entero dentro de la firma. Los
certificados de seudónimo quedan exentos.

### La rúbrica

Una fila siempre visible bajo las tarjetas: interruptor «Con rúbrica», miniatura
y «Cambiar…» (o «Cargar…» sin imagen).

- **Encendida**: las tarjetas y el recuadro del visor la llevan, y «Solo rúbrica»
  se puede elegir.
- **Apagada**: ninguna la lleva y «Solo rúbrica» queda al 45 %, sin poder
  elegirse (`title` «Necesita la rúbrica»).
- **Con «Solo rúbrica» elegida**, el interruptor queda encendido y bloqueado al
  45 % (`title` «Solo rúbrica la necesita»).
- **Encendida sin imagen**: miniatura punteada, «Cargar…», y un hueco punteado en
  las tarjetas y en el recuadro donde irá (`title` «Sin rúbrica cargada»).

La miniatura enseña **el fichero que se va a firmar**, ya normalizado: la rúbrica
viaja como JPEG sin alfa, así que un PNG recortado sale sobre blanco y así se ve.
El selector filtra PNG y JPEG; una imagen demasiado grande se reduce en silencio,
y los tres fallos —no es PNG ni JPEG, está dañada, es demasiado grande— se dicen
al elegir, nunca al firmar
([ADR-0012](../adr/0012-normalizacion-de-la-rubrica-en-rust.md)).

## Guardar en

La carpeta y el nombre del fichero que se va a escribir, **antes** de firmar
([ADR-0011](../adr/0011-destino-del-documento-firmado.md)). La carpeta se recorta
por la cola y el nombre por el medio, conservando siempre `-firmado`, su número
de desempate y la extensión: es el componente **ruta de destino** de
[design-system.md](design-system.md), y en el pie cada línea va en una sola fila
con elipsis y entera en el `title`.

- `Cambiar` abre **el diálogo de guardar**, que fija carpeta y nombre a la vez, y
  vale **solo para esta firma**: no toca la preferencia.
- **Segunda firma** del mismo original: `…-firmado-2.pdf`. La aplicación no borra
  nada del usuario, así que no hay «reemplazar».
- **Destino no disponible**: la caja se cambia por «No se puede escribir en
  **Documentos**» con el triángulo y borde de 2 px, a la misma altura. El botón
  de firmar **no se apaga** ni se degrada a otro destino: `Cambiar` sigue ahí.

## Certificado

**El primer bloque del panel**, bajo el rótulo «Certificado». Es **el mismo
componente** que el de la [ventana de sede](ventana-de-sede.md), y se describe
solo aquí.

**Cerrado**, una caja de dos líneas con ▾: 52 px de alto mínimo, relleno
`6px 12px`, borde de 1 px en `--rf-border-strong`, `--rf-radius-md`, fondo
`--rf-bg`. Arriba, a 14 px y peso 600; debajo, en `.rf-body rf-text-muted`; cada
una en una sola línea con elipsis. Dice **el certificado elegido**:

- Personal: el titular, y debajo «A título personal · 00099990D».
- De representante: la entidad, y debajo «Lucía Martín, representante».
- Sin elegir: «Elige un certificado», en `--rf-text-muted`.
- Buscando: el indicador y «Buscando certificados…».

**Abierto**, la caja se convierte en el buscador —lupa, borde de 2 px en
`--rf-primary`, «Nombre, empresa, NIF o almacén»— y la lista cuelga justo
debajo, **hacia abajo**, a 4 px y a todo el ancho del selector: 480 px de alto
máximo, relleno 6 px, `--rf-radius-lg`, borde `--rf-border-subtle`, `--rf-bg`,
`--rf-shadow-elevated`. Flota sobre el resto del panel y el pie, que no se
mueven ([desplegable](design-system.md#desplegable)).

**Cada fila**, con **empresa primero**:

1. A 13 px y peso 600, el titular o, si es de representante, la entidad.
2. En `.rf-body`: «A título personal · 00099990D», o «Lucía Martín,
   representante · G12345678» con el NIF de la entidad.
3. Una etiqueta `.rf-badge` de 11 px por almacén —«Firefox», «Chrome»,
   «Windows», «Instalado en rFirma», nunca una ruta— y la caducidad, «Caduca en
   03/2028», en `--rf-text-muted`.
4. Solo si no se puede usar: el icono —reloj si caducó, círculo tachado si está
   revocado— y el motivo en negrita, «Caducó el 3 de marzo de 2025».

El emisor va en el `title`: «Emitido por AC FNMT Usuarios», y con varios
almacenes, « · el mismo certificado en Firefox y Chrome»; en la fila que no se
puede usar, el `title` es el motivo. El elegido lleva fondo `--rf-surface` y ✓.

- **Una fila por certificado.** El mismo certificado —el mismo número de serie—
  en varios almacenes es **una** fila con todas sus etiquetas.
- **El DNI en claro**: la lista no sale del ordenador. La máscara es solo de la
  firma visible.
- **Agrupada**: «Disponibles» y «No se pueden usar», y dentro de cada grupo
  orden alfabético por la primera línea con `localeCompare("es")`.
- **Un certificado caducado o revocado se lista, dice por qué y no se deja
  elegir**: el nombre en `--rf-text-muted`, sin cursor, con su icono y su
  motivo. Esconderlo dejaría a quien viene a firmar con él mirando una lista
  donde falta. Hoy no se comprueba la revocación, solo las fechas.
- **El buscador** filtra por titular, entidad, NIF o DNI, emisor y almacén.
  Con texto, «N de M» encabeza la lista; sin coincidencias, «Ningún certificado
  coincide». `Escape` cierra y vacía la búsqueda, y elegir también la vacía.
- **Se recuerda al firmar con él**, no al elegirlo, y la próxima sesión sale ya
  puesto ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).
- La lista se cierra al elegir, al pulsar fuera y con `Escape`.

**La primera vez no hay certificado elegido.** Sin certificado recordado, la
caja dice «Elige un certificado» y «Firmar» está atenuado. No se preselecciona
ninguno, ni siquiera cuando hay uno solo —la identidad con que se firma no la
elige la aplicación—, y nunca uno que no sirva. Mientras tanto, el interruptor
de «Firma visible» está apagado y desactivado, y debajo dice «Elige un
certificado para añadir una firma visible.».

## Estados

En el artboard `Main`, palanca «Estado», más «Firma visible», «Lista de
certificados», «Firmas previas» y «Pie · destino»:

- **Sin certificado elegido**: la caja dice «Elige un certificado» y «Firmar»
  está al 55 %; «Firma visible» apagada y desactivada, con el aviso debajo.
- **Buscando certificados**: la caja dice «Buscando certificados…» con un
  indicador, y «Firmar» está al 55 %. El resto sigue editable, salvo «Firma
  visible», desactivada y con el aviso debajo.
- **Sin certificados**: arriba de la zona que se desliza, el triángulo, «Sin
  certificados» y «No hay ningún certificado con el que firmar.». En el pie, en
  la fila de 44 px, «Añadir un certificado…» (primario, lleva a los certificados
  en fichero de [Preferencias](preferencias.md)) y «Volver a buscar». No hay
  selector.
- **Listo**: la caja con el certificado elegido y «Firmar». Con alguna firma
  previa caducada o no válida, o un hallazgo, lo mismo; pulsarlo abre [«¿Firmar de todos modos?»](dialogo-firmar-de-todos-modos.md).
- **Certificados abiertos**: el buscador en lugar de la caja y la lista hacia
  abajo, sobre el resto del panel. La palanca «Lista de certificados» cambia
  cuántos hay, si se listan los que no se pueden usar y si se agrupan.
- **Rango con error**: ver arriba.
- **Firmando**: el selector, interruptor, controles y `Cambiar` al 35 %; el
  botón al 55 % dice «Firmando…». Encima, el
  [diálogo de progreso](dialogo-progreso-firma.md).
- **Firmado**, y los cinco «verify · …» (PDF con firmas, CAdES con
  contrafirmas, sin firmas, formato desconocido, fallo al leer las firmas): ver
  «El resumen».
- **Error al firmar**: la zona que se desliza se sustituye por una tarjeta con
  borde de 2 px en `--rf-border-strong`: triángulo y «No se ha podido firmar», la
  causa en llano —«El certificado ha dejado de estar disponible mientras se
  firmaba.»—, «El documento sigue como estaba: no se ha guardado nada.», el
  detalle técnico en monoespaciada (`CKR_DEVICE_REMOVED durante C_Sign (fase:
  firma)`) y «Copiar detalle». En el pie, «Reintentar» (primario) y «Volver». En
  la aplicación el detalle va plegado tras «Detalle técnico». **Excepción**: si
  la causa es que el llavero perdió el PIN del Almacén de rFirma, junto a
  «Copiar detalle» aparece también «Vaciar el almacén», con su misma
  confirmación (ADR-0034).

## El resumen: tras firmar y con `verify --gui`

Un solo resumen, y se ve igual llegues como llegues: después de firmar, o
abierto por `rfirma verify -i <doc> -gui`. Lista **todas las firmas del
documento tal como ha quedado**, con lo mismo que `rfirma verify -v` y nada más.
Sustituye a la zona que se desliza: no hay selector, ni aviso de firmas
previas, ni firma visible, ni modelo.

De arriba abajo:

- **Solo tras firmar**, una franja: fondo `--rf-surface`, borde de 1 px en
  `--rf-border-subtle`, `--rf-radius-md`, el círculo con la ✓ y «**Firmado a
  las 11:04**» a 14 px en negrita. Con `verify` no hay franja.
- «**Firmas del documento**», con el icono de documento, a 14 px en negrita.
- Dos insignias `.rf-badge`: el formato (**PAdES**, **CAdES**, **XAdES**) y el
  recuento («1 firma», «3 firmas», «2 firmas · 2 contrafirmas»).
- **Los hallazgos del documento**, si los hay, uno por línea y encima de las
  fichas, como en «La validez de cada firma». **Tras firmar, «Se ha modificado
  después de la última firma» se lee «Se modificó antes de tu firma»**: la
  última firma es ya la tuya, y el cambio es anterior a ella. Los demás
  hallazgos se leen igual.
- **Una ficha por firma**, `.rf-card` de 10 × 12 px de relleno, apiladas a 6 px
  y todo a la vista; las mismas del diálogo [«Ver firmas»](dialogo-ver-firmas.md).
  Arriba, «FIRMA 1» en `.rf-label` versalita; tras firmar, la tuya va la última
  y lleva la insignia **Nueva** (`.rf-badge--primary`); a la derecha, la
  **validez** con su icono. Debajo, una fila por dato: el rótulo en
  `--rf-text-muted`, a 104 px fijos, y el valor a 13 px:
  - **Firmante**, en seminegrita: el nombre y el NIF entre paréntesis; en un
    sello, la razón social y el identificador de organización
    (`EMPRESA FICTICIA SL (VATES-B00000000)`).
  - **En nombre de**, solo con certificado de representación: la entidad y su
    CIF.
  - **Emisor**.
  - **Fecha** («3 oct 2026, 11:04»), la que declara quien firma; o
    **Sellada** («10 ene 2023, 10:32 · TSA FNMT»), con el día, la hora y la
    TSA, cuando la firma lleva sello de tiempo.
  - **«No admite más firmas»**, sin rótulo y a todo lo ancho, en la firma que
    cierra el documento.
  - **Motivo**, la última fila, solo si no es válida.

  **Un campo ausente no se pinta**: la ficha tiene una fila menos.
- **Las contrafirmas van dentro de la firma que contrafirman** (CAdES y
  XAdES): «CONTRAFIRMA 1.1» dentro de «FIRMA 1», con sus mismas filas y su
  propia validez, tras un filete de 2 px en `--rf-border-strong` a la izquierda
  y 12 px de sangría. En PDF no hay árbol.

**Sin algoritmo ni número de serie**: son de `verify -vv`, y la interfaz enseña
lo de `verify -v`, que ahora incluye la validez.

**Tu ficha tiene las mismas filas que las demás.** Lo propio de la firma recién
hecha —la firma visible, sus páginas, el recuadro— no sale en el resumen; la
firma estampada se sigue viendo en la hoja del visor, porque es el documento.

Con `verify`, cuando no hay lista:

- **Sin firmas**: el icono de documento, «**Sin firmas**» y debajo, en
  `--rf-text-muted` con 26 px de sangría, «El documento no tiene firmas.». El
  mismo patrón que «Sin certificados».
- **Formato desconocido**: el triángulo, «**Formato no reconocido**» y «No es
  un PDF ni una firma CAdES o XAdES.».
- **Fallo al leer las firmas**: la tarjeta del error de firma —borde de 2 px en
  `--rf-border-strong`—, el triángulo y «**No se han podido leer las firmas**»,
  el detalle técnico en monoespaciada (lo que la terminal escribe en stderr) y
  «Copiar detalle». Sin lista.

**El pie, rotulado siempre «Documento»**, con la caja de la carpeta y el
nombre: tras firmar, el fichero firmado y `Cambiar`, que mueve el destino
(ADR-0011); con `verify`, el fichero abierto y `Cambiar` oculto sin mover nada.
La fila de 44 px es la misma en los dos: «Abrir el PDF» (primario; «Abrir el
fichero» si no es PDF), la carpeta (secundario, 44 px, `title` «Abrir la
carpeta») y «**Firmar**» (`--ghost`). Los dos primeros usan el portal `OpenURI`:
bajo el sandbox son la única forma de llegar al fichero sin saberse la ruta.

**«Firmar» vuelve al panel con el documento releído del disco**, como
`rfirma doc.pdf`. Si el fichero no es PDF, al 55 % y con el `title` «En el
escritorio solo se firman PDF». El acuse es de un documento concreto: al cambiar
de pestaña o cerrarla, se va.

## Componentes y tokens

`.rf-btn--primary|--secondary|--ghost`, `.rf-badge`, `.rf-badge--primary`,
`.rf-card`, `.rf-label`, `.rf-input`, `.rf-body`, `.rf-prose`,
`--rf-surface`, `--rf-bg`, `--rf-primary`, `--rf-on-primary`,
`--rf-border-strong`, `--rf-border-subtle`, `--rf-text-muted`,
`--rf-radius-sm|md|lg`, `--rf-shadow-elevated`. El interruptor es el de
[design-system.md](design-system.md).

## Decisiones

- **El selector de certificado se separa del botón de firmar** y sube al
  principio del panel; el botón queda en «Firmar» a secas. Es el mismo
  mecanismo que ya tenía la [ventana de sede](ventana-de-sede.md), donde
  selector y botón iban separados, y así las dos pantallas quedan iguales. Se
  descartó el botón partido «Firmar como <nombre> ▾», que venía de V3 A: no
  estaba claro cómo cambiar de certificado, y quien pulsaba «Firmar como…» en
  lugar de la flecha firmaba con un certificado que no quería. Separados, no se
  firma sin querer. Con él desaparece también «Elegir certificado ▾».
- **La lista se abre hacia abajo desde arriba del todo.** En Tauri un
  desplegable no puede salir de la ventana, y pegado arriba es donde tiene el
  espacio. Sigue flotando: la firma visible y el botón no se mueven al abrirla.
- **El selector funciona como el de antes**: se recuerda el último certificado
  con el que se firmó y la primera vez no hay preselección. Cambian el aspecto,
  los datos —la entidad primero en los de representante, las etiquetas de
  almacén, la caducidad— y la agrupación, que marca el que no se puede usar con
  su icono y el nombre en gris en lugar de atenuar la fila al 45 %.
- **Una fila por certificado, con etiquetas de almacén.** El mismo certificado
  en varios almacenes salía en filas duplicadas que no había forma de
  distinguir.
- **El buscador es lo único nuevo**: gestorías y representantes manejan muchos
  certificados.
- **El DNI va en claro** en el selector, sin la máscara de la firma visible: el
  listado no sale del ordenador y el número ya se ve en los almacenes del
  sistema y de los navegadores; el texto de la firma visible, en cambio, viaja
  dentro del documento.
- **Sin cabecera de documento**: nombre en la pestaña, páginas en el visor. Su
  «27 páginas · 2,4 MB» no aportaba nada que no se viera.
- **Modelos en lugar de casillas por dato.** La v0.3.1 tenía cinco casillas
  —Firmante, Emisor, Fecha, Rúbrica, Motivo— que componían un párrafo. Tres
  tarjetas con la firma real en miniatura dicen el resultado sin leer; el caso de
  quien quiere otra cosa lo cubre Personalizada. Se descartaron también un
  modelo «Breve» (lo hace Personalizada) y la frase editable como alternativa
  suelta (solo vive dentro de Personalizada).
- **Sin comodines ni motivo.** AutoFirma compone el texto con comodines entre
  `$$` que se escriben a mano y no se ven resueltos hasta firmar. Las pastillas
  son esos datos, pero se insertan desde un menú, no se escriben mal y se ven ya
  sustituidos. El motivo era un segundo campo de texto libre junto a una frase
  que ya lo es.
- **La rúbrica, una fila siempre visible.** Se descartó añadirla y quitarla desde
  un hueco en el propio recuadro del visor, con «+», ✕ y «Cambiar» al pasar el
  ratón: escondía la opción dentro del objeto que se arrastra. Y se descartó la
  casilla «Rúbrica» con un bloque «Imagen de la rúbrica» aparte: dos sitios para
  una decisión.
- **Firma visible apagada por defecto y encendida = colocada.** «Colocado» dejó
  de ser una bandera: con recuadro y sin páginas era un estado que nadie sabía
  dibujar.
- **La elección de páginas vive en el panel**, no en miniaturas: una tira se
  rompe a las 200 páginas.
- **El error de firma sustituye al panel**, como el resumen, en lugar de meter la
  tarjeta en el pie: el pie tiene 162 px fijos en todos los estados.
- **El pie enseña carpeta y nombre**, en una caja, bajo un solo `Cambiar` (de
  V3 B): el nombre lo elige la aplicación y hasta firmar no se veía.
- **«Firmar otro documento» no existe**: «Abrir PDF…» de la cabecera ya abre.
- **El aviso de firmas previas dice la validez, no solo el número.** El de antes,
  «Ya lleva 1 firma · Ver», con el número llegando como desconocido, no se
  montaba nunca y no decía si las firmas servían.
- **El aviso es una línea y el detalle va a un diálogo.** Se descartó el aviso
  desplegable, con una fila por firma, su veredicto y su motivo: en 380 px
  empujaba la firma visible fuera de la vista. «Ver firmas →» abre el diálogo,
  el mismo desde el panel y desde la sede.
- **«· N caducadas» solo si todos los problemas son caducadas**; si no,
  «· N problemas», con los hallazgos dentro. Qué es cada uno lo dice el
  diálogo.
- **Tres valideces, y la misma en todas partes** (ADR-0043). Se descartó el
  cuarto valor «no se ha podido comprobar del todo», que describía una
  limitación del validador y no algo de la firma.
- **El hallazgo del documento va aparte**, en su línea, y no se cuelga de la
  última firma: la pintaría no válida sin serlo. Antes era «El documento ha
  cambiado después de esta firma» dentro de la fila de la última.
- **Tras firmar, el hallazgo se lee desde tu firma.** «Se ha modificado después
  de la última firma» pasa a «Se modificó antes de tu firma» en el resumen: la
  última es la tuya, y el cambio se refiere a las firmas previas.
- **Monocromo.** Ni ámbar para la caducada ni rojo para la no válida: la paleta
  no tiene token de color para eso y los colores literales no se admiten
  ([design-system.md](design-system.md#2-color)). La ✓ en gris y a peso normal;
  ⚠ y ✗ en `--rf-text` a 700, y el borde de 2 px del aviso con problemas.
- **«Fecha» o «Sellada»**, no «Fecha declarada»: el sello de una TSA es lo
  único que fecha una firma, y cuando lo hay se dice con su nombre.
- **«Ya lo firmaste tú» en una franja al pie del aviso**, no como marca «Tú» en
  la fila.
- **El botón del pie no cambia con problemas.** La confirmación la hace
  [«¿Firmar de todos modos?»](dialogo-firmar-de-todos-modos.md), que abre
  cualquier ⚠ o ✗; el aviso no lleva más acción que «Ver firmas →».
- **Un solo resumen para después de firmar y para `verify --gui`.** `verify
  -gui` abría el escritorio con el fichero cargado para firmarlo, como
  `rfirma doc.pdf`; ahora abre el resumen. Se descartó una pantalla propia de
  verificación: dos resúmenes del mismo documento que dijeran cosas distintas.
  Por eso el resumen de después de firmar dejó de ser un acuse de tu firma
  —«RESUMEN», la insignia PAdES y la línea «Firma visible: En la página 6»— y
  pasó a listar todas las firmas, con la tuya marcada «Nueva». Con él se
  retiró la palanca «Ficha 14 (v1.0)», cuya lista de tarjetas con quién y
  cuándo es esto mismo, y «Volver a firmar» pasó a «Firmar».
- **Fichas apiladas, todo a la vista.** Se descartaron las fichas plegables
  (cada firma en una línea, el resto al desplegar) y la lista compacta con la
  ficha de la elegida debajo: con las una a cuatro firmas de un documento
  normal, un clic por firma para leer lo que `verify -v` da sin pedirlo.
- **El pie conserva «Firmar».** Se descartaron el pie sin «Firmar» (solo abrir
  y carpeta) y quitar el pie entero: tras ver las firmas, lo siguiente suele
  ser firmar, y el resumen no debe ser un callejón.
- **Sin número de serie**, aunque se dibujó en la primera tanda: es de
  `verify -vv`, y la interfaz enseña lo de `verify -v`.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`, el 25/09/2026. El aviso de
firmas previas y el paso a «¿Firmar de todos modos?», el 26/09/2026. El
selector de certificado, el 27/09/2026. El resumen unificado con `verify
--gui` y la validez de las firmas —el aviso compacto, el diálogo «Ver firmas» y
la validez en las fichas—, el 03/10/2026 (anotación `nota-validez`).
