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

## El aviso de cofirma

Icono de **información**, borde `--rf-border-subtle`:
«Ya lleva **1 firma**: la tuya será una **cofirma**».

No es una alarma —añadir una firma sin invalidar la anterior es lo normal—, así
que no lleva icono de aviso.

A la derecha, un **`Ver ›`** que despliega las firmas que el documento ya
tiene, con quién y cuándo, usando el mismo componente de firma que el resumen
del estado firmado. Es lo que permite auditar un PDF ajeno antes de añadirle
nada.

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
  (`CKR_DEVICE_REMOVED durante C_Sign (fase: firma)`) con un «Copiar detalle».
  El botón pasa a «Volver a intentarlo».
- **Firmado**: el panel entero se reemplaza por el resumen (ver abajo).

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
- El `Cambiar` del pie vale **solo para esa firma** y no toca la preferencia.
  Cambiar una preferencia desde un pie de página, sin decirlo, manda la
  siguiente firma a un sitio que el usuario no recuerda haber elegido.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards 2 a 5, 9 y 10.
