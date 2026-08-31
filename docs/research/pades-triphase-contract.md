# Contrato real de la firma trifásica PAdES

Investigación contra el código fuente original de `clienteafirma` (ruta local `/home/sergio/Developer/SideProjects/clienteafirma`, módulos `afirma-server-triphase-signer-core`, `afirma-crypto-pdf`, `afirma-crypto-cades`, `afirma-crypto-pdf-common` y `afirma-core`). Responde a las seis preguntas del issue [#3](https://github.com/sgomez/rfirma/issues/3).

Todas las citas son `fichero:línea` relativas a la raíz de `clienteafirma`.

## 1. Firmas exactas de `preProcessPreSign` / `preProcessPostSign` para PAdES

Interfaz genérica (`afirma-server-triphase-signer-core/src/main/java/es/gob/afirma/triphase/signer/processors/TriPhasePreProcessor.java:36,51,66`):

```java
TriphaseData preProcessPreSign(byte[] data, String signAlgorithm, X509Certificate[] cert,
        Properties extraParams, boolean checkSignatures) throws IOException, AOException;

byte[] preProcessPostSign(byte[] data, String signAlgorithm, X509Certificate[] cert,
        Properties extraParams, byte[] session) throws NoSuchAlgorithmException, IOException, AOException;

byte[] preProcessPostSign(byte[] data, String signAlgorithm, X509Certificate[] cert,
        Properties extraParams, TriphaseData sessionData) throws NoSuchAlgorithmException, IOException, AOException;
```

Implementación concreta de PAdES: `PAdESTriPhasePreProcessor.java:64-134` (preSign) y `:143-213` (postSign, con dos sobrecargas: una recibe `byte[] session` en bruto y hace `TriphaseData.parser(session)` en la línea 151, la otra recibe ya un `TriphaseData` parseado).

Parámetros y significado:
- `data` / `docBytes`: bytes del PDF de entrada (el mismo documento en pre y post).
- `algorithm`/`signAlgorithm`: nombre del algoritmo de firma (p. ej. `SHA256withRSA`); debe coincidir entre fases.
- `cert`: cadena de certificados del firmante (`X509Certificate[]`), obligatoria en ambas fases.
- `extraParams`: `java.util.Properties`, ver pregunta 5.
- `checkSignatures` (solo preSign): si es `true` y el PDF ya contiene firmas, se valida su integridad antes de continuar (`PAdESTriPhasePreProcessor.java:76-90`); no es obligatorio, por defecto se puede pasar `false`.
- `session`/`sessionData` (solo postSign): el `TriphaseData` que devolvió la prefirma, ya rellenado por el cliente con el resultado PKCS#1. Es obligatorio: si `triphaseData.getSignsCount() < 1` se lanza `AOException` (`PAdESTriPhasePreProcessor.java:158-161`).

**El spec `rfirma_development_spec.md` está equivocado en la firma de `preProcessPostSign`**: no acepta `(data, algorithm, certChain, extraParams, false)` como afirma el issue; el quinto parámetro es siempre datos de sesión (`byte[]` o `TriphaseData`), nunca un booleano. El booleano `checkSignatures` solo existe en `preProcessPreSign`.

## 2. Qué es `TriphaseData` estructuralmente

Definición: `afirma-core/src/main/java/es/gob/afirma/core/signers/TriphaseData.java`.

- Es una lista de `TriSign` (`:35-251`) más un `format` opcional (formato de firma, p. ej. `"PAdES"`).
- Cada `TriSign` (clase interna, `:39-213`) contiene:
  - `id`: identificador de esa firma individual (aleatorio con `UUID.randomUUID()` si no se especifica, `:81`).
  - `signatureId`: identificador de firma global, para agrupar varias firmas individuales de una misma operación (cofirmas/contrafirmas).
  - `dict`: un `Map<String,String>` de propiedades libres, clave→valor, todo en texto (los binarios van en Base64). Para PAdES las claves son las constantes de `PAdESTriPhasePreProcessor.java:45-57`: `TIME`, `PID`, `PRE`, `PK1`, `NEED_PRE`.
- **Serialización: solo hay un formato, XML**, no hay una variante JSON en `afirma-core`. `TriphaseData.toString()` (`:419-461`) genera manualmente (con `StringBuilder`, no con un serializador DOM) un XML del tipo:

```xml
<xml>
 <firmas format="PAdES">
  <firma Id="001" signid="...">
   <param n="NEED_PRE">true</param>
   <param n="PRE">MYICXDAYBgkqhkiG9[...]w0BA=</param>
   <param n="TIME">1234567890</param>
   <param n="PID">Base64(fileID)</param>
   <param n="PK1">EMijB9pJ0lj27Xqov[...]RnCM=</param>
  </firma>
 </firmas>
</xml>
```

Ejemplo tomado del javadoc de `parser`, `TriphaseData.java:255-271`.

- El parseo inverso se hace con `TriphaseData.parser(byte[] xml)` (`:272-287`, usa `SecureXmlBuilder`/DOM) y `TriphaseData.parser(Element xml)` (`:308-...`). No hay `fromJson`/`toJson` en esta clase.
- El spec del proyecto asume que `triphaseData.toString()` "es como si fuera XML" — es correcto en este punto: efectivamente **es** XML literal, no una representación abstracta. Pero conviene que el bridge Rust decida explícitamente si mantiene XML o lo traduce a JSON en la frontera FFI, porque Java solo entiende XML de forma nativa aquí.

## 3. Qué se firma exactamente (pregunta más importante)

**No es un hash crudo, ni un `DigestInfo`, ni los datos en claro del PDF: es el conjunto de atributos firmados CAdES (`SignedAttributes`) codificados en ASN.1 DER.**

Cadena de llamadas, con cita literal:

1. `PAdESTriPhaseSigner.preSign` (`afirma-crypto-pdf/src/main/java/es/gob/afirma/signers/pades/PAdESTriPhaseSigner.java:165-219`):
   - Calcula el rango de bytes del PDF a firmar (`pdfRangeBytes`, línea 178, vía `ptps.getSAP().getRangeStream()`).
   - Calcula el hash de ese rango:
     ```java
     // línea 185-190
     md = MessageDigest.getInstance(parameters.getDigestAlgorithm()).digest(pdfRangeBytes);
     ...
     parameters.setDataDigest(md);
     ```
   - Ese hash **no se devuelve directamente**; se inyecta como `dataDigest` dentro de un `CAdESParameters` (`config`), es decir, se convertirá en el valor del atributo CAdES `message-digest`.
   - Invoca `CAdESTriPhaseSigner.preSign(signerCertificateChain, signTime.getTime(), parameters)` (línea 209-213).

2. `CAdESTriPhaseSigner.preSign` (`afirma-crypto-cades/src/main/java/es/gob/afirma/signers/cades/CAdESTriPhaseSigner.java:152-183`):
   ```java
   // líneas 160-182
   public static byte[] preSign(
           final Certificate[] signerCertificateChain,
           final Date signDate,
           final CAdESParameters config) throws AOException {
       ...
       final ASN1EncodableVector signedAttributesVector =
               CAdESUtils.generateSignedAttributes(
                       signerCertificateChain[0],
                       config,
                       false  // No es contrafirma
                       );
       signedAttributes = SigUtils.getAttributeSet(
               new AttributeTable(signedAttributesVector)
            );
       ...
       return signedAttributes.getEncoded(ASN1Encoding.DER);
   }
   ```
   El valor de retorno (el mismo Javadoc de la línea 152 lo llama "Atributos firmados CAdES (prefirma)") es el `SET OF Attribute` de PKCS#7/CAdES **ya codificado en DER**, que internamente incluye (entre otros) el atributo `message-digest` con el hash del rango del PDF calculado en el paso 1, y los atributos `content-type`, `signing-certificate`/`signing-certificate-v2`, `signing-time` (aquí puesto a `null` explícitamente, ver más abajo), etc., según `CAdESUtils.generateSignedAttributes`.

3. `PAdESTriPhaseSigner.preSign` guarda ese resultado (`cadesPresign`) como `preSignature.getSign()` (línea 217) y `PAdESTriPhasePreProcessor.preProcessPreSign` lo mete en `TriphaseData` bajo la clave `PRE`:
   ```java
   // PAdESTriPhasePreProcessor.java:126
   signConfig.put(PROPERTY_NAME_PRESIGN, Base64.encode(preSignature.getSign()));
   ```

Por tanto: **`TriphaseData["PRE"]` = Base64(DER(SignedAttributes CAdES))**, no el hash puro del PDF ni un `DigestInfo` PKCS#1 clásico. El paso que el módulo Rust debe reproducir en frontera de firma es: tomar esos bytes DER, calcular su hash con el algoritmo de resumen implícito en `signAlgorithm` (p. ej. SHA-256 si el algoritmo es `SHA256withRSA`) y aplicar el padding PKCS#1 v1.5 (o la firma nativa PKCS#11/CNG/Keychain que internamente hace ese mismo cálculo) sobre ese resumen — **igual que una firma RSA "PKCS1" estándar sobre un buffer arbitrario de bytes**, no sobre el hash del PDF directamente. Detalle relevante: en `PAdESTriPhaseSigner.preSign` (líneas 194-203) se fuerzan explícitamente varias particularidades de CAdES-en-PAdES antes de generar los atributos:
```java
// líneas 194-203
parameters.setMetadata(null);        // la localización va en el PDF, no en el CAdES interno
parameters.setContentTypeOid(null);  // el tipo de datos firmados nunca se declara
parameters.setContentDescription(null);
parameters.setSigningTime(null);     // la marca de tiempo NO se incluye en la firma
```
Esto es importante para Rust/reimplementación: la firma trifásica de PAdES **no debe incluir el atributo `signing-time` CAdES** (la hora va aparte, en el campo `TIME` del `TriphaseData`, para reconstruirla en la postfirma con `Calendar`).

No determinado con el código leído: la ruta cliente (`afirma-crypto-padestri-client`) que efectivamente invoca `Signature.getInstance(...)`/PKCS#11 sobre estos bytes no se ha inspeccionado línea a línea en esta investigación (se limitó por presupuesto de tokens); se infiere el comportamiento estándar (RSA-PKCS1 sobre el hash de estos bytes DER) por el nombre `PK1` y el uso de `signatureAlgorithm` en `preProcessPostSign`, pero no hay cita de código que lo confirme literalmente.

## 4. Dónde se deposita el resultado de la firma para la postfirma

El cliente debe devolver el `TriphaseData` recibido en la prefirma, añadiendo el resultado de la firma PKCS#1 bajo la clave `PK1` en el mismo `TriSign` (misma `Id`). Esto se deduce de:

- La constante `PROPERTY_NAME_PKCS1_SIGN = "PK1"` (`PAdESTriPhasePreProcessor.java:54`).
- `checkSession` (`PAdESTriPhasePreProcessor.java:216-231`) exige que el `TriSign` recibido en la postfirma contenga los cuatro parámetros: `PRE`, `PK1`, `PID` (el `PROPERTY_NAME_PDF_UNIQUE_ID`), y `TIME`. Si falta alguno lanza `AOException` con `TriphaseErrorCode.Request.MALFORMED_PRESIGN`.
- En `preProcessPostSign` (líneas 191-198) se lee `signConfig.getProperty(PROPERTY_NAME_PKCS1_SIGN)` (Base64 decodificado) y se pasa como `pkcs1Signature` a `PAdESTriPhaseSigner.postSign`.

Es decir: el nombre exacto que debe usar el bridge Rust para depositar la firma PKCS#1 calculada nativamente es el parámetro `"PK1"` dentro del mismo `TriSign` (mismo `Id`) devuelto por la prefirma, junto con los otros tres campos (`PRE`, `PID`, `TIME`) sin modificar.

## 5. Qué `extraParams` consume PAdES y en qué fase

Nombres completos definidos en `afirma-crypto-pdf-common/src/main/java/es/gob/afirma/signers/pades/common/PdfExtraParams.java` (solo se ha enumerado, no leído el cuerpo entero). Ejemplos relevantes: `SIGNATURE_ROTATION`, `SIGN_TIME`, `PROFILE`, `SIGNATURE_SUBFILTER`, `COMPRESS_PDF`, `ALWAYS_CREATE_REVISION`, `IMAGE*` / `SIGNATURE_POSITION_ON_PAGE_*` (firma visible), `CERTIFICATION_LEVEL`, `PDF_VERSION`, `SIGNATURE_FIELD`, `VISIBLE_SIGNATURE*`, `SIGN_REASON`, `SIGNATURE_PRODUCTION_CITY`, `SIGNER_CONTACT`, `POLICY_IDENTIFIER*`, `SIGNER_CLAIMED_ROLES`, `OWNER_PASSWORD_STRING`, `HEADLESS`, `ALLOW_SIGNING_CERTIFIED_PDFS`, `TS_TYPE`, `TSA_URL`, `COMMITMENT_TYPE_INDICATION*`.

Por fase, según el flujo de código verificado:

- **Fase de prefirma (`preProcessPreSign` → `PAdESTriPhaseSigner.preSign`)**: `extraParams` se pasa completo a `PdfSessionManager.getSessionData(inPDF, cert, signTime, extraParams, secureMode)` (`PAdESTriPhaseSigner.java:176`), que es donde se construye el diccionario de firma del PDF, se reserva el `ByteRange` y se aplican las opciones de apariencia visible/posición/certificación (`CERTIFICATION_LEVEL`, `SIGNATURE_FIELD`, `VISIBLE_SIGNATURE*`, `IMAGE*`, `SIGNATURE_POSITION_ON_PAGE_*`, `SIGNATURE_SUBFILTER`, `PDF_VERSION`, `ALWAYS_CREATE_REVISION`, etc.), **porque estas opciones alteran los bytes del PDF antes de calcular el hash del rango**, y por tanto deben fijarse antes de generar `PRE`. No se ha leído el cuerpo de `PdfSessionManager` (fuera del presupuesto de esta investigación), así que la lista exacta de qué subconjunto de `PdfExtraParams` consume ese método puntualmente es "no determinado" a nivel de línea — solo se ha verificado que recibe el objeto completo.

- **Fase de postfirma (`preProcessPostSign` → `PAdESTriPhaseSigner.postSign` → `generatePdfSignature`)**: `extraParams` (guardado como `preSign.getExtraParams()`, ver `PdfSignResult` reconstruido en `PAdESTriPhasePreProcessor.java:196-202`) se reutiliza para decidir el sellado de tiempo: `TS_TYPE` y `TSA_URL` se comprueban explícitamente en `PAdESTriPhaseSigner.java:304-306` (`generatePdfSignature`) para activar/desactivar el sello de tiempo a nivel de firma. También se pasa a `AOPDFSigner.getSignEnhancerConfig()`/`insertSignatureOnPdf` para el ensamblado final.

No determinado con certeza: qué parámetros exactos son *exclusivos* de una fase u otra a nivel de cada constante individual — el objeto `extraParams` se pasa completo a ambas fases (`preProcessPreSign` y `preProcessPostSign` reciben el mismo `Properties` desde el llamador), así que en la práctica el propio código de sesión/render decide qué mira y cuándo; solo `TS_TYPE`/`TSA_URL` están confirmados como leídos específicamente en la fase de postfirma (línea citada arriba), y la construcción de apariencia/posición como leída en la fase de prefirma (por necesidad de fijar el `ByteRange` antes del hash).

## 6. ¿PAdES trifásico arrastra CAdES internamente?

**Sí, de forma directa y obligatoria.** `PAdESTriPhasePreProcessor` importa y llama a `PAdESTriPhaseSigner` (`afirma-crypto-pdf`), y este a su vez importa y llama literalmente a `CAdESTriPhaseSigner` y `CAdESParameters` (`afirma-crypto-cades`):

- Import: `PAdESTriPhaseSigner.java` (paquete `es.gob.afirma.signers.pades`) usa `CAdESTriPhaseSigner.preSign(...)` en la línea 209 y `CAdESTriPhaseSigner.postSign(...)` en la línea 283.
- Tanto la prefirma como la postfirma de PAdES son, en el fondo, una prefirma/postfirma CAdES sobre el hash del rango de bytes del PDF (ver pregunta 3).

**Conclusión para el `pom.xml`**: el módulo `rfirma-native-bridge` (o el submódulo Java equivalente) necesita la dependencia de `afirma-crypto-cades` (y transitivamente su codificación ASN.1, p. ej. BouncyCastle) además de `afirma-crypto-pdf`, no puede prescindir de ella aunque el objetivo final sea solo firmar PDFs con PAdES.

## Resumen de discrepancias encontradas con `rfirma_development_spec.md`

- La llamada a `PreProcessorFactory.getPreProcessor(format)` existe (`PreProcessorFactory.java:15`), pero también hay una sobrecarga que detecta el formato a partir de los propios bytes del documento (`getPreProcessor(byte[] data)`, línea 10, que detecta PDF y fuerza PAdES, líneas 55-56).
- `preProcessPreSign(data, algorithm, certChain, extraParams, false)` es correcto en firma (el quinto argumento es el booleano `checkSignatures`).
- `preProcessPostSign` con un booleano como quinto argumento **es incorrecto**: el quinto argumento siempre es la sesión (`byte[]` o `TriphaseData`), nunca un booleano.
- `triphaseData.toString()` como "si fuera XML" es correcto: es literalmente XML generado a mano, no hay variante JSON en `afirma-core`.
