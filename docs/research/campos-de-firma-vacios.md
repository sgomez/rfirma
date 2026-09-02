# Campos de firma vacíos preexistentes: quién los ve y qué cuesta ofrecerlos

Sondeo del [#149](https://github.com/sgomez/rfirma/issues/149), hijo del mapa
[#148](https://github.com/sgomez/rfirma/issues/148) (rfirma v0.3, «el recuadro»). Decide la
mitad de la **ficha 8**: ofrecer, si el PDF ya trae un campo de firma vacío, ese campo en
vez de hacer arrastrar el recuadro.

**Respuesta corta.** Enumerar es de la interfaz y sale gratis: `pdf.js` 6.3.289 —el que ya
está en el proyecto— devuelve **nombre, `/Rect` y página** de cada campo `/FT /Sig` con
`page.getAnnotations()`, sin orden nueva en Rust ni entrada FFI nueva en el puente. Pero
`pdf.js` **no puede decir si un campo está vacío o ya firmado**: hunde `fieldValue` a `null`
para todo widget de firma, por código, así que `/V` es invisible desde el frontal. Y firmar
sobre un campo que ya tiene firma **no da error**: AutoFirma la sustituye y el documento
sale con una sola firma, la nueva. Ese par —no sé distinguirlos, y equivocarme destruye una
firma ajena en silencio— es el hallazgo que decide el diseño.

En el puente, en cambio, no hay obra: `signatureField` es una clave más de un `Properties`
que Java acepta sin filtrar, y el sello de sesión del ADR-0016 la sella igual que a las
otras. Lo que sí tiene consecuencias es que **`signatureField` anula la geometría**: con la
clave puesta, los cinco parámetros de posición son inertes —salida byte a byte idéntica con
ellos y sin ellos— y `signaturePages=all` **se ignora sin avisar**, o sea que anclar a un
campo existente y el multipágina de la decisión 2 del mapa son mutuamente excluyentes.

---

## El banco

Vive en `docs/research/assets/empty-fields-driver/` de la rama
`research/empty-signature-fields`, que no se fusiona.

| Fichero | Qué es |
| --- | --- |
| `mkpdf.py` | Escribe a mano `empty-fields.pdf`: 3 páginas A4 (`595×842`) y tres campos `/FT /Sig` **sin `/V`**. Segundo argumento opcional: `/Rotate` de la última página |
| `pom.xml`, `src/main/java/probe/Probe.java` | Driver contra los artefactos de AutoFirma 1.9.1 en `~/.m2`, JVM completa (GraalVM 25.3.4). `list` enumera con `PdfUtil`; `sign` firma con o sin `signatureField` |
| `probe-pdfjs.mjs`, `compare-pdfjs.mjs` | Enumeración con el `pdfjs-dist` **del propio proyecto** (`rfirma-app/node_modules`), versión 6.3.289 |
| `run.sh` | Reconstruye el banco entero |
| `edge.sh`, `multipage.sh` | Los modos de fallo y el cruce con `signaturePages` |

Los tres campos del PDF de partida:

| Campo | Página | `/Rect` | Particularidad |
| --- | --- | --- | --- |
| `Firma1` | 1 | `[72 600 300 700]` | **vacío y con `/AP /N`** (apariencia de recuadro dibujado, lo que ponen Acrobat y LibreOffice) |
| `FirmaInvisible` | 2 | `[0 0 0 0]` | rectángulo degenerado |
| `Firma2` | 3 | `[200 100 450 180]` | vacío y sin apariencia |

El certificado es el **de pruebas de la FNMT** (`SP_Empleado_publico_activo.p12`) del kit
documentado en [`token-pkcs11-pruebas.md`](token-pkcs11-pruebas.md). El certificado personal
del titular no se usa aquí, como en el resto del proyecto.

---

## 1. Quién los enumera: `pdf.js`, y sin tocar nada más

`page.getAnnotations()` sobre `empty-fields.pdf`, con la misma llamada de carga que hace
rfirma (`getDocument({ data: bytes })`):

```
pagina 1  Widget  fieldType=Sig  fieldName=Firma1          rect=[72,600,300,700]
pagina 2  Widget  fieldType=Sig  fieldName=FirmaInvisible  rect=[0,0,0,0]
pagina 3  Widget  fieldType=Sig  fieldName=Firma2          rect=[200,100,450,180]
```

Lo que AutoFirma obtiene por su cuenta, con `PdfUtil.getPdfEmptySignatureFields`, es **la
misma lista**:

```
campos vacios: 3
  FirmaInvisible   pagina=2 rect=[0 0 0 0]
  Firma1           pagina=1 rect=[72 600 300 700]
  Firma2           pagina=3 rect=[200 100 450 180]
```

Dos diferencias de forma, ambas a favor del frontal: `getAnnotations()` entrega los campos
**en orden de página**, mientras que `AcroFields.getBlankSignatureNames()` los entrega
desordenados (arriba salen 2, 1, 3); y `getAnnotations()` no cuesta una travesía FFI.

**Veredicto de la pregunta 1: esto es interfaz.** No hace falta orden nueva en Rust ni
entrada FFI nueva. Ninguna de las tres guardias de `commands/mod.rs` entra en juego, porque
no hay orden nueva que numerar.

### El pero: `pdf.js` no distingue vacío de firmado

Es una decisión de código de `pdf.js`, no una limitación del PDF de prueba
(`rfirma-app/node_modules/pdfjs-dist/legacy/build/pdf.worker.mjs:61816-61822`):

```js
class SignatureWidgetAnnotation extends WidgetAnnotation {
  constructor(params) {
    super(params);
    this.data.fieldValue = null;   // <- siempre, haya /V o no
```

Y `getFieldObject()`, la vía alternativa, hace lo mismo (`:61825-61832`): `value: null`
fijo, tipo `"signature"`. (En el banco, además, `doc.getFieldObjects()` devolvió `{}`.)

Medido sobre los tres PDF del banco —el de partida, uno firmado sobre `Firma2` y otro
firmado sobre `FirmaInvisible`—, esto es todo lo que `pdf.js` deja ver:

| PDF | Campo | `fieldValue` | `hasAppearance` | `annotationFlags` |
| --- | --- | --- | --- | --- |
| `empty-fields.pdf` | `Firma1` | `null` | **`true`** | 4 |
| `empty-fields.pdf` | `FirmaInvisible` | `null` | `false` | 4 |
| `empty-fields.pdf` | `Firma2` | `null` | `false` | 4 |
| `signed-field.pdf` | `Firma1` | `null` | `true` | 4 |
| `signed-field.pdf` | `Firma2` | `null` | `true` | **132** |
| `signed-invisible.pdf` | `FirmaInvisible` | `null` | `true` | **132** |

`fieldValue` no sirve: es `null` en las seis filas. `hasAppearance` tampoco: es `true` en
`Firma1`, que **está vacío** —por eso el banco le pone un `/AP`, que es lo normal en los
campos que crean Acrobat y LibreOffice—. Queda el bit 8 de `annotationFlags` (`132 = 4+128`,
*Locked*), que iText pone al firmar; pero es una costumbre del productor, no algo que la
ISO 32000-1 exija a nadie, y apoyarse en él es apostar a que todo el que firme se comporte
como iText.

**Conclusión: desde `pdf.js` se enumeran los campos de firma; no se sabe cuáles están
vacíos.** Quien lo sabe de verdad hoy es Java (`AcroFields.getBlankSignatureNames()`, vía
`PdfUtil.getPdfEmptySignatureFields`, `PdfUtil.java:412-454`). En Rust no hay con qué: el
backend no tiene ninguna dependencia de PDF (`rfirma-app/src-tauri/Cargo.toml` — ni `lopdf`,
ni `pdf`, ni `pdfium`; el PDF es bytes opacos).

Por qué importa, y no es una pega teórica: ver el modo de fallo 1 de la sección 5.

---

## 2. Qué se sabe de cada campo, y en qué espacio de coordenadas

De cada campo, `getAnnotations()` da lo que la ficha 8 necesita: `fieldName` (el nombre que
hay que pasar a `signatureField`), `rect` y la página (implícita, es la que se preguntó).

**El `/Rect` es utilizable tal cual como recuadro**, y es la mejor noticia del sondeo:
`pdf.js` lo entrega **en espacio de usuario PDF y ya normalizado** (esquinas ordenadas), que
es exactamente la definición de `UserSpaceRect` en
`rfirma-app/src/viewer/signatureBox.ts:22-31`. Pintarlo es `toPixels(viewport, rect)`, la
función que ya existe. No hay conversión que escribir.

Con página rotada la cosa se afila. Con `/Rotate 90` en la página 3, los dos enumeradores
**no dicen lo mismo**:

| Quién | `Firma2` con `/Rotate 90` |
| --- | --- |
| `pdf.js` `getAnnotations().rect` | `[200, 100, 450, 180]` — el `/Rect` crudo |
| `PdfUtil.getPdfEmptySignatureFields` | `[100 145 180 395]` |

No hay contradicción: son **dos espacios distintos**, los mismos dos que separa
[`coordenadas-recuadro-pades.md`](coordenadas-recuadro-pades.md). `pdf.js` da el espacio de
usuario, que es donde vive `UserSpaceRect` y donde el viewport sabe pintar.
`AcroFields.getFieldPositions` da ya el espacio de los `extraParams`, es decir `T⁻¹` aplicado
(con `mx1 = 595`: `x' = y`, `y' = 595 − x`, y renormalizado). Dicho de otro modo:

* Para **resaltar el campo sobre la hoja**, el `/Rect` de `pdf.js` vale tal cual, con
  rotación o sin ella.
* Para **convertir un campo en un recuadro corriente** —copiar su geometría a los cinco
  parámetros de posición y firmar como si lo hubiera arrastrado la persona— hay que aplicar
  la `T⁻¹` de `coordenadas-recuadro-pades.md`. Y eso **crea un campo nuevo**, no rellena el
  que había.

---

## 3. Qué le pasa al puente: nada que construir, algo que decidir

**El puente no filtra.** Los `extraParams` cruzan la frontera FFI como un bloque de texto
`java.util.Properties` que Rust compone en `signing/properties.rs:30` y que Java carga entero
con `Properties.load(...)` en `SessionStamp.parseParams`
(`rfirma-native-bridge/.../SessionStamp.java:314-326`), para pasárselo sin tocar a
`PAdESTriPhasePreProcessor` (`PadesBridge.java:93-95`). No hay lista blanca en Java.
`signatureField` llega si Rust lo escribe. **El puente Java no se toca.**

**El sello de sesión no se rompe.** El ADR-0016 sella los `extraParams` *efectivos* releídos
tras la prefirma, sin saber qué claves son; añadir una más entra igual que las demás.

**La lista blanca real es Rust**, y ahí sí hay una decisión. `SignatureConfig`
(`rfirma-app/src-tauri/src/signing/config.rs:115-127`) tiene cuatro campos y el `enum
Setting` (`:33-73`) cierra la configuración en **cinco ajustes**, con dos pruebas que lo
vigilan (`closes_the_configuration_at_five_settings`, `emits_no_key_outside_the_five_settings`,
`:191-212`) y un destructurado exhaustivo en `extra_params()` que no compila si aparece un
campo nuevo. El [#31](https://github.com/sgomez/rfirma/issues/31) dejó `signatureField`
fuera de las cinco claves; conviene precisar que **no** figura en la lista negra explícita de
`never_sends_what_the_spec_ruled_out` (`:302-319`): simplemente no existe.

Hay dos formas de meterlo, y el sondeo se moja:

* **Un sexto ajuste** `Setting::SignatureField` junto a `Setting::Geometry`. Barato de
  escribir y **malo**: los dos ajustes se contradicen entre sí, y el sello acabaría
  transportando cinco parámetros de posición que Java no mira (ver más abajo). Un sello con
  datos muertos que además mienten sobre dónde va la firma.
* **Convertir `Setting::Geometry` en una elección de dos ramas** —recuadro arrastrado
  (página + cuatro esquinas) *o* campo existente (nombre)—, o sea un `enum` de colocación en
  `SignatureConfig::signature_box`. La configuración **sigue teniendo cinco ajustes**, las
  dos guardias siguen en verde sin tocar el número, y el tipo hace imposible enviar las dos
  cosas a la vez. **Es la que este sondeo recomienda.**

Que las dos ramas se excluyen no es una opinión de diseño, está medido: firmar con
`signatureField=Firma2` **enviando** los cinco parámetros de posición apuntando a otro sitio
(página 1, `[40 40 160 90]`) y firmar **borrándolos** produce el mismo fichero, 56 696 bytes
las dos veces, con la firma en el `/Rect` del campo. Los parámetros de posición son
literalmente inertes. El código dice lo mismo
(`PdfSessionManager.java:496` y `:542-544`):

```java
if (signaturePositionOnPage != null && signatureField == null) { ... }
// Firma en un campo preexistente (visile o invisible)
else if (signatureField != null) {
    sap.setVisibleSignature(signatureField);
}
```

### El choque con el multipágina

`signatureField` **gana también a `signaturePages`**. Firmando con
`signatureField=Firma2;signaturePages=all` sobre el PDF de tres páginas, el resultado es
byte a byte el mismo que sin `signaturePages` (56 696 bytes), y el widget aparece **solo en
la página 3**, la del campo:

```
pagina 1: Firma1 id=10R rect=72,600,300,700 flags=4
pagina 2: FirmaInvisible id=11R rect=0,0,0,0 flags=4
pagina 3: Firma2 id=12R rect=200,100,450,180 flags=132
```

Sin aviso, sin error, sin registro. Es exactamente la degradación silenciosa que el ID-22
rechaza y que el mapa señala como lo que **no** hay que copiar. La decisión 2 del #148 mete
el multipágina en v0.3; este sondeo añade que **anclar a un campo existente lo apaga**, y que
si rfirma ofrece las dos cosas tiene que decirlo en la interfaz, no dejar que el puente lo
resuelva callando.

`PdfSessionManager` desactiva además la página nueva al final cuando hay campo
(`:396`, `pages.contains(NEW_PAGE) && signatureField == null`), lo cual da igual: el `append`
ya quedó fuera por la decisión 3 del mapa.

---

## 4. Qué hace exactamente AutoFirma

**Cuándo pregunta.** Lo primero de todo, antes de mirar siquiera si se pidió firma visible:
`VisiblePdfSignatureManager.getVisibleSignatureParams` llama a
`PdfUtil.getPdfEmptySignatureFields(data)` en la línea 56, y si la lista no está vacía
(`:59`) va a `PdfEmptySignatureFieldsChooserDialog.selectField(...)` (`:95`). El panel de
arrastre (`SignPdfDialog`) viene después, si es que viene.

**Qué pregunta.** `selectField` (`PdfEmptySignatureFieldsChooserDialog.java:261-308`) es un
`JOptionPane` con un desplegable de campos y **tres** salidas:

| Botón | Devuelve | Efecto |
| --- | --- | --- |
| usar el campo elegido | el `SignatureField` | se añade el `extraParam` `signatureField` y **se salta el arrastre** |
| crear un campo nuevo | `null` | flujo normal: panel de arrastre si se pidió firma visible; si no, se firma sin diálogo |
| cancelar | lanza `AOCancelledOperationException` | aborta la operación entera |

**Qué pasa si el usuario dice que no.** Cae en `null` y se comporta como si el documento no
tuviera campos: `showVisibleSignatureDialog` o firma directa
(`VisiblePdfSignatureManager.java:100-108`).

**Si elige uno.** Se fuerza `signatureVisible = false` al llamar al diálogo de rúbrica
(`:117-121`), con el comentario de la fuente: *«nunca se permitirá seleccionar el área de
firma, ya que se usará la del campo seleccionado»*. La rúbrica sí se puede seguir
configurando, recortada al `/Rect` del campo.

**Se respeta el `/Rect` del campo**, no el que se pase. Medido arriba y confirmado en el
código.

---

## 5. Modos de fallo medidos

**1. Firmar sobre un campo ya firmado sale «bien» y se lleva la firma anterior por delante.**
Es el peor. Firmando otra vez `Firma2` sobre `signed-field.pdf`, el driver no protesta
—`OK /tmp/149-yafirmado.pdf 112769 bytes`— y `pdfsig` del resultado cuenta **una sola
firma**, la nueva:

```
Signature #3:
  - Signature Field Name: Firma2
  - Signing Time: Sep 02 2026 17:00:06
  - Total document signed
  - Signature Validation: Signature is Valid.
```

La firma original ha dejado de estar referenciada por el campo. Nadie avisa. Y como la
sección 1 demuestra que `pdf.js` **no puede distinguir** un campo firmado de uno vacío, una
implementación ingenua de la ficha 8 —enumerar con `getAnnotations()`, ofrecerlo todo— llega
a este resultado por el camino corto en cuanto alguien abra un PDF con dos campos y uno ya
firmado. Es el argumento decisivo para que el filtro de «vacío» no se haga por heurística.

**2. Campo inexistente: falla, y falla alto.** `signatureField=NoExiste` lanza
`java.lang.IllegalArgumentException: The field NoExiste does not exist.` desde
`PdfSignatureAppearance.setVisibleSignature` (`:300`), a través de
`PdfSessionManager:543` → `PAdESTriPhaseSigner.preSign:176`. Es ruidoso, que es lo correcto,
pero es una excepción cruda: si rfirma manda un nombre inventado, lo que sube por el puente
es eso.

**3. `/Rect` degenerado.** `getPdfEmptySignatureFields` solo descarta el campo si
`getFieldPositions` devuelve `null` o menos de cinco valores (`PdfUtil.java:436-438`); **no
comprueba que el rectángulo tenga área**. Y `PdfVisibleAreasUtils.isVisibleSignature`
(`:632-651`) da `true` en cuanto hay `signatureField`, **saltándose** la comprobación de
dimensión que sí aplica al recuadro arrastrado. Resultado: `FirmaInvisible` (`[0 0 0 0]`)
aparece en la lista de AutoFirma y se firma sin rechistar. En el banco, firmarlo produce un
PDF válido con la firma en un campo de área cero. rfirma tendrá que decidir si eso se ofrece
(y cómo se resalta un rectángulo de 0×0 sobre la hoja) o se filtra.

**4. Documento ya firmado en otro campo.** `PdfUtil.getAppendMode` fuerza revisión
incremental si el PDF trae firmas (`:232-237`). Correcto, pero significa que firmar un campo
vacío de un documento ya firmado cambia el modo de escritura sin que nadie lo pida.

**5. Un campo, varios widgets.** `getPdfEmptySignatureFields` usa solo el primer grupo de
cinco flotantes que devuelve `getFieldPositions` e ignora los demás: un campo replicado en
varias páginas —justo lo que produce el mecanismo del [#116](https://github.com/sgomez/rfirma/issues/116)—
se ofrecería como si estuviera solo en la primera. `pdf.js`, en cambio, lo devolvería una vez
por página, con el mismo `fieldName`: **hay que deduplicar por nombre** al construir la
lista.

---

## Lo que este sondeo recomienda al spec

1. **Enumerar en el frontal, con `pdf.js`**, dentro de `viewer/`. Hay que ampliar el puerto
   `PdfPage` de `rfirma-app/src/viewer/pdf.ts` con un `getAnnotations` y exponerlo desde
   `adaptPage` en `pdfjsLoader.ts:31-44`; el `PDFPageProxy` está ahí, solo que hoy no sale
   del adaptador. Sin orden de Tauri nueva, sin FFI nueva, sin dependencia de PDF en Rust.
   Deduplicar por `fieldName` y quedarse con la primera página.
2. **Decidir cómo se filtra lo que ya está firmado**, que es lo único que `pdf.js` no
   resuelve. Tres salidas, en orden de coste:
   * *Ofrecer solo lo que se pueda demostrar vacío.* Requiere que alguien lea `/V`: una
     entrada FFI nueva en el puente que exponga `PdfUtil.getPdfEmptySignatureFields`, o un
     analizador de PDF en Rust. Es lo correcto y es lo caro.
   * *Ofrecerlos todos y dejar que falle.* Inaceptable: el modo de fallo 1 no falla, borra
     una firma.
   * *Ofrecer solo si el documento no tiene ninguna firma todavía.* En un PDF sin firmar,
     todo campo `/Sig` está vacío por construcción, así que la ambigüedad **desaparece**, y
     es exactamente el caso que la ficha 8 describe: «quien redactó el documento ya decidió
     dónde va la firma». Es la salida barata y honesta para v0.3, y deja el caso multifirma
     para cuando exista la entrada FFI.

     Ojo con el detector: **`documentInfo.IsSignaturesPresent` no vale**. Sale de
     `/SigFlags & 1` (`pdf.worker.mjs:66499-66503`), que significa «este documento tiene
     campos de firma», no «tiene firmas»; medido, da `true` en los cuatro PDF del banco,
     firmados y sin firmar. Lo que sí separa los dos grupos limpiamente es buscar
     `/ByteRange` en los bytes crudos —0 apariciones en `empty-fields.pdf`, 1 en cada uno de
     los tres firmados—, que la interfaz ya tiene en la mano porque son los mismos bytes que
     le pasa a `pdf.js`. Es sólido por construcción: todo diccionario de firma lleva
     `/ByteRange` y no puede vivir comprimido en un `/ObjStm`, justo para que se pueda
     parchear. Y se equivoca solo hacia el lado seguro: un falso positivo apaga una comodidad,
     nunca destruye una firma.
3. **Colocación como elección de dos ramas**, no como sexto ajuste: `Setting::Geometry`
   pasa a emitir *o* los cinco parámetros de posición *o* `signatureField`. La configuración
   sigue cerrada en cinco y el tipo impide la combinación que Java resuelve callando.
4. **Decir en la interfaz que anclar a un campo apaga el multipágina** y fija el tamaño.
   Elegir un campo no es «poner el recuadro ahí»: es aceptar el rectángulo, la página y el
   tamaño que otro decidió. Las fichas 6 y 24 dejan de aplicar mientras el ancla esté
   puesta, y eso hay que verlo, no descubrirlo.
5. **La vista previa de la ficha 7 sigue funcionando** sobre un campo anclado sin cambios: la
   prefirma en seco de [`prefirma-en-seco-pdfjs.md`](prefirma-en-seco-pdfjs.md) manda el
   mismo `extraParams`, y el widget resultante lo pinta `pdf.js` igual (medido: la anotación
   de `Firma2` sale con `hasAppearance: true` tras firmar).

## Lo que no se ha medido

* **Campos de firma dentro de un árbol de campos** (`/Kids`, nombres cualificados con
  puntos como `formulario.firma`). El banco usa campos planos. `AcroFields` devuelve el
  nombre cualificado completo, y `getAnnotations().fieldName` de `pdf.js` también lo
  compone; que coincidan exactamente **no está comprobado**, y es lo que decide si el nombre
  que ve la interfaz sirve tal cual como `signatureField`.
* **Campos con `/Ff` de solo lectura o con `/Lock`**, que la ISO permite y que cambiarían si
  el campo debe ofrecerse.
* **`doc.getFieldObjects()`**, que en el banco devolvió `{}` pese a haber tres campos con
  `/T`; no se ha investigado por qué, porque para firmas devuelve `value: null` de todos
  modos y no aporta nada sobre `getAnnotations()`.
* **El detector `/ByteRange` contra PDF cifrados o con firmas incrustadas de forma exótica.**
  Se ha medido sobre los cuatro ficheros del banco, todos producidos por iText; no sobre
  documentos de otros productores.
* **La rúbrica de imagen dentro de un campo anclado**: AutoFirma la recorta al `/Rect` del
  campo, pero no se ha comprobado cómo queda la escala con la normalización de rúbrica de
  Rust del ADR-0012.
