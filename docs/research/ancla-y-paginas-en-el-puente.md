# El ancla y las páginas: qué acepta el puente y qué se le escapa

Sondeo del [#150](https://github.com/sgomez/rfirma/issues/150), hijo del mapa
[#148](https://github.com/sgomez/rfirma/issues/148) (rfirma v0.3, «el recuadro»). Decide la
parte **no visual** de las fichas 8 (anclar el recuadro) y 24 (una página, algunas o todas):
por dónde viaja la página hasta el puente, qué le hace el sello de sesión del
[ADR-0016](../adr/0016-sello-de-sesion-una-sola-invariante.md) y dónde queda la degradación
silenciosa que el **ID-22** rechaza.

**Respuesta corta, en cuatro líneas.**

1. **El puente no hay que tocarlo.** No filtra claves: `signaturePages` con toda su gramática
   —`all`, `3`, `1-3,-3--1`, `-1`, `append`— llega hoy a la prefirma y produce el widget
   replicado, sin una sola línea de Java nueva. Medido, no deducido.
2. **El sello tampoco.** `signaturePages` **no sale mutado** de la prefirma: el sello lo lleva
   idéntico a como se envió, y la postfirma lo reconstruye de ahí. La invariante del ADR-0016
   ya cubre el ancla por existir.
3. **Los dos convenios son incompatibles y no se cruzan.** `signaturePages` es de la **firma
   visible**; `imagePage` es del **sello de imagen sin firmar**, que rfirma no usa y no va a
   usar. `0` significa «primera página» en uno y «todas» en el otro. La confusión que anuncia
   el informe es real en AutoFirma, pero en rfirma no llega a existir mientras `imagePage`
   siga fuera del contrato.
4. **La degradación silenciosa está más abajo de lo que decía el ticket, y ya está abierta
   hoy.** El «quita el parámetro y reintenta» de AutoFirma vive en su interfaz Swing y **no
   es alcanzable desde el camino trifásico**. Lo que sí es alcanzable —y lo que hereda
   rfirma— es peor: `PdfUtil.getPages` **no lanza nunca**. Recorta los números fuera de
   rango, ignora la basura con un `WARNING` y, si no queda nada, firma en la **última
   página**. Ya pasa con el `signaturePage` singular que rfirma envía hoy: `signaturePage=99`
   sobre un PDF de 3 páginas firma en la 3 y devuelve éxito. **No hay excepción que capturar:
   la única defensa posible es validar en Rust antes de llamar.**

---

## El banco

Todo se midió **contra el puente de rfirma**, no contra AutoFirma directamente: es la
frontera que interesa. El banco **no se fusiona**: se queda en la rama
`research/signature-pages-bridge`, que no se borra, en `docs/research/assets/signature-pages-probe/`:

| Fichero | Qué es |
| --- | --- |
| `SignaturePagesProbeTest.java` | El sondeo. Se ejecuta soltándolo en `rfirma-native-bridge/src/test/java/es/gob/afirma/nativebridge/` |
| `salida.txt` | La salida literal de la corrida que sostiene las tablas de abajo |

No se versiona dentro de `src/test/` a propósito: no afirma nada, imprime. Una prueba que no
prueba no debe estar en la suite.

```
JAVA_HOME=~/.sdkman/candidates/java/25.3.4+1.r25-graalce \
  mvn -o -f rfirma-native-bridge/pom.xml test -Dtest=SignaturePagesProbeTest \
      -Dcheckstyle.skip=true -Dpmd.skip=true -Dspotbugs.skip=true
```

Ciclo trifásico completo en JVM —`PadesBridge.preSign` → PKCS#1 con la JCE →
`PadesBridge.postSign`— sobre un PDF generado al vuelo de 3 páginas A4 (595×842), recuadro
`100,100–300,200`, y el certificado **de pruebas de la FNMT** del kit versionado
([`token-pkcs11-pruebas.md`](token-pkcs11-pruebas.md)). El PDF firmado se relee con
`PdfReader` y se cuentan las anotaciones por página, el tamaño de cada página, los campos de
firma y el `/Rect` de cada widget.

El oráculo de código es AutoFirma en `/home/sergio/Developer/SideProjects/clienteafirma`,
commit `0d7f3cf`. Las rutas Java que se citan son de ahí.

---

## 1. Qué acepta hoy el puente

### La geometría no viaja en JSON

Conviene corregir el enunciado del ticket: **de ida no hay JSON**. La petición cruza la
frontera FFI con los `extraParams` serializados como un bloque `java.util.Properties` en
texto (`rfirma-app/src-tauri/src/signing/properties.rs:30`, `to_java_properties`), dentro de
`PreSignRequest` (`rfirma-app/src-tauri/src/ffi.rs:285-294`). El JSON es solo la **respuesta**
(`NativeBridge.java:99-103`: `{"ok":true,"session":…,"pre":…,"stamp":…}`), y en ella no hay ni
geometría ni página.

Hoy rfirma emite cinco claves de geometría (`rfirma-app/src-tauri/src/signing/config.rs:21-25`):

```
signaturePage                            ← singular
signaturePositionOnPageLowerLeftX / Y
signaturePositionOnPageUpperRightX / Y
```

`signaturePages` (plural) **no aparece en ningún punto del código de rfirma**.

### El puente es un pasamanos: no hay lista blanca

`SessionStamp.parseParams` es un `Properties.load` en crudo
(`rfirma-native-bridge/src/main/java/es/gob/afirma/nativebridge/SessionStamp.java:314-326`) y
`PadesBridge.preSign` se lo pasa tal cual al `PAdESTriPhasePreProcessor`
(`PadesBridge.java:70-90`). Ni un `switch`, ni un `Set` de claves permitidas, ni una mención
de `signaturePage` en todo `rfirma-native-bridge/src/main/java/`.

**Conclusión: `signaturePages` pasaría hoy sin tocar Java.** Confirmado además por medición:
los diez valores de la tabla siguiente cruzaron el puente y produjeron el PDF que se
esperaba.

La lista cerrada de cinco ajustes del [#31](https://github.com/sgomez/rfirma/issues/31) no es
una lista blanca del puente: es una decisión de **qué emite Rust**, sostenida por el enum
`Setting` y sus pruebas (`config.rs:34-72`, `closes_the_configuration_at_five_settings`). La
puerta está abierta; lo que la cierra es el lado Rust.

### Qué significa cada valor, medido

PDF de 3 páginas A4. «Anotaciones» = anotación de widget por página del PDF resultante.

| `signaturePages` | Páginas con widget | Campos de firma | Nota |
| --- | --- | --- | --- |
| *(ausente)* | 3 | `Signature1` | por omisión, la **última** |
| `all` | 1, 2, 3 | `Signature1` | widget replicado, **un solo campo** |
| `2` | 2 | `Signature1` | |
| `1-3,-3--1` | 1, 2, 3 | `Signature1` | unión deduplicada |
| `-1` | 3 | `Signature1` | negativo = desde el final |
| `append` | 4, en un documento que pasa a tener **4 páginas** | `Signature1` | inserta página en blanco |
| `0` | 1 | `Signature1` | **no** es «todas» |
| `99` | 3 | `Signature1` | **recortado en silencio** |
| `2-99` | 2, 3 | `Signature1` | **recortado en silencio** |
| `pepe` | 3 | `Signature1` | **ignorado en silencio** |

La gramática que hay detrás está en `PdfUtil.getPages`
(`afirma-crypto-pdf/…/pades/PdfUtil.java:696-740`) y en `normalizePage` (`:535-550`):

```java
int page = Integer.parseInt(pageStr);
if (page < 0)          { page = page + totalPages + 1; }  // -1 => última
if (page <= 0)         { page = 1; }                      // 0 o negativo excesivo => primera
if (page > totalPages) { page = totalPages; }             // exceso => última
```

Tres detalles que el javadoc de `PdfExtraParams` **no** dice
(`afirma-crypto-pdf-common/…/pades/common/PdfExtraParams.java:306-331`):

* **`append` solo cuenta como primer token** (`PdfUtil.java:711-713` mira `pagesStr[0]`), y el
  resto de la lista se descarta. El propio diálogo de AutoFirma emite `"3,append"`
  (`SignPdfUiPanel.java:530-534`), que por tanto **no** crea página nueva: `append` acaba en
  `getPagesRange`, lanza `IncorrectPageException` y se ignora con un warning. Es un fallo del
  original. `append` queda fuera de v0.3 por decisión del mapa, así que no nos afecta —pero
  conviene no copiar el formato.
* **`all` tampoco se combina**: mismo `pagesStr[0]`, el resto se ignora.
* **El javadoc no documenta `append`** en absoluto.

### Precedencia entre singular y plural

`PdfUtil.getPages:699-703` mira primero `signaturePages` y solo si falta cae en
`signaturePage`. Medido: con `signaturePage=1` y `signaturePages=3` el widget sale en la
**3**; con `signaturePage=1` y `signaturePages=all`, en las **tres**. El plural gana siempre.

Esto abarata la migración: si rfirma empieza a emitir `signaturePages`, no hay que quitar el
singular en el mismo paso para que funcione. Pero conviene quitarlo igual, porque un sello con
las dos claves describe una configuración que solo se entiende leyendo el código de AutoFirma.

---

## 2. `signaturePages` frente a `imagePage`: dos convenios que no se tocan

El aviso del informe es correcto, y la conclusión práctica es que **no nos incumbe**.
`imagePage` es de `PdfPreProcessor.addImage`
(`afirma-crypto-pdf/…/pades/PdfPreProcessor.java:221-259`), el **sello de imagen previo a la
firma**: una imagen estampada en el PDF que no forma parte de la firma visible. Se invoca
desde `PdfSessionManager.java:435` y su UI es otra distinta
(`SignPdfUiPanelStamp.java:535`).

| | `signaturePages` (firma visible) | `imagePage` (sello sin firmar) |
| --- | --- | --- |
| Todas las páginas | `all` | **`0`** |
| Última | `-1` | `-1` |
| `-2`, `-3` | penúltima, antepenúltima | **no soportado** (bucle vacío o absurdo) |
| Listas y rangos | sí | **no**: `Integer.parseInt` directo |
| Página nueva | `append` | no existe |
| Valor `0` | **primera página** | **todas** |
| Valor inválido | se ignora con `WARNING` | **`IOException`, aborta la firma** |

La colisión de `0` es real y está documentada en las dos direcciones
(`PdfUtil.java:56-69` con `NEW_PAGE = 0`, y `PdfPreProcessor.java:43-45` con `ALL_PAGES = 0`).

**Qué toca a rfirma: solo `signaturePages`.** La rúbrica de rfirma es
`signatureRubricImage` (`PdfSessionManager.java:108`), que **no tiene noción de página
propia**: se pinta dentro del recuadro de la firma visible y hereda sus páginas. `imagePage`
no está en la lista cerrada del #31 y no hay ninguna ficha de v0.3 que lo pida. Mientras siga
así, los dos convenios no coexisten en rfirma y no hay nada que reconciliar en la interfaz.

Lo que sí hay que evitar es **importar el convenio equivocado por descuido**: si algún día la
interfaz ofrece «todas las páginas» como un valor numérico `0`, estará hablando el idioma de
`imagePage` mientras el puente lee el de `signaturePages`, y `0` firmará en la primera página
sin decir nada. La forma de cerrarlo es que el tipo de Rust no sea un número: un enum
(`Page(n)` / `All` / `Last`) que se serializa al literal correcto en un único sitio.

---

## 3. Qué le hace el sello de sesión: nada, y eso es la buena noticia

El ADR-0016 sella los `extraParams` **efectivos**, no los enviados, porque
`PdfSessionManager:150-156` reescribe `signatureSubFilter` cuando hay política o perfil
baseline y `PAdESTriPhaseSigner:174` **no clona** el `Properties`:

```java
final Properties extraParams = xParams != null ? xParams : new Properties();
```

El puente aprovecha esa no-clonación como canal de salida: relee el objeto justo después de
la prefirma (`PadesBridge.java:108-110`, «`// extraParams EFECTIVOS: el objeto que acaba de
mutar la prefirma.`»).

**La pregunta del ticket era si `signaturePages` sale mutado. No sale.** Dos evidencias
independientes:

* **Por código.** La línea 155 de `PdfSessionManager` es la **única** escritura sobre
  `extraParams` de toda la clase (`setProperty`/`put`/`remove`, verificado por grep), y toca
  `signatureSubFilter`. La resolución de páginas vive en una variable local
  (`List<Integer> pages`, `PdfSessionManager.java:383-408`) que **no se persiste** en ningún
  sitio.
* **Por medición.** En los diez casos de la tabla, las claves del sello son exactamente las
  enviadas y `signaturePages` sale con el mismo texto que entró —incluidos `99` y `pepe`—.

Consecuencia: **el ADR-0016 no necesita enmienda para el ancla**. El sello transporta la
página como un `P.signaturePage=…` cualquiera; si mañana es `P.signaturePages=1-2`, la
transporta igual, en claro y en orden alfabético (`SessionStamp.java:166-182`).

Y el mecanismo que hace falta que funcione ya funciona: **la postfirma no acepta
`extraParams` del llamante**, los saca del sello (`PadesBridge.java:198`,
`preProcessPostSign(pdf, stamp.algorithm(), chain, stamp.extraParams(), session)`). La
resolución de `signaturePages` se **repite de cero** en la postfirma —la sesión trifásica solo
lleva `NEED_PRE`, `PRE`, `TIME` y `PID`
(`PAdESTriPhasePreProcessor.java:124-128`, comprobado volcando la sesión: no hay ni rastro de
página)— y sale idéntica porque parte del mismo texto sellado. Si alguien pudiera cambiar
`signaturePages` entre fases, el resultado sería un PDF con «Digest Mismatch»; el sello lo
impide desde Rust (`session_seal.rs:40`, `verify_unchanged`) y desde Java
(`PadesBridge.java:142-172`).

**Lo único que hay que enmendar del ADR-0016 es la prosa**, si acaso: hoy justifica el sellado
de los efectivos con un solo ejemplo (`signatureSubFilter`), y conviene que diga que el ancla
también viaja dentro, precisamente porque no se muta.

---

## 4. La degradación silenciosa: dónde está de verdad

El ticket y el mapa dicen que «AutoFirma quita el parámetro, reintenta y la firma acaba en la
última sin avisar». Es cierto, pero **está en otra capa y describe otro caso**, y la parte que
nos toca es peor.

### El reintento que sí existe, y que no nos alcanza

El «quitar y reintentar» está en la aplicación, no en la biblioteca:

* `afirma-simple/…/ui/SignPanelSignTask.java:636-658` — captura `IncorrectPageException` y
  `InvalidSignaturePositionException`, clona el `Properties`, hace `remove` de
  `signaturePage`, `signaturePages` (y, en el segundo caso, de las cuatro coordenadas) y
  vuelve a llamar a `signData`.
* `afirma-simple/…/CommandLineLauncher.java:855-859`, con
  `removeSignaturePageProperties` (`:1198-1213`) — lo mismo para la línea de órdenes.

Un `grep` de esas dos excepciones fuera de los tests solo devuelve `afirma-crypto-pdf` y
`afirma-simple`: **no hay ningún `catch` en `afirma-server-triphase-signer-core` ni en
`PAdESTriPhasePreProcessor`** (cuyo único `catch` es `InvalidPdfException`,
`PAdESTriPhasePreProcessor.java:99-114`). En el camino trifásico, que es el de rfirma, esas
excepciones **suben como error de la prefirma**. Ese reintento no lo heredamos.

### La degradación que sí heredamos: no hay excepción

`PdfUtil.getPages` **no lanza al llamante**. Nunca. Lo que hace con lo que no cuadra:

| Entrada | Qué hace | Dónde |
| --- | --- | --- |
| Número mayor que el total | lo **recorta** a la última | `normalizePage:548-549` |
| Número `0` o negativo excesivo | lo **recorta** a la primera | `normalizePage:544-545` |
| Token no parseable, o rango invertido | `IncorrectPageException` **capturada y registrada como `WARNING`**, token descartado | `getPages:721-728` |
| Lista que se queda vacía | `pages.add(totalPages)` — **la última página** | `getPages:731-737` |

Medido en el puente, con la clave que rfirma **envía hoy**:

| Enviado | Resultado |
| --- | --- |
| `signaturePage=99` sobre 3 páginas | widget en la página **3**, `preSign` y `postSign` con éxito |
| `signaturePage=0` | widget en la **1** |
| `signaturePage=pepe` | widget en la **3** |

**Es decir: la segunda puerta del ID-22 ya está abierta, sin multipágina y sin
`signaturePages`.** El ID-22 —«un recuadro fuera de página se recorta en silencio y la firma
sale válida igual, así que rFirma lo impide antes de firmar»— hoy solo está implementado para
el **rectángulo** (`Page::check_fits`, `signing/placement.rs:252-277`, con el fallo
`boxOutOfPage` en `commands/orders.rs:67`). El **número de página no lo valida nadie**:
`PlacementOrder.page: u32` (`commands/orders.rs:35`) llega del frontend y se copia hasta
`SignatureBox { page }` sin compararse con ningún contador; `page_count`, `total_pages` y
`numPages` no aparecen en todo `src-tauri/src/` (el `pageCount` solo existe en el frontend,
`rfirma-app/src/App.tsx:533,547`).

### Y una tercera puerta que abre el multipágina

`PdfUtil.correctPositionSignature` (`PdfUtil.java:607-632`) hace dos cosas silenciosas antes
de firmar:

```java
if (pageSize.getWidth() <= signaturePosition.getLeft()
        || pageSize.getHeight() <= signaturePosition.getBottom()) {
    pagesList.remove(page);          // descarta la página, sin avisar
}
…
if (signaturePosition.getTop()   > firstPageSize.getTop())   { … }   // recorta el recuadro
if (signaturePosition.getRight() > firstPageSize.getRight()) { … }   // contra la PRIMERA página
```

Solo lanza (`InvalidSignaturePositionException`) si se queda **sin ninguna** página. Y el
recorte se calcula contra la **primera página de la lista**, no contra cada una.

Medido con un PDF de dos páginas de tamaños distintos (1 = A4 595×842, 2 = 200×200) y el
recuadro `100,100–300,200`:

| Enviado | `/Rect` resultante | Lectura |
| --- | --- | --- |
| `signaturePages=2` | `[2, 100, 100, **200**, 200]` | **recortado en silencio** contra la página pequeña |
| `signaturePages=all` | `[1, 100,100,300,200]` y `[2, 100,100,**300**,200]` | el recorte se hizo contra la página 1; en la 2 el widget **se sale del papel** y nadie protesta |

Este segundo caso es específico del multipágina y **no lo puede detectar la comprobación
actual del ID-22**, que mira una sola página. Con «todas las páginas» y un documento de
páginas heterogéneas —muy común en un expediente escaneado con un anexo apaisado— el recuadro
puede quedar fuera del papel en unas cuantas.

### Qué se le puede pedir a Rust

**Todo, porque del puente no viene nada.** La respuesta de la prefirma no dice en qué páginas
acabó el widget: la sesión solo lleva `NEED_PRE`, `PRE`, `TIME` y `PID` (volcada en el banco),
y el sello solo devuelve el texto que se envió. No hay ningún valor de retorno que comparar,
ni ninguna excepción que capturar en el caso normal. **Detectar la degradación después de
firmar exigiría releer el PDF firmado y contar anotaciones**, que es cerrar la puerta con el
caballo fuera.

De ahí, tres cosas que el spec de v0.3 tiene que nombrar y que este sondeo deja medidas:

1. **Rust conoce el número de páginas y valida el destino antes de llamar.** El frontend ya lo
   tiene (`pageCount`); el backend no. Sin eso, cualquier `page` fuera de rango se firma en la
   última con cara de éxito. Esto **arregla también un agujero que existe hoy**, antes del
   multipágina.
2. **La comprobación del recuadro deja de ser contra «la página» y pasa a ser contra
   todas las páginas de destino.** `Page::check_fits` se aplica a cada página del conjunto, no
   a una. Con el widget replicado el `/Rect` es forzosamente el mismo en todas
   ([`recuadro-replicado-pdfsig.md`](recuadro-replicado-pdfsig.md)), así que «cabe en todas»
   es una condición que la interfaz puede comprobar y explicar antes de firmar.
3. **La página deja de ser un `u32` y pasa a ser un tipo con nombre.** `Todas`, `Última` y
   `Página(n)` no son tres números: son tres cosas, y el `0` de `imagePage` demuestra lo que
   cuesta confundirlas. La serialización al literal de `signaturePages` ocurre en un solo
   sitio, junto a las otras cuatro claves de `Setting::Geometry`.

Nada de esto necesita Java nuevo, ni un ADR nuevo, ni tocar el sello.

---

## Lo que no se ha medido

* **La replicación del widget en sí.** Ocurre dentro de
  `PdfSignatureAppearance.preClose(exc, signTime, pages)` del fork de iText
  (`es.gob.afirma.lib:afirma-lib-itext`), cuyo código **no está en el repositorio de
  AutoFirma**. Lo que se sabe de su resultado ya está medido en
  [`recuadro-replicado-pdfsig.md`](recuadro-replicado-pdfsig.md) y en el
  [#118](https://github.com/sgomez/rfirma/issues/118) (VALIDe).
* **`imagePage` empíricamente.** Solo se ha leído su código. Como no está en el contrato de
  rfirma ni se propone meterlo, medirlo no cambiaría ninguna decisión.
* **`append` con rúbrica y con documentos reales.** El mapa lo deja fuera de v0.3; se midió
  solo lo justo para confirmar que el puente lo acepta y que el formato `"N,append"` del
  original no funciona.
* **El comportamiento en `librfirma_crypto.so`**, la biblioteca de Native Image. Todo esto se
  midió en JVM. Nada de lo tocado depende de AWT ni de reflexión dinámica —es aritmética sobre
  `Properties` y objetos de iText que ya se ejercitan en la firma visible actual—, pero la
  equivalencia no se ha vuelto a comprobar aquí.

## Discoveries

* `docs/research/recuadro-replicado-pdfsig.md:355-358` sitúa `PlacementOrder` en
  `commands/mod.rs:314-328`. Hoy vive en `commands/orders.rs:29-70`: el fichero se partió
  después de escribirse aquel informe.
* El [#31](https://github.com/sgomez/rfirma/issues/31) llama a la decisión del sello
  «ADR-0012»; al publicarse acabó siendo el **ADR-0016** (el 0012 es hoy el de la rúbrica).
