# Panel de firma

Columna derecha de 360 px. Reúne todo lo que hay que decidir antes de firmar y
termina en el botón que firma. Es la única región con un botón primario.

## Casos de uso que la usan

- Firmar un PDF en local — de «documento cargado» a «firmado».

## Estructura

Área desplazable arriba, pie fijo abajo.

**Desplazable**, de arriba abajo:

1. **Documento**: icono, nombre y «27 páginas · 2,4 MB». Sin botón de cambiar:
   para eso está la [bandeja](bandeja-de-documentos.md).
2. **Aviso de cofirma**, solo si el PDF ya trae firmas.
3. **Certificado**.
4. **Firma visible**.

**Pie fijo**: «Se guardará en» con **la última carpeta y el nombre del
fichero** —nunca la ruta—, un `Cambiar`, y debajo el botón primario a ancho
completo. El destino se ve antes de firmar, no después. Lo fija el
[ADR-0011](../adr/0011-destino-del-documento-firmado.md); la línea es el
componente **ruta de destino** del [sistema de diseño](design-system.md).

### Geometría

- La columna es de 360 px con 16 px de relleno y **11 px** entre bloques. Los
  11 px no salen de la escala de espaciado a propósito: es la columna más
  apretada de la ventana y los 8 px de `xs` juntan demasiado los divisores.
- **Cabecera**: icono de hoja de 20 px en `--rf-text-muted`, desplazado 2 px
  hacia abajo para alinear con la primera línea; a su lado, el nombre en
  `.rf-title` **a 14 px** recortado con elipsis y, 4 px debajo, la línea de
  metadatos en `.rf-body rf-text-muted`.
- **Rótulos de sección** («CERTIFICADO», «FIRMA VISIBLE»): `.rf-label` en
  versalitas con `letter-spacing: .6px`. Los rótulos de campo («Contenido»,
  «Se guardará en») son `.rf-label` a secas, sin versalitas.
- **Aviso de cofirma**: 10 px de relleno vertical y 16 px horizontal, borde de
  1 px en `--rf-border-subtle`, `--rf-radius-md`, fondo `--rf-bg`, icono de
  información de 20 px.
- **Disparador del certificado**: 16 px de relleno, 6 px entre líneas, borde
  `--rf-border-strong`. Escarapela de 20 px y nombre en `.rf-title` a 14 px;
  debajo, DNI y emisora en una sola línea `.rf-body rf-text-muted` separadas
  por `·`; a la derecha, la punta de flecha de 16 px, girada 180° al abrir.
- **Filas de la lista**: 12 px de relleno vertical y 16 px horizontal, 4 px
  entre líneas, separadas por 1 px en `--rf-border-subtle` salvo la última.
  La elegida lleva la marca de verificación de 16 px en `--rf-primary`, a la
  derecha y a 3 px del borde superior.
- **Esqueletos de carga**: dos bloques de **66 px** —el alto de la tarjeta que
  van a sustituir, para que el panel no salte—, con `--rf-radius-md` y borde
  `--rf-border-subtle`. El primero lleva el degradado horizontal que dice que
  la búsqueda sigue viva; el segundo, solo su borde.
- **Casillas del contenido**: cuadrado de 18 px con `--rf-radius-sm`, contorno
  `--rf-border-strong` cuando está vacío y relleno `--rf-primary` con la marca
  de verificación de 12 px (trazo 3) cuando está marcado. Fila de 24 px de alto
  mínimo, con 8 px entre la casilla y su texto. La ayuda de la casilla —cuando
  la lleva, como «Elige antes una imagen»— va **2 px debajo del rótulo y en su
  misma columna**: 26 px de sangría, los 18 del cuadrado más los 8 que lo
  separan del texto. Es hermana del `<label>`, no va dentro, para que no entre
  en el nombre accesible de la casilla; la sangría es lo que la deja donde el
  artboard la dibuja.
- **Interruptor**: pastilla de 40×24 px con el pomo de 16 px. Va **delante**
  del texto, **a 8 px** de él. Es el mismo componente que el de Preferencias,
  pero no la misma separación: allí el artboard lo dibuja con `rf-gap-sm`
  (16 px) y aquí con `rf-gap-xs`. Un solo valor no puede ser los dos, así que
  el diálogo pide el suyo (`switch--wide`) y este es el de por omisión.
