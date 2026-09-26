# Panel de firma

Columna derecha de 380 px. Reúne lo que hay que decidir antes de firmar —la firma
visible y dónde se guarda— y termina en el botón que firma, que es también donde
se elige el certificado.

## Casos de uso que la usan

- Firmar un PDF en local — desde que hay un documento abierto hasta «firmado».

## Estructura

**Zona que se desliza** arriba y **pie fijo** abajo.

Zona que se desliza, de arriba abajo, cada bloque solo cuando toca:

1. **Aviso de cofirma**, si el PDF ya trae firmas.
2. **«Sin certificados»**, **«Firmado a las 11:04» y el resumen**, o **el error
   de firma**, según el estado.
3. **Firma visible**: rótulo e interruptor; encendida, el segmentado de páginas
   y su línea o campo.
4. **Modelo**, **Con rúbrica** y, con Personalizada, **la frase**.

Pie fijo:

1. **«Guardar en»** con `Cambiar`, y una caja con la carpeta y el nombre del
   fichero.
2. **El botón partido**: «Firmar como <nombre y primer apellido>» y ▾, que abre
   la lista de certificados hacia arriba.

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
- **Rótulos** («Firma visible», «Modelo», «Guardar en»): `.rf-label`.
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
- **Botón partido**: fila de 44 px. Izquierda `.rf-btn--primary` a todo lo que
  sobra, radio solo a la izquierda, «Firmar como» a peso 400 y el nombre con
  elipsis (entero en el `title`). Derecha, 44 px, separada por 1 px en
  `--rf-on-primary`, con el chevron de 14 px: abajo cerrado, arriba abierto.
- **Lista de certificados**: flota sobre el pie a 24 px de cada lado y con su
  borde inferior 68 px por encima del fondo del panel, `z-index: 6`, relleno
  6 px, `--rf-radius-lg`, borde `--rf-border-subtle`, `--rf-bg`,
  `--rf-shadow-elevated`. Crece hacia arriba hasta 8 px por debajo del borde del
  panel y a partir de ahí se desplaza.

## El aviso de cofirma

Icono de información y borde `--rf-border-subtle`: «Ya lleva **1 firma** ·
Ver». Con varias, desplegado con quién y cuándo y «Ocultar». No es una alarma:
añadir una firma sin invalidar la anterior es lo normal.

**Lo que no se sabe no ocupa sitio** (ID-44). Hoy el número de firmas llega como
desconocido, y entonces el aviso no se monta: ni un guion ni un marcador. Vuelve
en cuanto alguien las cuente, sin rediseño.

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

**Se elige en el botón de firmar.** «Firmar como Lucía Martín» dice con qué se
firma; el ▾ abre la lista hacia arriba, sobre el pie, porque en Tauri la ventana
corta por abajo.

- **Cada fila**: el titular a 14 px y peso 600; debajo, emisor · almacén ·
  caducidad en `.rf-body rf-text-muted`. El elegido, con fondo `--rf-surface` y
  ✓. El almacén va por nombre —«Firefox», «Chrome», «Instalado en rFirma»—,
  nunca por ruta: el mismo certificado en dos almacenes es indistinguible sin él.
- **Agrupada**: «Disponibles» y «No utilizables», y dentro de cada grupo orden
  alfabético por titular con `localeCompare("es")`, desempatando por almacén. El
  orden en que responden los módulos no significa nada para quien elige.
- **Un certificado caducado o revocado se lista, dice por qué y no se deja
  elegir**: al 45 %, sin cursor, con el motivo en tercera línea y en negrita
  —«Caducó el 3 de marzo de 2025»—. Esconderlo dejaría a quien viene a firmar
  con él mirando una lista donde falta. Hoy no se comprueba la revocación, solo
  las fechas.
- **Se recuerda al firmar con él**, no al elegirlo, y la próxima sesión sale ya
  puesto ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).
- La lista se cierra al elegir, al pulsar fuera y con `Escape`.

**La primera vez no hay certificado elegido.** Sin certificado recordado, el
botón dice «Elegir certificado ▾» y es uno solo: el botón entero abre la lista
hacia arriba. Al elegir uno pasa a «Firmar como <nombre> ▾». No se preselecciona
ninguno, ni siquiera cuando hay uno solo —la identidad con que se firma no la
elige la aplicación—, y nunca uno que no sirva. Mientras tanto, el interruptor
de «Firma visible» está apagado y desactivado, y debajo dice «Elige un
certificado para añadir una firma visible.».

