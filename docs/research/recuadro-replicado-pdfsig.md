# Un campo de firma, tres anotaciones: qué dice `pdfsig` del recuadro replicado

Sondeo del [#116](https://github.com/sgomez/rfirma/issues/116), hijo del mapa
[#113](https://github.com/sgomez/rfirma/issues/113). Decide la **ficha 24** del informe
*rFirma después de v0.1* (v0.3), la que el mapa agrupa con «redimensionar y anclar el
recuadro (6, 8, 24)»: si rfirma debe ofrecer estampar el recuadro en más de una página, y
con qué mecanismo.

La pregunta del ticket: AutoFirma firma «en todas las páginas» con **un solo campo de
firma cuyo widget se replica en cada página**. Un campo con varias anotaciones se sale de
la especificación PDF. ¿Qué dice `pdfsig` de un documento así?

**Respuesta corta: `pdfsig` no dice nada.** Valida el documento como **una sola firma
válida** y con el documento **entero** cubierto, tanto con `signaturePages=all` como con
`1-2`. Poppler tampoco protesta al rasterizar, y **pinta el recuadro en todas las páginas
que lo listan**. La irregularidad es real —hay **una** anotación referenciada desde
**tres** `/Annots`, con `/P` apuntando solo a la primera página— pero ninguna herramienta
disponible aquí la denuncia. Lo que sí es una limitación dura del diseño es que, al ser
literalmente el mismo objeto, **el `/Rect` es forzosamente idéntico en todas las
páginas**: con este mecanismo no se puede colocar el recuadro en sitios distintos página a
página.

---

## Qué se midió y cómo

### El banco

Todo vive en `docs/research/assets/replicated-widget-driver/` de la rama
`research/replicated-widget-pdfsig`, que no se fusiona: el banco de pruebas se queda ahí
(los dos PDF los necesita la tarea de VALIDe, el [#118](https://github.com/sgomez/rfirma/issues/118)).

| Fichero | Qué es |
| --- | --- |
| `src/main/java/probe/Probe.java`, `pom.xml`, `run.sh` | Driver contra los artefactos de AutoFirma 1.9.1 en `~/.m2`, JVM completa (GraalVM 25.3.4) |
| `mkpdf.py`, `three-pages.pdf` | El PDF de partida, 3 páginas A4 (`595×842`), sin firmar ni anotar |
| `pages-all.pdf` | Firmado con `signaturePages=all` |
| `pages-1-2.pdf` | Firmado con `signaturePages=1-2` |
| `inspect.py`, `objstm.py` | Inspección estructural sin xref; `objstm.py` infla los `/Type /ObjStm` que deja `setFullCompression` |

El certificado es el **de pruebas de la FNMT** del kit documentado en
[`token-pkcs11-pruebas.md`](token-pkcs11-pruebas.md). El certificado personal del titular
no se usa aquí, como en el resto del proyecto.

Los parámetros de firma, tal cual (`Probe.java:33-40`):

```java
p.setProperty("signatureSubFilter", "ETSI.CAdES.detached");
p.setProperty("signaturePages", pages);            // "all" o "1-2"
p.setProperty("signaturePositionOnPageLowerLeftX",  "100");
p.setProperty("signaturePositionOnPageLowerLeftY",  "100");
p.setProperty("signaturePositionOnPageUpperRightX", "300");
p.setProperty("signaturePositionOnPageUpperRightY", "160");
p.setProperty("layer2Text", "Firmado por: prueba rfirma #116\nsignaturePages=" + pages);
p.setProperty("signReason", "Sondeo #116");
```

> **Aviso de reproducibilidad.** La línea `P12=` de `run.sh` apunta a un worktree ya borrado.
> Los dos PDF ya están producidos y **no hace falta volver a firmar**; si se necesita una
> variante, hay que corregir esa ruta al clon del kit de la FNMT.

Herramientas: poppler 26.01.0 (`pdfsig`, `pdftoppm`, `pdftocairo`, `pdftotext`), `python3`
sin dependencias externas. **`qpdf` y `mutool` no están instalados en este entorno**, y
tampoco `pikepdf` ni `pypdf`; lo que el ticket pedía comprobar con ellos queda **sin
medir** (ver «Lo que no se ha medido»).

---

## Lo que `pdfsig` dice

```
$ pdfsig pages-all.pdf
Digital Signature Info of: pages-all.pdf
Signature #1:
  - Signature Field Name: Signature1
  - Signer Certificate Common Name: EIDAS CERTIFICADO PRUEBAS - 99999999R
  - Signing Time: Sep 01 2026 21:57:21
  - Signing Hash Algorithm: SHA-256
  - Signature Type: ETSI.CAdES.detached
  - Signed Ranges: [0 - 277], [54279 - 56676]
  - Total document signed
  - Signature Validation: Signature is Valid.
  - Certificate Validation: Unknown issue with Certificate or corrupted data.
```

`pages-1-2.pdf` da lo mismo palabra por palabra salvo el segundo rango
(`[54279 - 56670]`).

Tres cosas importan:

1. **`Signature #1` y ninguna más.** Poppler cuenta **una** firma, no tres. Coherente con
   la estructura: hay un campo, no tres.
2. **`Total document signed`** en los dos. El `/ByteRange` cubre todo el fichero; no hay
   revisión incremental posterior sin firmar.
3. **`Signature is Valid.`** La irregularidad estructural no afecta al digest ni al
   `SubFilter`, que es `ETSI.CAdES.detached` como pide el ID de firma PAdES-BES del
   proyecto.

El `Certificate Validation: Unknown issue…` es la cadena de la CA de pruebas de la FNMT,
que no está en el almacén de confianza de este equipo. Sale igual al validar cualquier PDF
firmado con el kit de pruebas —incluido uno de una sola página—, así que **no** tiene que
ver con el widget replicado.

---

## La estructura: una anotación, tres referencias

`iText` con `setFullCompression` esconde las páginas y el widget dentro de un
`/Type /ObjStm`, así que `inspect.py` (que solo lee texto claro) no los ve. Hay que inflar
el object stream:

```
$ python3 objstm.py pages-all.pdf
== ObjStm obj 20: 9 objetos ==
  obj 7: <</FT/Sig/T(Signature1)/V 1 0 R/F 132/Type/Annot/Subtype/Widget
          /Rect[100 100 300 160]/AP<</N 6 0 R>>/P 9 0 R/DR<</XObject<</FRM 5 0 R>>>>>>
  obj 13: <</Type/Pages/Kids[9 0 R 14 0 R 15 0 R]/Count 3/ITXT(2.1.7)>>
  obj  9: <</Type/Page/Parent 13 0 R/MediaBox[0 0 595 842]/…/Contents 17 0 R/Annots[7 0 R]>>
  obj 14: <</Type/Page/Parent 13 0 R/MediaBox[0 0 595 842]/…/Contents 18 0 R/Annots[7 0 R]>>
  obj 15: <</Type/Page/Parent 13 0 R/MediaBox[0 0 595 842]/…/Contents 19 0 R/Annots[7 0 R]>>
```

Y el catálogo, este sí en claro (`inspect.py`):

```
== /AcroForm ==
  obj 12: <</Type/Catalog/Pages 13 0 R/AcroForm<</Fields[7 0 R 7 0 R 7 0 R]
           /DA(/Helv 0 Tf 0 g )/DR<<…>>/SigFlags 3>>>>
```

De aquí sale, punto por punto, lo que el ticket preguntaba.

### ¿Cuántos objetos de anotación hay?

**Uno.** Contando los diccionarios `/Type /Annot` y `/Subtype /Widget` tanto en el texto
claro como dentro de los object streams inflados:

```
pages-all.pdf   Annot dicts: 1   Widget: 1
pages-1-2.pdf   Annot dicts: 1   Widget: 1
three-pages.pdf Annot dicts: 0   Widget: 0
```

No hay widgets separados por página. Tampoco hay tres campos.

### ¿`7 0` es campo y widget a la vez?

**Sí, es el diccionario fusionado campo/widget.** El mismo objeto lleva las claves de
campo (`/FT /Sig`, `/T (Signature1)`, `/V 1 0 R`) y las de anotación
(`/Type /Annot`, `/Subtype /Widget`, `/Rect`, `/AP`, `/P`, `/F 132`). Es exactamente la
fusión que autoriza **PDF 32000-1:2008 §12.7.3.1**, para un campo sin hijos que además es
su propia anotación widget. Esa parte **no** es la irregular.

`/F 132` = `Print` (4) + `Locked` (128), la combinación normal de un widget de firma
(§12.5.3, Tabla 165).

### ¿Qué lleva `/Annots` de cada página?

Las tres páginas hijas de `/Pages /Kids [9 0 R 14 0 R 15 0 R]` llevan
**`/Annots [7 0 R]`**: la misma referencia indirecta al mismo objeto. En `pages-1-2.pdf`
la página 3 (`obj 15`) **no tiene `/Annots`** en absoluto, y su `/Fields` es
`[7 0 R 7 0 R]` en vez de `[7 0 R 7 0 R 7 0 R]`. O sea: el rango de páginas se respeta
literalmente, y el número de entradas repetidas en `/Fields` coincide con el número de
páginas estampadas.

### ¿A qué página apunta `/P`?

**`/P 9 0 R`, que es la página 1** (el primer `/Kids` de `obj 13`). En los dos PDF, porque
en los dos la primera página de la lista es la 1.

Esto es lo que se sale de la norma:

- **§12.5.2, Tabla 164 (`/P`)**: la entrada es *una referencia indirecta al objeto de
  página con el que la anotación está asociada*. Es un único objeto de página; la
  gramática no ofrece manera de expresar «asociada a tres páginas».
- **§7.7.3.3, Tabla 30 (`/Annots`)**: el array de una página *debe contener referencias
  indirectas a todas las anotaciones asociadas con esa página*.

Las dos afirmaciones juntas hacen del array de `/Annots` de la página y del `/P` de la
anotación **las dos mitades de una misma relación**, que la norma trata como uno-a-uno. Un
objeto con `/P` a la página 1 que aparece en el `/Annots` de la 2 y la 3 rompe esa
correspondencia: para las páginas 2 y 3 el documento afirma una asociación que la propia
anotación desmiente.

A eso se suma que `/AcroForm /Fields` repite el mismo objeto (**§12.7.2, Tabla 218**: array
de referencias a los campos raíz del documento). No son tres campos con el mismo nombre
—que sería el caso legítimo de los botones de radio, §12.7.3.1— sino **una entrada
duplicada tres veces**. Poppler lo resuelve como un solo campo (`Signature #1`), que es lo
sensato, pero nada en la norma dice qué debe hacer un lector con un `/Fields` con
duplicados.

> **Cita no verificada contra el texto original.** Las tres secciones se citan de memoria
> de PDF 32000-1:2008; no hay copia de la norma en este equipo. Los números de sección y
> tabla son los que corresponden, pero la redacción literal no se ha contrastado.

### El `/Rect`: la limitación dura

`/Rect[100 100 300 160]`, y **es forzosamente el mismo en todas las páginas**, porque es
literalmente el mismo objeto. No es una casualidad de esta prueba: **con este mecanismo es
imposible** colocar el recuadro en coordenadas distintas según la página. Lo mismo vale
para la apariencia: `/AP << /N 6 0 R >>` es un único Form XObject
(`/BBox[0 0 200 60]`, que envuelve al `/FRM 5 0 R`), así que el dibujo también es
idéntico en las tres.

AutoFirma es consciente y lo resuelve recortando contra la **primera** página de la lista,
no contra cada una (`clienteafirma/afirma-crypto-pdf/src/main/java/es/gob/afirma/signers/pades/PdfUtil.java:607-633`):

```java
// Comprobamos que la firma se pueda estampar en al menos una de las paginas
for (final Integer page : pages) {
    final Rectangle pageSize = pdfReader.getPageSizeWithRotation(page.intValue());
    if (pageSize.getWidth() <= signaturePosition.getLeft()
            || pageSize.getHeight() <= signaturePosition.getBottom()) {
        pagesList.remove(page);          // ← la pagina se cae de la lista, en silencio
    }
}
…
// Redimensionamos el area de firma para que quede completamente dentro de la
// primera pagina en la que se vaya a imprimir
final int firstPage = pagesList.get(0).intValue();
```

Dos consecuencias en un documento con páginas de tamaños distintos: las páginas donde la
esquina inferior izquierda no cabe **desaparecen de la lista sin aviso** (solo un `WARNING`
si el rango era inválido, `PdfUtil.java:725`), y el rectángulo se recorta al tamaño de la
primera, con lo que en una página más pequeña puede salirse.

Y el `/P` sale del primero de la lista, no del conjunto
(`PdfSessionManager.java:532`):

```java
sap.setVisibleSignature(signaturePositionOnPage,
        pages != null ? pages.get(0).intValue() : pdfReader.getNumberOfPages(), null);
…
sap.preClose(exc, signTime, pages);      // :598 — la lista entera, para los /Annots
```

Es decir: `setVisibleSignature` fija `/P` con **una** página, y el `preClose` de tres
argumentos del iText propio de AutoFirma
(`com.aowagie.text.pdf.PdfSignatureAppearance.preClose(HashMap, Calendar, List<Integer>)`,
presente en `afirma-lib-itext-1.7.jar` y **ausente** del iText original) es quien mete la
referencia en el `/Annots` de las demás.

La lista de páginas la resuelve `PdfUtil.getPages` (`PdfUtil.java:696-740`), que acepta
`all`, `append`, números sueltos separados por coma, rangos con guion y **números
negativos contados desde el final** (`PdfExtraParams.java:307-322`); `signaturePages` es un
alias exacto de `signaturePage`, «para evitar confusiones por parte de los integradores»
(`PdfExtraParams.java:326-330` (la constante, en `:330`)).

---

## Qué hace un lector real

Poppler, que es la puerta de calidad del proyecto (#61):

```
$ pdftotext -f 3 -l 3 pages-all.pdf -
Pagina 3 de 3, sondeo rfirma 116

Firmado por: prueba rfirma
#116
signaturePages=all
```

El texto de la rúbrica sale en las **tres** páginas de `pages-all.pdf` y en las **dos**
primeras de `pages-1-2.pdf`; en la página 3 de `pages-1-2.pdf` solo sale el texto propio de
la página.

Rasterizando a 72 dpi y contando píxeles no blancos (`v < 200`) dentro del rectángulo del
widget —`/Rect[100 100 300 160]` en espacio de usuario es `x∈[100,300]`, `y∈[682,742]` en
la imagen, porque `842 − 160 = 682`—:

| Página | `three-pages.pdf` (sin firmar) | `pages-all.pdf` | `pages-1-2.pdf` |
| --- | --- | --- | --- |
| 1 | 0 | **776** | **765** |
| 2 | 0 | **776** | **765** |
| 3 | 0 | **776** | **0** |

Poppler **pinta el widget en cada página que lo lista en su `/Annots` e ignora el `/P`**.
Las cifras idénticas página a página son la confirmación empírica de que el dibujo es el
mismo objeto: no hay ni un píxel de diferencia.

Ni `pdftoppm` ni `pdftocairo` emiten un solo aviso por `stderr` sobre esta estructura.

**`qpdf --check` y `mutool clean -s` no se han podido ejecutar: ninguna de las dos
herramientas está instalada en este equipo.** Quedan pendientes; son las dos que con más
probabilidad tendrían algo que decir sobre el `/Fields` duplicado.

---

## Qué hace hoy el puente de rfirma con la geometría

La geometría de una sola página ya está resuelta y documentada; aquí solo interesa **dónde
está el escalar que habría que convertir en lista**. La conversión de píxeles del visor a
`extraParams`, incluida la corrección por `/Rotate` que no se ve venir, vive en
[`coordenadas-recuadro-pades.md`](coordenadas-recuadro-pades.md); el ciclo trifásico con
firma visible, en [`firma-visible-trifasica.md`](firma-visible-trifasica.md). No se repiten.

El recorrido, de la interfaz al puente:

| Punto | Fichero:línea | Campo de página |
| --- | --- | --- |
| Visor (TS) | `rfirma-app/src/viewer/signatureBox.ts:40-42` | `SignaturePlacement { page: number; rect }` |
| Orden de firma (TS) | `rfirma-app/src/signing/flow.ts:55-64` | `placement.page: number` |
| Entrada del comando | `rfirma-app/src-tauri/src/commands/mod.rs:314-328` | `PlacementOrder { page: u32, … }` |
| Página del documento | `rfirma-app/src-tauri/src/signing/placement.rs:127-131` | `Page { number: u32, … }` |
| Caja resultante | `rfirma-app/src-tauri/src/signing/config.rs:82-92` | **`SignatureBox { pub page: u32, … }`** |
| Memoria entre sesiones | `rfirma-app/src-tauri/src/memory/state.rs:39` | `visible_signature: Option<SignatureBox>` |
| A `extraParams` | `rfirma-app/src-tauri/src/signing/config.rs:96-111` | `PAGE_KEY → page.to_string()` |
| Al puente | `rfirma-app/src-tauri/src/ffi.rs:284-293` | `PreSignRequest.extra_params: &str` (ya es texto) |

`SignatureBox` es el tipo canónico: `Serialize + Deserialize` (`config.rs:81`), un `page:
u32` 1-based y las cuatro esquinas `i32`. Las claves literales están en `config.rs:21-27`
y se agrupan bajo `Setting::Geometry` en `config.rs:61-67`.

**El lado Java no toca la geometría en absoluto.** `NativeBridge.java:89` recibe los
`extraParams` como un blob de texto en formato `java.util.Properties` y se limita a
parsearlo (`:95`, vía `SessionStamp.parseParams`); `PadesBridge.preSign` los pasa tal cual
a `PAdESTriPhasePreProcessor` (`PadesBridge.java:70-90`). No hay ni una mención a
`signaturePage` en `rfirma-native-bridge/src/main/java/`. **El puente es un pasamanos.**

Y `signaturePages` (plural) **no aparece en ningún sitio del repo de rfirma**, ni ninguna
noción de multipágina. Todo el modelo es monopágina y está documentado como tal
(`commands/mod.rs:315-317`, `config.rs:37`).

---

## Qué exigiría que `signaturePages` viajara por el sello del ADR-0016

El [ADR-0016](../adr/0016-sello-de-sesion-una-sola-invariante.md) mete los `extraParams`
**efectivos** —los que la prefirma dejó mutados— dentro de un bloque opaco que Rust
conserva sin interpretar y que la postfirma compara byte a byte antes de firmar
(`SessionStamp.java:64-69` para los campos, `:167-181` para el `encode()`, con cada
extraParam bajo el prefijo `P.` y ordenado alfabéticamente).

Esto tiene una consecuencia agradable: **el sello no necesita ningún cambio.** La página ya
viaja dentro como `P.signaturePage`; si mañana es `P.signaturePages=1-2`, el sello la
transporta igual, se ordena igual y se compara igual. `SessionSeal` en el lado Rust
(`rfirma-app/src-tauri/src/signing/session_seal.rs:25`) es un newtype opaco sobre un
`String`, sin campos: no sabe ni quiere saber qué hay dentro.

Lo que sí cambiaría, en orden de menor a mayor riesgo:

1. **`SignatureBox.page`** (`config.rs:84`) deja de ser `u32`. La forma con menos daño es
   `pages: PageSelection`, un tipo propio que se serialice a la cadena que AutoFirma
   entiende (`all`, `1,2,5`, `2-5`, negativos). **No** un `Vec<u32>` en crudo: el visor
   necesita distinguir «todas» de «las tres que hay ahora mismo», porque el documento
   puede cambiar. Y el `Serialize/Deserialize` de este tipo **es formato persistido**
   ([ADR-0010](../adr/0010-memoria-entre-sesiones.md), `memory/state.rs:39`), así que hay
   que decidir la migración de los estados ya guardados con `page: u32`.
2. **La clave emitida.** `PAGE_KEY` (`config.rs:21`) pasa de `signaturePage` a
   `signaturePages`, o se emiten las dos. Ojo: `Setting::Geometry` (`config.rs:61-67`) hoy
   tiene pruebas que garantizan que no se emite ninguna clave fuera de las cinco; esas
   pruebas hay que tocarlas a conciencia, no a la brava.
3. **`PlacementOrder`** (`commands/mod.rs:314-328`) y su `signature_box()`
   (`:332-348`), más `Page` (`placement.rs:127-131`) y el `Ok(SignatureBox { page:
   self.number, … })` de `placement.rs:225-236`. Aquí está el nudo real: hoy el `media_box`
   y la `rotation` que la conversión necesita **son de una página concreta**
   (`App.tsx:333-340` hace `pdf.getPage(placement.page)`). Con varias páginas hay que
   decidir contra cuál se convierten las coordenadas —AutoFirma usa la primera de la
   lista, `PdfUtil.java:625-632`— y qué se hace si las demás tienen otro tamaño u otra
   rotación. **Eso no es un cambio de tipo, es una decisión de producto.**
4. **El Java: nada.** `NativeBridge` y `PadesBridge` no interpretan la geometría, y
   `PdfSessionManager` ya acepta `signaturePages` desde AutoFirma 1.9.1.
5. **Las guardias del ADR-0011** (`commands/mod.rs:1308` y `:1381`): **ninguna toca**, si
   el cambio se limita a lo anterior. La lista fija está en `:1312`:

   ```rust
   let outputs = ["struct CertificateView", "struct SignedDocumentView"];
   ```

   y la hermana en `:1384` cubre `"struct OpenedDocumentView"`. `PlacementOrder` y
   `SignatureBox` son tipos de **entrada** (`Deserialize`), no de salida, y por eso no
   están en ninguna de las dos listas. **Pero**: si la multipágina obliga a devolver al
   frontend un tipo de salida nuevo —por ejemplo un resumen de en qué páginas se estampó—
   ese tipo necesita su propia entrada en la lista, porque la guardia recorre nombres
   fijos y un tipo nuevo pasa sin que nadie lo mire (está en el `CLAUDE.md` del proyecto).

---

## Recomendación para la ficha 24

**No replicar el widget. Si rfirma ofrece «varias páginas», que sea con un campo de firma
por página; y para v0.3, lo más defendible es no ofrecerlo todavía.**

El razonamiento, con lo medido:

- **A favor de replicar**: funciona. `pdfsig` lo valida, poppler lo pinta en todas las
  páginas, el documento queda entero cubierto por el `/ByteRange`, y es exactamente lo que
  AutoFirma lleva años emitiendo, así que el parque de PDF con esta estructura ya existe y
  los lectores conviven con ella. El coste de implementación es bajo: el puente no cambia y
  el sello del ADR-0016 tampoco.
- **En contra**: el `/Rect` compartido es una limitación que el usuario va a notar el
  primer día. La ficha 24 vive en el mismo grupo que «redimensionar y anclar el recuadro»
  (fichas 6 y 8): la funcionalidad hermana es *colocar el recuadro donde quieras*, y
  replicar el widget dice justo lo contrario —*en el mismo sitio en todas, y si una página
  es más pequeña, ahí te las apañas* (`PdfUtil.java:625-632`)—. Es una función que hay que
  explicar con una nota al pie, y las funciones así envejecen mal.
- **Y sobre todo**: rfirma no tiene la deuda de AutoFirma. AutoFirma replica el widget
  porque su iText fue parcheado para eso hace años; rfirma parte de cero y **no gana nada**
  heredando una estructura que la norma no contempla, en una aplicación cuyo argumento es
  ser el reemplazo limpio.

La alternativa correcta —**un campo de firma por página**— es estructuralmente
irreprochable, permite un `/Rect` distinto por página, y es lo que hacen los productos
comerciales. Pero tiene un coste que este sondeo **no ha medido** y que hay que medir antes
de comprometerse: en PAdES, varios campos de firma sobre el mismo documento normalmente
significan **varias firmas** (una por campo), o bien un solo `/V` compartido entre campos
—que es otra irregularidad, distinta y probablemente peor—. Convertir «estampar en 3
páginas» en «3 firmas del mismo certificado» cambia lo que `pdfsig` cuenta, lo que VALIDe
informa y lo que el usuario entiende. **Eso hay que sondearlo antes de escribir código.**

De ahí la recomendación para v0.3: **ficha 24 sin «todas las páginas»**. Que v0.3 traiga
redimensionar y anclar el recuadro en **una** página (fichas 6 y 8), que es lo que el
usuario pide de verdad, y que el multipágina espere a un sondeo propio sobre el coste de un
campo por página. Si algún día el multipágina entra por presión de compatibilidad con
AutoFirma, replicar el widget es una salida **tolerable** —está medida y valida— pero es la
segunda opción, no la primera.

---

## Lo que no se ha medido

Lo digo en voz alta porque decide cuánto pesa la recomendación:

- **`qpdf --check` y `mutool clean -s`**: no están instalados. Son las dos herramientas con
  más probabilidad de denunciar el `/Fields` con entradas duplicadas y el `/P` que no
  cuadra con los `/Annots`. Sin ellas, «ninguna herramienta protesta» significa en realidad
  «poppler no protesta».
- **VALIDe**. Es la tarea [#118](https://github.com/sgomez/rfirma/issues/118) y usa
  **estos mismos dos PDF**. Es el veredicto que de verdad importa para el usuario español,
  y hasta que llegue, esta ficha no está cerrada del todo.
- **Adobe Acrobat Reader, Foxit y los visores de navegador** (`pdf.js`, el de Chrome). No
  hay ninguno disponible sin GUI aquí. Interesa sobre todo si alguno marca el documento
  como modificado o si alguno pinta el recuadro **solo** en la página del `/P` —que sería
  el comportamiento estrictamente conforme, y dejaría dos de las tres estampaciones
  invisibles—.
- **Páginas de tamaños o rotaciones distintas.** Las tres del banco son A4 sin rotar. El
  recorte contra la primera página (`PdfUtil.java:625-632`) solo se ve en un documento
  mixto, y es justo donde se espera el problema.
- **Cofirma sobre un documento así**: añadir una segunda firma a `pages-all.pdf` y ver si
  el segundo campo convive con el primero replicado. Fuera del alcance del ticket.
- **El coste real de «un campo por página»**, discutido arriba. Es lo que decidiría si la
  alternativa que recomiendo es viable o si «todas las páginas» es directamente
  irrenunciablemente feo en PAdES.

---

## Reproducir

Sin volver a firmar, que es lo normal:

```bash
cd docs/research/assets/replicated-widget-driver

pdfsig pages-all.pdf
pdfsig pages-1-2.pdf

python3 objstm.py pages-all.pdf          # el widget y las paginas, inflando el /ObjStm
python3 inspect.py pages-all.pdf         # el /AcroForm y el /Type /Sig, en texto claro

for p in 1 2 3; do pdftotext -f $p -l $p pages-all.pdf -; done
pdftoppm -r 72 -png pages-all.pdf /tmp/all      # y mirar las tres
```

Firmar de nuevo requiere GraalVM 25.3.4 (`GRAALVM_HOME`), los artefactos de AutoFirma 1.9.1
en `~/.m2` (`mvn clean install` en el clon de `clienteafirma`), y **corregir la ruta `P12=`
de `run.sh`** al `.p12` del kit de la FNMT en
`~/.local/share/rfirma-test-certs`. Después:

```bash
mvn -q package        # deja target/probe-1.jar y target/cp.txt
./run.sh
```