- **Rúbrica**: la miniatura es un rectángulo de 56×36 px con borde
  `--rf-border-strong` y `--rf-radius-sm`, y comparte fila con el botón
  secundario, que ocupa el resto y mide 36 px de alto.
- **Pie**: 16 px de relleno, borde superior de 1 px en `--rf-border-subtle`.
  Dentro: el rótulo «Se guardará en», 4 px debajo la fila con el icono de
  carpeta de 20 px, el destino en `.rf-prose` y un `Cambiar` fantasma de 32 px
  de alto, 8 px de relleno lateral y 12 px de cuerpo. Debajo, el botón primario
  a ancho completo. El icono y el `Cambiar` se alinean **arriba**
  (`align-items: flex-start`, el icono 3 px más abajo para casar con la primera
  línea) porque el destino puede ocupar dos renglones.
- **Aviso de error**: 16 px de relleno, borde de **2 px** en
  `--rf-border-strong` y `--rf-radius-md`; título con el triángulo de aviso de
  20 px y el texto en `.rf-title` a 14 px.

## El aviso de cofirma

Icono de **información**, borde `--rf-border-subtle`:
«Ya lleva **1 firma**: la tuya será una **cofirma**».

No es una alarma —añadir una firma sin invalidar la anterior es lo normal—, así
que no lleva icono de aviso.

A la derecha, un **`Ver ›`** que despliega las firmas que el documento ya
tiene, con quién y cuándo, usando el mismo componente de firma que el resumen
del estado firmado. Es lo que permite auditar un PDF ajeno antes de añadirle
nada.

## Lo que no se sabe no ocupa sitio

El artboard enseña «27 páginas · 2,4 MB», «Ya lleva **1 firma**: la tuya será
una **cofirma**» y, en el resumen, «2 firmas». Hoy **el tamaño y el número de
firmas llegan como desconocidos**, y averiguar si un PDF ya viene firmado está
fuera del alcance de la entrada de documentos.

La regla, decidida al transcribir el canvas (ID-44):

> Lo que se sabe se pinta; **lo desconocido no ocupa sitio**. Ni un guion, ni
> un «—», ni un marcador de posición, ni el separador que lo precedería.

Así, con el tamaño desconocido la línea de metadatos dice exactamente
«27 páginas» y termina ahí; con el número de firmas desconocido, el aviso de
cofirma y el `Ver ›` **no se montan**. Un «—» donde debería ir un dato no dice
«todavía no se sabe»: dice «este documento no tiene tamaño», que es falso, y
deja al lector decidiendo cuál de las dos cosas significa.

**Qué reaparece cuando el dato exista**: en cuanto la entrada de documentos
devuelva el tamaño, vuelve el `· 2,4 MB` de la línea de metadatos; en cuanto
alguien cuente las firmas del PDF, vuelven el aviso de cofirma entero con su
`Ver ›` y la insignia de firmas del resumen. Ninguno de los dos necesita
rediseño: el sitio ya está descrito arriba, solo está vacío.

El canvas es anterior a esa constatación y por eso los pinta con datos de
ejemplo.

## Certificado

**Un desplegable**, no una tarjeta. Cerrado ocupa el mismo hueco que ocupaba la
tarjeta y enseña lo mismo —escarapela, titular, y debajo `DNI · Emitido por X`—
más la punta de flecha a la derecha. Abierto despliega la lista de certificados
**superpuesta** sobre el panel.

Superpuesta y no en flujo: la firma visible y el botón de firmar **no se
mueven** al abrirla, y con nueve certificados el panel sigue midiendo lo mismo.
Un acordeón que empuja el contenido saca el botón primario de la vista justo
mientras se elige. La capa va a `calc(100% + 4px)` del disparador, con
`--rf-shadow-elevated` y la lista recortada a **232 px** con desplazamiento
propio, que son tres filas y media: el borde cortado es lo que dice que hay más.

**Cada fila lleva el almacén.** Titular en `.rf-title` a 14 px y, debajo,
`DNI · emisor · almacén` en `.rf-body rf-text-muted`. El almacén no es adorno:
el mismo certificado en el perfil de Firefox y en `~/.pki/nssdb` es
indistinguible sin él, y quien tiene tres iguales no puede elegir a ciegas. Va
por **nombre** —«Firefox», «Chrome», «Tarjeta»—, nunca por la ruta del módulo ni
por el `configdir` del perfil, que son rutas del anfitrión
([ADR-0011](../adr/0011-destino-del-documento-firmado.md)). En el disparador
cerrado **no aparece**: elegido ya no desambigua nada.

