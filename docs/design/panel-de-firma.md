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

**Pie fijo**: «Se guardará en» con el **nombre de la carpeta** —no la ruta— y
un `Cambiar`, y debajo el botón primario a ancho completo. El destino se ve
antes de firmar, no después. Lo fija el
[ADR-0011](../adr/0011-destino-del-documento-firmado.md).

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
- **Tarjeta de certificado**: 16 px de relleno, 6 px entre líneas, borde
  `--rf-border-strong`. Escarapela de 20 px y nombre en `.rf-title` a 14 px;
  debajo, DNI y emisora en una sola línea `.rf-body rf-text-muted` separadas
  por `·`.
- **Esqueletos de carga**: dos bloques de **66 px** —el alto de la tarjeta que
  van a sustituir, para que el panel no salte—, con `--rf-radius-md` y borde
  `--rf-border-subtle`. El primero lleva el degradado horizontal que dice que
  la búsqueda sigue viva; el segundo, solo su borde.
- **Casillas del contenido**: cuadrado de 18 px con `--rf-radius-sm`, contorno
  `--rf-border-strong` cuando está vacío y relleno `--rf-primary` con la marca
  de verificación de 12 px (trazo 3) cuando está marcado. Fila de 24 px de alto
  mínimo, con 8 px entre la casilla y su texto.
- **Interruptor**: pastilla de 40×24 px con el pomo de 16 px. Va **delante**
  del texto.
- **Rúbrica**: la miniatura es un rectángulo de 56×36 px con borde
  `--rf-border-strong` y `--rf-radius-sm`, y comparte fila con el botón
  secundario, que ocupa el resto y mide 36 px de alto.
- **Pie**: 16 px de relleno, borde superior de 1 px en `--rf-border-subtle`.
  Dentro: el rótulo «Se guardará en», 4 px debajo la fila con el icono de
  carpeta de 20 px, la carpeta en `.rf-prose` recortada con elipsis y un
  `Cambiar` fantasma de 32 px de alto, 8 px de relleno lateral y 12 px de
  cuerpo. Debajo, el botón primario a ancho completo.
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

Tarjeta con el nombre del titular, el DNI y la autoridad emisora.

## Firma visible

Cuatro piezas en este orden:

1. **Interruptor**: «Estampar un recuadro de firma en el documento».
2. **Pista de ubicación**: «Página 3 · arrástralo para colocarlo».
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

## Estados

- **Sin certificado**: botón «Elegir certificado»; la sección de firma visible
  al 40 % y sin interacción; el botón primario deshabilitado.
- **Cargando certificados**: «Buscando certificados…» y dos esqueletos.
- **Sin certificados**: bloque con borde `--rf-border-strong`, explicación
  («si usas una tarjeta, comprueba que está insertada y que el lector está
  conectado») y dos salidas: «Volver a buscar» y «Otro módulo…».
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
- **Firmado**: el panel entero se reemplaza por el resumen (ver abajo). Está
  implementado **en lo que no necesita datos nuevos**: la cabecera con el
  nombre del fichero que quedó escrito, el rótulo `RESUMEN`, la insignia
  `PAdES` —rFirma no produce otro formato— y el pie con «Firmar otro
  documento» a ancho completo. Es el único acuse de recibo del recorrido:
  antes la postfirma devolvía un `SignedDocument` que nadie leía y la ventana
  volvía al panel con el nombre del fichero **original**, así que quien
  firmaba no sabía si se había escrito nada.
  Siguen **pendientes** tres piezas del artboard, y por la regla del dato
  desconocido no ocupan sitio: la insignia con el número de firmas y las
  tarjetas de cada firma —nadie cuenta todavía las firmas del PDF, ver
  arriba— y los botones «Abrir el PDF» y «Abrir la carpeta», que no son piel
  sino dos `OpenURI` que aún no existen. Reaparecen en cuanto haya quien lea
  las firmas del resultado y quien abra un URI.

## El resumen, tras firmar

Sustituye a la configuración, que ya no sirve de nada:

- Nombre del fichero resultante y su tamaño.
- `Resumen`: insignias `PAdES` y `2 firmas`, y **todas las firmas del
  documento**, no solo la del usuario, con la insignia `La tuya` en la suya.
- «Abrir el PDF» (primario) y «Abrir la carpeta» (secundario). Los dos son el
  portal `OpenURI`, que funciona sin declarar ningún permiso. Cargan más peso
  del que parece: bajo el arenero son la única forma que tiene el usuario de
  llegar al fichero sin saberse la ruta.
- Al pie, «Firmar otro documento» como `--ghost`.

Enseñar todas las firmas es la contrapartida del aviso de cofirma: si antes se
avisa de que el PDF ya llevaba una, el resumen tiene que enseñarlas todas.

## Diferencias con el canvas, declaradas

Además de las tres del ticket, estas piezas existen en el código y **no** en
ningún artboard. Se quedan, y se anotan aquí porque el ID-44 pide declararlas
antes que el código y no al revés:

- **El botón `Cambiar` de la tarjeta de certificado.** Ningún artboard lo
  dibuja, pero con varios certificados en la tarjeta no habría forma de
  cambiar de uno a otro sin volver a buscar.
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

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards 2 a 5, 9 y 10.