## Estados

En el artboard `Main`, palanca «Estado», más «Firma visible», «Lista de
certificados», «Pie · destino» y «Ficha 14»:

- **Sin certificado elegido**: «Elegir certificado ▾» en un solo botón, que abre
  la lista; «Firma visible» apagada y desactivada, con el aviso debajo.
- **Buscando certificados**: el botón dice «Buscando certificados…» con un
  indicador, sin nombre, al 55 % y con el ▾ inerte. El resto sigue editable,
  salvo «Firma visible», desactivada y con el aviso debajo.
- **Sin certificados**: arriba de la zona que se desliza, el triángulo, «Sin
  certificados» y «No hay ningún certificado con el que firmar.». En el pie, en
  la fila de 44 px, «Añadir un certificado…» (primario, lleva a los certificados
  en fichero de [Preferencias](preferencias.md)) y «Volver a buscar».
- **Listo**: el botón «Firmar como …».
- **Certificados abiertos**: la lista sobre el pie, el chevron hacia arriba.
- **Rango con error**: ver arriba.
- **Firmando**: interruptor, controles y `Cambiar` al 35 %; el botón al 55 %
  dice «Firmando como …». Encima, el
  [diálogo de progreso](dialogo-progreso-firma.md).
- **Firmado**: ver «El resumen».
- **Error al firmar**: la zona que se desliza se sustituye por una tarjeta con
  borde de 2 px en `--rf-border-strong`: triángulo y «No se ha podido firmar», la
  causa en llano —«El certificado ha dejado de estar disponible mientras se
  firmaba.»—, «El documento sigue como estaba: no se ha guardado nada.», el
  detalle técnico en monoespaciada (`CKR_DEVICE_REMOVED durante C_Sign (fase:
  firma)`) y «Copiar detalle». En el pie, «Reintentar» (primario) y «Volver». En
  la aplicación el detalle va plegado tras «Detalle técnico».

## El resumen, tras firmar

Arriba de la zona que se desliza:

- «**Firmado a las 11:04**» con el círculo y la ✓.
- **RESUMEN** en versalitas y la insignia **PAdES**. El rótulo se queda aunque
  cuelgue una sola insignia: guarda el sitio de la ficha 14.
- **Con la ficha 14 (v1.0)**: la insignia «N firmas» y una tarjeta por firma, la
  tuya primero con `La tuya` en `.rf-badge--primary` y «***9999** · hoy, 11:04»,
  después las previas con quién y cuándo.
- «Firma visible» pasa a una línea de solo lectura: «No», «En la página 6», «En
  todas las páginas» o «En N de M páginas». El aviso de cofirma desaparece.

En el pie, «Guardado en», `Cambiar` oculto sin mover nada, y en la fila de 44 px:
«Abrir el PDF» (primario), la carpeta (secundario, 44 px, `title` «Abrir la
carpeta») y «Volver a firmar» (`--ghost`). Los dos primeros usan el portal
`OpenURI`: bajo el sandbox son la única forma de llegar al fichero sin saberse la
ruta.

**«Volver a firmar» vuelve al panel con el original releído del disco.** El
acuse es de un documento concreto: al cambiar de pestaña o cerrarla, se va.

## Componentes y tokens

`.rf-btn--primary|--secondary|--ghost`, `.rf-badge`, `.rf-badge--primary`,
`.rf-card`, `.rf-label`, `.rf-input`, `.rf-body`, `.rf-prose`,
`--rf-surface`, `--rf-bg`, `--rf-primary`, `--rf-on-primary`,
`--rf-border-strong`, `--rf-border-subtle`, `--rf-text-muted`,
`--rf-radius-sm|md|lg`, `--rf-shadow-elevated`. El interruptor es el de
[design-system.md](design-system.md).

## Decisiones

- **El certificado va en el botón**, no en una fila propia del panel (V3 A).
  Dice con qué se firma justo donde se firma, y libera el alto de la fila. El
  recibo de V3 B lo repetía en un renglón «Firmar como»: se quitó.
- **La lista abre hacia arriba y flota**: la firma visible y el botón no se
  mueven al abrirla.
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
- **«Firmar otro documento» no existe**: el «+» de la tira ya abre.

Validado en el lienzo
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **Recorrido de firma**, artboard `Main`, el 25/09/2026.