**Un certificado caducado o revocado se lista, dice por qué, y no se deja
elegir.** Fila `disabled`, atenuada, con el motivo en tercera línea y en negrita
—«Caducó el 3 de marzo de 2020»—, con la misma prosa que ya compone
`statusWarning`. Esconderlo sería más limpio y peor: quien viene a firmar con
ese certificado se quedaría mirando una lista donde falta, sin saber por qué.
Es lo que pide la historia 8 del [#46](https://github.com/sgomez/rfirma/issues/46).

**La primera vez no hay preselección.** Con varios certificados y nada
recordado, el disparador dice «Elegir certificado» y el botón de firmar está
apagado. Elegir con qué identidad se firma un documento con validez jurídica no
lo hace la aplicación por su cuenta, y el orden de la lista solo dice en qué
orden cargaron los módulos. Con uno solo sí se elige solo: elegir entre una cosa
no es elegir.

**El certificado se recuerda al firmar con él**, no al elegirlo en la lista, y
la próxima sesión sale ya puesto. El glosario dice «el certificado usado la
última vez» y la historia 7 dice «con cuál firmé»; ninguna dice «el último que
miré». Si al arrancar ya no está, el panel vuelve a «Sin certificado» sin ruido
([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).

El artboard es
[`EstadoElegirCertificado`](artboards/EstadoElegirCertificado.dc.html), y es el
único que se puede pulsar.

## Firma visible

Cuatro piezas en este orden:

1. **Interruptor**: «Estampar un recuadro de firma en el documento».
2. **Colocación**: en qué páginas se sella y en cuál está el recuadro.
   Ver «[Colocación](#colocación)».
3. **Contenido**: cinco casillas de igual forma y ritmo.
   - Tu rúbrica
   - Nombre y apellidos
   - DNI
   - Fecha y hora de la firma
   - Un motivo
4. **Imagen de la rúbrica**: miniatura real de la imagen cargada y un botón
   para cambiarla.

**El DNI se estampa enmascarado**, siempre y sin interruptor: `99999999R` sale
como `***9999**`, con la misma máscara que AutoFirma aplica por omisión
(`*`, mínimo tres dígitos seguidos, tres ocultos y cuatro visibles). No se
promete más de lo que hace: el certificado entero viaja dentro de la firma con
el DNI en claro, y cualquier lector de PDF lo enseña al inspeccionarla. La
máscara protege de la lectura casual del recuadro, no del documento.

**Firma visible y rúbrica no son lo mismo**, y la estructura lo dice sin
explicarlo: la *firma visible* es el recuadro que se estampa en la página; la
*rúbrica* es la firma manuscrita escaneada que va dentro de él, y es opcional.
Ver [ADR-0006](../adr/0006-firma-visible-se-configura-sobre-el-documento.md) y
el glosario de [CONTEXT.md](../../CONTEXT.md).

**No hay comodines**, ni arriba ni abajo. El usuario nunca escribe
`$$SUBJECTCN$$` ni `$$SIGNDATE$$`: marca qué dato aparece. Y tampoco los hay por
debajo: rFirma compone el texto del recuadro y lo envía ya resuelto en
`layer2Text`. Lo fuerza esta lista de casillas — AutoFirma **no tiene comodín
para el DNI**, que vive en el RDN `serialNumber` y solo asoma dentro de
`$$SUBJECTCN$$` y `$$SUBJECTDN$$`, con el nombre pegado. Separar «Nombre y
apellidos» de «DNI» no se puede expresar con sus comodines. Ver
[#31](https://github.com/sgomez/rfirma/issues/31).

**Sin imagen cargada, la casilla «Tu rúbrica» está apagada** con la pista
«Elige antes una imagen»: no se puede marcar una rúbrica que no existe.

**La miniatura enseña el resultado real, sobre blanco.** La rúbrica viaja al PDF
como JPEG y el JPEG no tiene alfa, así que un PNG recortado sale con fondo
blanco. No se avisa con un cartel: se enseña. Lo que la miniatura muestra es el
fichero que se va a firmar, ya normalizado, no el que el usuario eligió.

**Al elegir la imagen** el selector del portal filtra por tipo MIME
(`image/png`, `image/jpeg`). Una imagen demasiado grande **se reduce en
silencio** —es lo que el usuario habría pedido— y solo hay tres fallos que se
cuentan: *no es una imagen PNG o JPEG*, *la imagen está dañada* y *la imagen es
demasiado grande*. Todo esto ocurre al elegir, con el diálogo aún abierto, nunca
al firmar. Ver el
[ADR-0012](../adr/0012-normalizacion-de-la-rubrica-en-rust.md).

## Colocación

Bloque con borde `--rf-border-strong`, `--rf-radius-md` y fondo `--rf-bg`, bajo
el interruptor de firma visible. Sustituye a la línea «Página 3 · arrástralo
para colocarlo», que **mentía**: decía la página que estabas mirando, no la del
recuadro.

Dentro, el rótulo `Colocación` y tres opciones en radio:

| Opción | Pie | Conjunto |
| --- | --- | --- |
| `Solo 1 página` | `Página 3`, o `aún sin colocar` | la página del recuadro |
| `Estas páginas` | — | lo que diga su campo de texto |
| `Todas las páginas (27)` | — | las *n* del documento |

**La etiqueta es fija y el número va en el pie.** «Esta página» no dice cuál y
deja de ser cierto en cuanto pasas de página; el pie sí lo dice, y dice además
de dónde sale: la página donde has puesto el recuadro. La fija el arrastre, no
lo que estés mirando ([#152](https://github.com/sgomez/rfirma/issues/152)).

Debajo de las tres, y solo con más de una página elegida, la frase de la
limitación: «El mismo recuadro, en el mismo sitio y del mismo tamaño, en las 13
páginas: es un solo campo de firma repetido, no 13 firmas». Es la traducción
honesta de lo que se estampa —un único campo de firma con el widget replicado,
medido en [recuadro-replicado-pdfsig.md](../research/recuadro-replicado-pdfsig.md)
y validado por VALIDe como PAdES B-Level con **un solo firmante**—, y por eso el
`/Rect` es forzosamente el mismo en todas.

### Colocado es tener páginas

**No hay un estado «colocado» aparte: el recuadro existe si hay al menos una
página sellada.** Con el conjunto vacío no se dibuja recuadro en ninguna parte,
el pie del panel pide colocar la firma y `Firmar documento` está **apagado**,
sea cual sea la opción marcada — también con `Todas las páginas`, porque elegir
todas no coloca nada. Quitar la última página del conjunto devuelve exactamente
a ese estado. No existe un estado intermedio con un recuadro que no esté en
ninguna página, y eso es lo que hace que «recién abierto» y «me he quedado sin
páginas» sean una sola cosa que describir, dibujar y probar.

### El campo de «Estas páginas»

Formato de impresión de toda la vida: `1,2-3,10-20`. Números y rangos separados
por comas, sin sintaxis propia.

**No es el rango tecleado que prohíben las notas del mapa.** Lo prohibido de
AutoFirma es su sintaxis —`1-3,-3--1`, con rangos negativos— y, sobre todo, su
degradación silenciosa. Aquí **nada se recorta ni se ignora** (ID-22): cada
entrada que no se puede resolver se dice y **apaga el botón de firmar**.

| Lo escrito | Lo que se ve |
| --- | --- |
| `10-40` en un documento de 27 | «El documento tiene 27 páginas y has escrito hasta la 40.» |
| `3-1` | «`3-1` va al revés: el primer número tiene que ser el menor.» |
| `0` | «No hay página 0: la primera es la 1.» |
| `1;2;3` | «`1;2;3` no se entiende. Números y rangos separados por comas: 1,2-3,10-20.» |
| vacío | Sin error, pero el conjunto queda vacío: se aplica «colocado es tener páginas». |

El campo lleva borde `--rf-text` en lugar de `--rf-border-strong` cuando hay
error, y el mensaje va debajo con el triángulo de aviso de 15 px.

**El campo se escribe solo.** Sellar o quitar una página desde el visor
reescribe su contenido en forma comprimida: quitar la 12 de `3,10-20` deja
`3,10-11,13-20`. Es el camino de vuelta del conjunto al texto, y hace que los
dos caminos —teclear y pulsar— no puedan discrepar.

### Tres caminos, un mismo resultado

Se coloca la firma de tres maneras, y ninguna es más oficial que otra:
arrastrando un recuadro sobre la hoja, pulsando el botón que hay bajo la página
(ver [visor de documento](visor-de-documento.md)), o escribiendo en este campo.

**Colocada por botón o por campo, el recuadro cae en su posición estándar**:
abajo a la derecha, a un 8 % del ancho y del alto desde el borde. Solo el
arrastre elige sitio; los otros dos caminos tienen que poner el recuadro en
algún lado, y ese es el sitio menos malo — es donde va una firma en un papel.

### Al cambiar de opción sobrevive la página del recuadro

De `Estas páginas` = {3, 10…20} a `Solo 1 página` queda **la 3**, la del gesto
original, no la que estés mirando ni la más baja del conjunto por casualidad. El
número del pie ya venía diciendo esa página, así que cambiar de opción no lo
mueve y no hay sorpresa. De `Solo 1 página` a `Estas páginas`, el campo arranca
con esa misma página escrita.

### Ni medidas escritas ni línea de ayuda

**No hay cuatro campos en puntos.** Se propusieron con la ficha 6 y se
descartaron: el recuadro se mueve arrastrando y cambia de tamaño por los
tiradores, y basta. Con ellos se cae también la pregunta de qué hacer con una
medida tecleada que se sale de la página — arrastrando no puede ocurrir, porque
soltar fuera no se acepta.

**Y no hay línea de ayuda.** «Arrástralo para moverlo y agárralo por una esquina
para cambiar su tamaño» son tres renglones en la columna más apretada de la
ventana para explicar dos gestos que se descubren al primer intento. Va en
**tooltip** sobre el recuadro.

## Estados

- **Sin certificado**: el desplegable cerrado y vacío, con «Elegir
  certificado» donde iría el titular; la sección de firma visible al 40 % y sin
  interacción; el botón primario deshabilitado.
- **Eligiendo**: la lista desplegada sobre el panel. Se cierra al elegir, al
  pulsar fuera y con `Escape`.
- **Cargando certificados**: «Buscando certificados…» y dos esqueletos.
- **Sin certificados**: bloque con borde `--rf-border-strong`, explicación
  («si usas una tarjeta, comprueba que está insertada y que el lector está
  conectado») y dos salidas: «Volver a buscar» y «Otro módulo…».
- **Sin colocar**: hay certificado y la firma visible está encendida, pero el
  conjunto de páginas está vacío. El pie añade, sobre el botón, «Coloca la firma
  sobre el documento: arrastra un recuadro o pulsa el botón que hay bajo la
  página», y `Firmar documento` está apagado. Es el estado del PDF recién
  abierto y también el de haber quitado la última página del conjunto.
- **Rango con error**: el campo de «Estas páginas» tiene algo que no se puede
  resolver. Mensaje bajo el campo y `Firmar documento` apagado.
- **Listo**: todo activo, botón «Firmar documento».
- **Destino no disponible**: la carpeta de destino no está o no se puede
  escribir. El pie sustituye «Se guardará en» por «No se puede escribir en
  *Documents*», con el `Cambiar` al lado. El botón de firmar **no se apaga** y
  no se degrada a otro destino: quien firme aquí elige dónde, y nadie se queda
  con el documento cargado y sin salida ([ADR-0011](../adr/0011-destino-del-documento-firmado.md)).
- **Error de firma**: el pie sustituye «Se guardará en» por un aviso con borde
  de 2 px, la causa en lenguaje llano y el detalle técnico en monoespaciada
  (`CKR_DEVICE_REMOVED durante C_Sign (fase: firma)`). El artboard lo dibuja
  **desplegado**; en la aplicación va plegado tras «Detalle técnico», porque un
  `CKR_*` crudo bajo el mensaje ocupa el pie entero y solo lo necesita quien va
  a escribir un informe de fallo. El «Copiar detalle» del artboard **está
  pendiente**: es una acción sobre el portapapeles, no una diferencia de piel.
  El botón pasa a «Volver a intentarlo».
- **Firmado**: el panel entero se reemplaza por el resumen (ver abajo). El
  acuse de recibo **es de un documento concreto y solo se ve con ese documento
  delante**. El recuento de páginas de la cabecera sale del PDF que la ventana
  tiene abierto, no del fichero escrito, así que con otro documento delante
  enseñaría el nombre de uno con las páginas de otro; y sin ninguno dejaría una
  tercera columna al lado del visor vacío, que es lo que quita el ID-51. Por
  eso el estado guarda el asa del documento que se firmó y se cierra solo en
  cuanto deja de estar activo: se elige otro en la bandeja, se olvida el
  activo, se vacía la lista.

## El resumen, tras firmar

Sustituye a la configuración, que ya no sirve de nada:

- **Cabecera**: el nombre del fichero resultante y, debajo, la línea de
  metadatos con las páginas y **el tamaño**. El tamaño lo conoce
  `finish_signing`, que escribe con `std::fs::write` y sabe cuántos bytes ha
  puesto; hoy `SignedDocumentView` lo descarta.
- **`Resumen`**, con la insignia `PAdES` —rFirma no produce otro formato— sola
  debajo. El encabezado se queda aunque solo cuelgue una insignia de él:
  **guarda el sitio de la ficha 14**, que traerá la insignia con el número de
  firmas y la tarjeta de cada firma del documento, con `La tuya` en la del
  usuario. Enseñarlas todas es la contrapartida del aviso de cofirma: si antes
  se avisa de que el PDF ya llevaba una, el resumen tiene que enseñarlas.
- **Tres botones, uno sobre otro y a ancho completo**, en el pie fijo:
  «Abrir el PDF» primario, «Abrir la carpeta» secundario y «Volver a firmar»
  como `--ghost`. Los dos primeros son el portal `OpenURI`, que funciona sin
  declarar ningún permiso, y cargan más peso del que parece: bajo el arenero
  son la única forma que tiene el usuario de llegar al fichero sin saberse la
  ruta.

**«Volver a firmar» vuelve al panel de firma con el original, releído del
disco.** Es abrir el documento otra vez: el usuario ha podido modificarlo fuera
o haberse equivocado al configurar la firma. El enlace del portal sigue siendo
válido mientras la ruta no cambie, que es la misma regla que sostiene la
insignia `No disponible` de los recientes. Si al releer el documento tiene
menos páginas y el recuadro recordado se sale, avisa el aviso de recuadro fuera
de página del visor (ID-22): no hace falta uno nuevo.

**El firmado anterior se queda en la carpeta.** La segunda firma sale numerada
—`contrato-firmado-2.pdf`— como manda el ADR-0011, así que quien se equivocó
acaba con dos ficheros y el equivocado delante por orden alfabético. Se acepta:
la aplicación no borra nada del usuario nunca, y ofrecer «reemplazar» obligaría
al pie a prometer una destrucción **antes** de firmar, que es un estado nuevo
del pie por un caso poco frecuente.

**No hay «Firmar otro documento».** Lo hubo, como `--ghost` al pie, y se
retira: la [bandeja](bandeja-de-documentos.md) siempre ofrece abrir y aceptar
arrastre, y dos caminos para lo mismo es uno de más. Del resumen se sale por
tanto de dos maneras: eligiendo otro documento en la bandeja, o volviendo al
panel del mismo con «Volver a firmar».

## Diferencias con el canvas, declaradas

Además de las tres del ticket, estas piezas existen en el código y **no** en
ningún artboard. Se quedan, y se anotan aquí porque el ID-44 pide declararlas
antes que el código y no al revés:

- **El desplegable de certificado**, que sustituye a la tarjeta y al botón
  `Cambiar` que esta ficha declaraba antes. Ningún artboard del canvas original
  lo dibujaba porque ninguno contemplaba tener más de un certificado; el
  artboard `EstadoElegirCertificado` se añadió después, ya con esta decisión
  tomada. El disparador es ahora el sitio donde se cambia, así que el botón
  `Cambiar` desaparece: dos caminos para lo mismo es uno de más.
- **El rótulo «Imagen de la rúbrica».** El artboard pone la miniatura y el
  botón sin encabezado; sin él la fila queda colgando de la lista de casillas
  y no se lee como su propio bloque.
- **El bloque «Lo que dirá el recuadro».** Lo exige el ID-19: el texto lo
  compone Rust y la vista previa enseña **esa** cadena, no una imitación.
- **El aviso de recuadro fuera de página** del visor, que exige el ID-22.
- **El marco de la rúbrica va solo con rúbrica elegida.** El artboard dibuja
  siempre el marco de 56 × 36 px con el garabato dentro; aquí la miniatura
  enseña el JPEG real ya normalizado, así que sin imagen no hay nada honesto
  que enseñar y el botón se queda solo en su fila.

## Componentes y tokens

`.rf-card`, `.rf-btn--primary|--secondary|--ghost`, `.rf-badge`,
`.rf-badge--primary`, `.rf-label`, `.rf-hint`, `.rf-prose`, `.rf-divider`,
`--rf-surface`, `--rf-border-strong` para controles y avisos,
`--rf-border-subtle` para divisores.

El interruptor y las casillas no están en el sistema de diseño: se maquetan con
tokens. Si se repiten en otra pantalla, hay que subirlos a
[design-system.md](design-system.md).

## Decisiones

- El botón «Cambiar» junto al nombre del documento se retiró: la bandeja ya
  hace eso, y dos caminos para lo mismo es uno de más.
- **La elección de páginas vive en el panel, no en el visor.** Se probaron una
  tira de miniaturas bajo la hoja y una etiqueta colgando del propio recuadro.
  La tira se rompe a las 200 páginas y añade mobiliario al visor; la etiqueta
  esconde la decisión dentro del objeto que se está moviendo. Gana el panel, que
  es donde ya vive todo lo que hay que decidir antes de firmar.
- **«Colocado» dejó de ser una bandera.** Serlo obligaba a describir un estado
  con recuadro y sin páginas que nadie sabía dibujar ni qué debía hacer al
  firmar.
- La miniatura de la rúbrica estuvo dentro de la lista de casillas y se sacó:
  rompía el ritmo de la lista y escondía que sin imagen la casilla no debe
  poder marcarse.
- El texto que explicaba qué es una rúbrica se eliminó: lo cargan las
  etiquetas.
- **Los formatos de la rúbrica y su normalización** están fijados en el
  [ADR-0012](../adr/0012-normalizacion-de-la-rubrica-en-rust.md): PNG y JPEG,
  normalizados en Rust al elegirlos, transparencia aplanada a blanco y la
  miniatura obligada a ser honesta al respecto.
- El `Cambiar` del pie vale **solo para esa firma** y no toca la preferencia.
  Cambiar una preferencia desde un pie de página, sin decirlo, manda la
  siguiente firma a un sitio que el usuario no recuerda haber elegido.
- **El pie enseña carpeta *y* nombre**, no solo la carpeta. El nombre lo elige
  la aplicación —`-firmado`, con desempate `-2`, `-3`— y hasta ahora no se veía
  hasta después de firmar. La regla de recorte y el porqué de cada pieza están
  en el componente **ruta de destino** del
  [sistema de diseño](design-system.md); lo que esta ficha añade es que
  `Cambiar` sigue abriendo **el diálogo de guardar**, que es el único gesto que
  fija carpeta y nombre a la vez, mientras que el ajuste persistente de
  Preferencias usa un selector de directorio, que es el único que necesita
  nombrar una carpeta. Que el gesto sea distinto no es incoherencia: uno decide
  una vez y el otro decide siempre.
- **La línea del destino no se corta, envuelve.** Estuvo con
  `white-space: nowrap` y `overflow: hidden`, y eso cortaba en seco —y sin
  puntos suspensivos— lo que el recorte ya había acortado: con `…/Documentos/`
  delante y el `Cambiar` al lado, en 360 px no cabía ni un nombre corto.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards 2 a 5, 9 y 10. El pie del destino y el
resumen se rehicieron el 02/09/2026 con las decisiones del
[#123](https://github.com/sgomez/rfirma/issues/123): están en `Main`, con las
palancas «Pie · destino» y «Pie · recorte», y en `EstadoExito`, con la palanca
«Ficha 14».

**El bloque de colocación se validó el 02/09/2026** con las decisiones del
[#155](https://github.com/sgomez/rfirma/issues/155), que absorbió al
[#154](https://github.com/sgomez/rfirma/issues/154). Está en `Main`, que **se
puede pulsar**: la palanca «Colocación» salta a cada uno de los ocho casos y
desde ahí los radios, el campo, el botón de bajo la hoja y las flechas de página
funcionan de verdad; la palanca «tecleado» recorre los tres errores del rango.
