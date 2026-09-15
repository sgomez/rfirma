# 12. Parámetros extra por formato de firma

Este capítulo documenta de forma exhaustiva el catálogo de parámetros adicionales
(`properties` / `extraParams`) específicos para cada formato criptográfico de firma
soportado activamente por AutoFirma 1.9.2: **PAdES**, **CAdES**, **XAdES**, **FacturaE**
y **ASiC** (ASiC-S CAdES y ASiC-S XAdES). Conforme a las directrices de este manual,
los formatos considerados obsoletos o deprecados (OOXML, ODF, CMS y XMLDSig) han
sido omitidos.

Los mecanismos generales de codificación en Base64 URL-safe, deserialización en
`java.util.Properties`, macro-expansión de políticas de firma mediante
`ExtraParamsProcessor` y filtrado de certificados se detallaron en el
[capítulo 11](11-extraparams-y-filtros.md).

Todas las citas corresponden al código fuente de
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma) en el tag
`v1.9.2` (commit `b4fe147c3`), con rutas relativas a la raíz de dicho repositorio.

---

## 1. Dónde viven los parámetros de formato en el código original

| Formato | Clases principales de constantes y signers | Módulo |
|---|---|---|
| **PAdES** | `PdfExtraParams.java`, `AOPDFSigner.java`, `PdfSessionManager.java`, `PdfPreProcessor.java`, `PdfUtil.java`, `PdfVisibleAreasUtils.java`, `PdfTimestamper.java` | `afirma-crypto-pdf-common`, `afirma-crypto-pdf` |
| **CAdES** | `CAdESExtraParams.java`, `AOCAdESSigner.java`, `CAdESParameters.java`, `CAdESSignerMetadataHelper.java`, `CommitmentTypeIndicationsHelper.java` | `afirma-crypto-cades` |
| **XAdES** | `XAdESExtraParams.java`, `AOXAdESSigner.java`, `XAdESSigner.java`, `XAdESCommonMetadataUtil.java`, `XAdESUtil.java`, `XAdESTspUtil.java`, `Utils.java` | `afirma-crypto-xades`, `afirma-crypto-core-xml` |
| **FacturaE** | `AOFacturaESigner.java` | `afirma-crypto-xades` |
| **ASiC (CAdES / XAdES)** | `CAdESASiCExtraParams.java`, `AOCAdESASiCSSigner.java`, `XAdESASiCExtraParams.java`, `AOXAdESASiCSSigner.java` | `afirma-crypto-cades`, `afirma-crypto-xades` |
| **Sellado de Tiempo (TSA)** | `TsaParams.java`, `CMSTimestamper.java` | `afirma-crypto-core-pkcs7-tsp` |

Adicionalmente, los desarrolladores de AutoFirma incorporaron ficheros HTML de documentación
interna en los subdirectorios `doc-files/extraparams.html` de cada módulo criptográfico
(`afirma-crypto-pdf/.../doc-files/extraparams.html`, `afirma-crypto-cades/.../doc-files/extraparams.html`,
`afirma-crypto-xades/.../doc-files/extraparams.html` y `afirma-crypto-cades/asic/.../doc-files/extraparams-asic-s.html`).

---

## 2. Parámetros de firmas PAdES (PDF)

El firmador de PDF se implementa en `AOPDFSigner` (`afirma-crypto-pdf/src/main/java/es/gob/afirma/signers/pades/AOPDFSigner.java`)
y sus constantes se definen en `PdfExtraParams` (`afirma-crypto-pdf-common/src/main/java/es/gob/afirma/signers/pades/common/PdfExtraParams.java`).

### 2.1 Catálogo general de claves PAdES

| Clave en `properties` | Constante en `PdfExtraParams` | Tipo | Valor por defecto | Descripción |
|---|---|---|---|---|
| `signatureSubFilter` | `SIGNATURE_SUBFILTER` | Cadena | `"adbe.pkcs7.detached"` | Subfiltro en el diccionario PDF (`"adbe.pkcs7.detached"` para PAdES básico o `"ETSI.CAdES.detached"` para PAdES-BES). Si se declara política de firma o perfil baseline, se fuerza a `ETSI.CAdES.detached`. |
| `pdfVersion` | `PDF_VERSION` | Entero / Cadena | Según entrada o `7` | Versión del PDF de salida (`2` = 1.2, `3` = 1.3, `4` = 1.4, `5` = 1.5, `6` = 1.6, `7` = 1.7, `-1` = respetar versión de entrada). |
| `compressPdf` | `COMPRESS_PDF` | Booleano | `true` | Comprime el PDF de salida si la versión es >= 4 (PDF 1.4+). Si es `false`, omite la compresión (`setFullCompression`). Se ignora en PDF/A-1. |
| `alwaysCreateRevision` | `ALWAYS_CREATE_REVISION` | Booleano | `false` | Fuerza la creación de una nueva revisión incremental incluso si el documento no tenía firmas previas. En documentos cifrados siempre se crea revisión. |
| `signReservedSize` | `SIGN_RESERVED_SIZE` | Entero | `27000` | Tamaño en bytes reservado en el diccionario PDF para almacenar la firma binaria (`CSIZE = 27000`). |
| `signTime` | `SIGN_TIME` | Cadena | Fecha/hora actual | Fecha y hora de firma en formato `yyyy:MM:dd:HH:mm:ss`. Solo aplica en firmas monofásicas locales. |
| `includeOnlySignningCertificate` | `INCLUDE_ONLY_SIGNNING_CERTIFICATE` | Booleano | `false` | Si es `true`, incluye únicamente el certificado del firmante; si es `false`, incluye la cadena completa de certificación. |
| `signingCertificateV2` | `SIGNING_CERTIFICATE_V2` | Booleano | Según algoritmo | `true` para usar `SigningCertificateV2`, `false` para V1. Si no se indica, usa V1 con firmas SHA-1 y V2 para cualquier otro algoritmo. |
| `doNotIncludePolicyOnSigningCertificate` | `DO_NOT_INCLUDE_POLICY_ON_SIGNING_CERTIFICATE` | Booleano | `false` | Si es `true`, omite la inclusión de la política de certificación en el atributo `SigningCertificate`. |
| `doNotUseCertChainOnPostSign` | `DO_NOT_USE_CERTCHAIN_ON_POSTSIGN` | Booleano | `false` | Si es `true`, evita usar la cadena de certificados en el diccionario PDF y en `PdfSignatureAppearance` (uso en modo trifásico). |
| `ownerPassword` | `OWNER_PASSWORD_STRING` | Cadena | `null` | Contraseña del propietario / apertura del PDF cifrado. |
| `userPassword` | `USER_PASSWORD_STRING` | Cadena | `null` | Contraseña de usuario del PDF cuando está protegido contra apertura o modificaciones. |
| `certificationLevel` | `CERTIFICATION_LEVEL` | Entero | `0` | Nivel de certificación: `0` (firma ordinaria sin certificar), `1` (autor, sin cambios posteriores), `2` (autor, solo formularios), `3` (común, formularios y firmas adicionales). |
| `allowSigningCertifiedPdfs` | `ALLOW_SIGNING_CERTIFIED_PDFS` | Booleano | `false` / modal | Si es `true`, permite firmar PDFs certificados. Si no se indica, muestra diálogo de advertencia al usuario. Si es `headless=true` y no está a `true`, lanza excepción inmediata. |
| `allowCosigningUnregisteredSignatures` | `ALLOW_COSIGNING_UNREGISTERED_SIGNATURES` | Booleano | `false` | Si es `true`, permite firmar un PDF con firmas previas no registradas en AcroFields. Si es `false`, lanza `PdfHasUnregisteredSignaturesException`. |
| `allowShadowAttack` | `ALLOW_SHADOW_ATTACK` | Booleano | `false` | Si es `true`, desactiva la comprobación de ataques PDF Shadow Attack en las firmas previas. |
| `pagesToCheckShadowAttack` | `PAGES_TO_CHECK_PSA` | Entero / Cadena | `"10"` | Número de páginas a comprobar contra Shadow Attacks (por defecto 10, o `"all"` para todas). |
| `allowModifiedForm` | `ALLOW_SIGN_MODIFIED_FORM` | Booleano | `false` | Si es `true`, desactiva la comprobación de cambios en campos de formulario tras la firma anterior. |
| `checkCertificates` | `CHECK_CERTIFICATES` | Booleano | `false` | Indica si se debe comprobar la validez de los certificados durante la verificación de firmas preexistentes. |
| `attach` | `ATTACH` | Cadena Base64 | Ninguno | Contenido binario en Base64 a adjuntar al PDF. Requiere `attachFileName`. Si `secureMode=false` y longitud < 500, admite ruta local o URI. |
| `attachFileName` | `ATTACH_FILENAME` | Cadena | Ninguno | Nombre del fichero adjunto incrustado en el PDF. |
| `attachDescription` | `ATTACH_DESCRIPTION` | Cadena | `null` | Descripción textual del fichero adjunto. |
| `image` | `IMAGE` | Cadena Base64 | Ninguno | Imagen JPEG en Base64 para estampar en el documento antes de firmarlo. |
| `imagePage` | `IMAGE_PAGE` | Entero | Ninguno | Página para estampar `image`: `1..N`, `-1` (última página), `0` (todas las páginas). |
| `imagePositionOnPageLowerLeftX` | `IMAGE_POSITION_ON_PAGE_LOWER_LEFTX` | Entero | Ninguno | Coordenada X inferior izquierda de la imagen previa. |
| `imagePositionOnPageLowerLeftY` | `IMAGE_POSITION_ON_PAGE_LOWER_LEFTY` | Entero | Ninguno | Coordenada Y inferior izquierda de la imagen previa. |
| `imagePositionOnPageUpperRightX` | `IMAGE_POSITION_ON_PAGE_UPPER_RIGHTX` | Entero | Ninguno | Coordenada X superior derecha de la imagen previa. |
| `imagePositionOnPageUpperRightY` | `IMAGE_POSITION_ON_PAGE_UPPER_RIGHTY` | Entero | Ninguno | Coordenada Y superior derecha de la imagen previa. |
| `signReason` | `SIGN_REASON` | Cadena | `null` | Motivo de la firma (se añade al diccionario PDF `/Reason`). Si se especifica `policyIdentifier`, se elimina automáticamente en `checkParams`. |
| `signatureProductionCity` | `SIGNATURE_PRODUCTION_CITY` | Cadena | `null` | Ciudad de creación de la firma (se añade al diccionario PDF `/Location`). |
| `signerContact` | `SIGNER_CONTACT` | Cadena | `null` | Información de contacto del firmante (se añade al diccionario PDF `/ContactInfo`). |
| `signerClaimedRoles` | `SIGNER_CLAIMED_ROLES` | Cadena | `null` | Cargos o roles declarados separados por el carácter `'\|'`. |
| `policyIdentifier` | `POLICY_IDENTIFIER` | Cadena | `null` | OID o URN OID de la política de firma. |
| `policyIdentifierHash` | `POLICY_IDENTIFIER_HASH` | Cadena Base64 | `null` | Huella digital de la política de firma en Base64. |
| `policyIdentifierHashAlgorithm` | `POLICY_IDENTIFIER_HASH_ALGORITHM` | Cadena | `null` | Algoritmo de hash de la política (`SHA-1`, `SHA-256`, `SHA-384`, `SHA-512`). |
| `policyQualifier` | `POLICY_QUALIFIER` | Cadena URL | `null` | URL con la descripción humana de la política (típicamente un documento PDF). |
| `commitmentTypeIndications` | `COMMITMENT_TYPE_INDICATIONS` | Entero | `null` | Número de declaraciones de compromiso a incluir. |
| `commitmentTypeIndication`*n*`Identifier` | `COMMITMENT_TYPE_INDICATION_IDENTIFIER` | Cadena | Ninguno | Identificador de tipo (`1` = Origen, `2` = Recepción, `3` = Entrega, `4` = Envío, `5` = Aprobación, `6` = Creación). |
| `commitmentTypeIndication`*n*`CommitmentTypeQualifiers` | `COMMITMENT_TYPE_INDICATION_QUALIFIERS` | Cadena | `null` | Lista de OIDs calificadores adicionales separados por `'\|'`. |

---

### 2.2 Firma visible en PDF

La activación de la firma visible en PDF puede ser solicitada explícitamente mediante
el protocolo o configurada a través de coordenadas y campos.

#### A. Activación interactiva vía protocolo
En `ProtocolInvocationLauncherSign.java:923-994` y `ProtocolInvocationLauncherSignAndSave.java:948-1025`:
* `visibleSignature`:
  * `"want"`: fuerza obligatoriamente la selección de un área visible por parte del usuario.
    Si el usuario cancela la selección gráfica, se aborta la firma con error `SAF_43`
    (`VisibleSignatureMandatoryException` -> `ProtocolInvocationLauncherErrorManager.ERROR_VISIBLE_SIGNATURE`).
  * `"optional"`: muestra el diálogo modal preguntando al usuario si desea incluir firma
    visible. Si el usuario acepta, procede a marcar el recuadro; si declina, continúa con firma invisible.
* `visibleAppearance`:
  * `"custom"`: abre el diálogo modal de configuración de apariencia gráfica (`SignAppearanceDialog`)
    para personalizar texto, tipografías, logotipo y rúbrica manuscrita.

#### B. Posicionamiento directo en página
En `PdfVisibleAreasUtils.java:616-651` y `PdfUtil.java:699-730`:
* `signatureField`: nombre de un campo de firma Acrobat preexistente (*AcroField*). Si se
  especifica, se estampa la firma en dicho campo y **se ignoran por completo** los parámetros
  de página y coordenadas (`PdfUtil.java:708`).
* `signaturePage` / `signaturePages`: determina las páginas donde se imprime la firma visible.
  Admite sintaxis rica:
  * Número individual: `"1"` (primera página), `"-1"` (última página), `"-2"` (penúltima página).
  * Lista de páginas: `"1,2,5"`.
  * Rangos: `"2-5"`.
  * Combinaciones complejas: `"1-3,-3--1"`.
  * Palabra clave `"all"`: estampa la firma en todas las páginas del documento.
  * Palabra clave `"append"`: inserta una **página nueva en blanco al final** del documento
    (con las mismas dimensiones que la página 1) y estampa la firma en ella (`PdfSessionManager.java:395-400`).
* Coordenadas del recuadro visible:
  * `signaturePositionOnPageLowerLeftX`
  * `signaturePositionOnPageLowerLeftY`
  * `signaturePositionOnPageUpperRightX`
  * `signaturePositionOnPageUpperRightY`

#### C. Rúbrica manuscrita gráfica
* `signatureRubricImage`: imagen JPEG en Base64 con la firma manuscrita.
  * Se inserta en la capa 2 de la apariencia (`PdfSessionManager.java:107`).
  * Si está presente y no se especifica `layer2Text`, **suprime cualquier texto en capa 2** (`PdfSessionManager.java:231`).
  * Si `secureMode=false` y la cadena mide menos de 500 caracteres, puede referenciar una ruta de fichero local o URI.

#### D. Composición de texto en Capa 2 (`layer2Text`)
Si no se proporciona rúbrica gráfica y no se especifica `layer2Text`, AutoFirma inyecta un
texto por defecto (`PdfSessionManager.java:609-621`):
```text
Firmado por $$SUBJECTCN$$
Fecha: $$SIGNDATE=dd/MM/yyyy HH:mm:ss z$$
Motivo: $$REASON$$          (solo si se especificó signReason)
Lugar de firma: $$LOCATION$$ (solo si se especificó signatureProductionCity)
```

En `PdfVisibleAreasUtils.java:244-355`, el texto de `layer2Text` admite las siguientes
etiquetas dinámicas (macros):

| Macro | Origen del dato | Cita en código |
|---|---|---|
| `$$SUBJECTCN$$` | CN (*Common Name*) del titular del certificado. Pasa por el filtro de ofuscación si está activo. | `PdfVisibleAreasUtils.java:263-268` |
| `$$ISSUERCN$$` | CN del emisor (CA) del certificado. | `PdfVisibleAreasUtils.java:269` |
| `$$CERTSERIAL$$` | Número de serie decimal del certificado. | `PdfVisibleAreasUtils.java:270` |
| `$$PSEUDONYM$$` | RDN de seudónimo (OID `2.5.4.65`) si existe en el Subject. | `PdfVisibleAreasUtils.java:275-278` |
| `$$OU$$` | Primera unidad organizativa (primer RDN `ou`) del Subject. | `PdfVisibleAreasUtils.java:280-282` |
| `$$OUS$$` | Todas las unidades organizativas unidas por coma (`OU1, OU2`). | `PdfVisibleAreasUtils.java:283-288` |
| `$$TITLE$$` | Cargo o título profesional (RDN `t`) del Subject. | `PdfVisibleAreasUtils.java:294-295` |
| `$$SUBJECTDN$$` | Distinguished Name completo del titular. Pasa por ofuscación si está activa. | `PdfVisibleAreasUtils.java:273, 301` |
| `$$GIVENNAME$$` | Nombre de pila (RDN `GIVENNAME`) del Subject. | `PdfVisibleAreasUtils.java:304-305` |
| `$$SURNAME$$` | Apellidos (RDN `SURNAME`) del Subject. | `PdfVisibleAreasUtils.java:308-309` |
| `$$ORGANIZATION$$` | Organización (RDN `o`) del Subject. | `PdfVisibleAreasUtils.java:312-313` |
| `$$REASON$$` | Valor del parámetro `signReason`. | `PdfVisibleAreasUtils.java:317` |
| `$$LOCATION$$` | Valor del parámetro `signatureProductionCity`. | `PdfVisibleAreasUtils.java:318` |
| `$$CONTACT$$` | Valor del parámetro `signerContact`. | `PdfVisibleAreasUtils.java:319` |
| `$$SIGNDATE=PATRON$$` | Fecha y hora formateada según el patrón de `java.text.SimpleDateFormat` indicado. | `PdfVisibleAreasUtils.java:322-345` |
| `$$SIGNDATE$$` | Fecha y hora con formato por defecto de `SimpleDateFormat()`. | `PdfVisibleAreasUtils.java:346-350` |

#### E. Tipografía, color y estilo
Controlados en `PdfSessionManager.java:248-292` y `PdfVisibleAreasUtils.java:88-145`:
* `layer2FontFamily`: `0` = Courier (defecto), `1` = Helvetica, `2` = Times Roman, `3` = Symbol, `4` = ZapfDingBats.
* `layer2FontSize`: tamaño numérico en puntos (por defecto `12`).
* `layer2FontStyle`: máscara de bits entera: `0` = Normal, `1` = Negrita, `2` = Cursiva, `3` = Negrita/Cursiva, `4` = Subrayado, `8` = Tachado (admite combinaciones OR).
* `layer2FontColor`: nombre textual de color (insensible a mayúsculas): `"black"` (defecto), `"white"`, `"gray"`, `"lightGray"`, `"darkGray"`, `"red"`, `"pink"`.
* `signatureRotation`: rotación en grados en sentido horario del texto de firma: `0` (defecto), `90`, `180`, `270`.
* `includeQuestionMark`: booleano (`false` por defecto). Si es `true`, activa el render de interrogación/aspa de validez de Acrobat (`sap.setRender(true)`).
* `layer4Text`: texto de la capa 4 de apariencia (rara vez utilizado, solo si no hay rúbrica).

#### F. Máscara de ofuscación de datos sensibles
Implementada en `PdfTextMask.java:6-187` y `PdfVisibleAreasUtils.java:653-725`:
* `obfuscateCertText`: con `true`, oculta cifras numéricas de documentos de identidad en el CN y DN del titular.
* `obfuscationMask`: patrón con formato `caracterSustitutivo;longitudDigitos;posiciones;desplazamiento`.
  * Por defecto: carácter `'*'`, mínimo 3 dígitos consecutivos, posiciones `{false, false, false, true, true, true, true}`, desplazamiento `true` (oculta los 3 primeros dígitos y muestra los 4 siguientes).

---

### 2.3 Reglas de prevalencia e incompatibilidades en `AOPDFSigner.checkParams`

En `AOPDFSigner.java:705-754`, AutoFirma aplica una serie de reglas de saneamiento antes de firmar:

1. **Rechazo de algoritmos obsoletos**:
   Si el algoritmo de firma comienza por `"MD"` (por ejemplo `MD5withRSA` o `MD2withRSA`), lanza `IllegalArgumentException("PAdES no permite huellas digitales MD2 o MD5 (Decision 130/2011 CE)")` (`líneas 707-709`).
2. **Firmas Baseline**:
   Si `profile=baseline`, emite advertencia si el algoritmo es SHA-1 (`líneas 715-717`) y **elimina silenciosamente** `signatureSubFilter`, fijándolo obligatoriamente a `ETSI.CAdES.detached` (`líneas 719-725`).
3. **Commitment Type Indications en PAdES**:
   Las firmas PAdES-BES no admiten *Commitment Type Indications*. Si se especifica `commitmentTypeIndications` sin indicar una política de firma (`policyIdentifier`) y no es perfil baseline, **se elimina silenciosamente** (`líneas 730-735`).
4. **Conflicto entre `signReason` y `policyIdentifier`**:
   Si se especifican simultáneamente `signReason` y `policyIdentifier`, **se elimina silenciosamente `signReason`** en favor de la política de firma (`líneas 739-743`).
5. **Conflicto entre `signReason` y `commitmentTypeIndications`**:
   Si se especifican simultáneamente `signReason` y `commitmentTypeIndications`, **se elimina `commitmentTypeIndications`** en favor de la razón de firma (`líneas 749-753`).

---

## 3. Parámetros de firmas CAdES

El firmador de CAdES se implementa en `AOCAdESSigner` (`afirma-crypto-cades/src/main/java/es/gob/afirma/signers/cades/AOCAdESSigner.java`)
y sus constantes se definen en `CAdESExtraParams` (`afirma-crypto-cades/src/main/java/es/gob/afirma/signers/cades/CAdESExtraParams.java`).
La carga y resolución de parámetros se realiza en `CAdESParameters.load` (`CAdESParameters.java:100-335`).

### 3.1 Catálogo general de claves CAdES

| Clave en `properties` | Constante en `CAdESExtraParams` | Tipo | Valor por defecto | Descripción |
|---|---|---|---|---|
| `mode` | `MODE` | Cadena | `"implicit"` | Modo de firma: `"implicit"` (datos embebidos en el PKCS#7/CMS) o `"explicit"` (firma separada / detached). |
| `precalculatedHashAlgorithm` | `PRECALCULATED_HASH_ALGORITHM` | Cadena | `null` | Algoritmo de hash cuando los datos pasados en `data` son una huella digital precalculada (modo explícito sin acceso al contenido original). Si se indica, **invalida y elimina la clave `mode`** (`AOCAdESSigner.java:514-520`). |
| `signingCertificateV2` | `SIGNING_CERTIFICATE_V2` | Booleano | Según algoritmo | `true` para incluir el atributo `SigningCertificateV2`, `false` para V1. Si no se indica, usa V2 para SHA-2 y algoritmos distintos de SHA-1. Si `profile=baseline`, este parámetro se ignora y se elimina (`AOCAdESSigner.java:522-527`). |
| `includeOnlySignningCertificate` | `INCLUDE_ONLY_SIGNNING_CERTIFICATE` | Booleano | `false` | Si es `true`, incrusta únicamente el certificado del firmante; si es `false`, incluye la cadena completa de certificación en el mensaje CMS. |
| `doNotIncludePolicyOnSigningCertificate` | `DO_NOT_INCLUDE_POLICY_ON_SIGNING_CERTIFICATE` | Booleano | `false` | Si es `true`, omite la inclusión de las políticas del certificado en el atributo `SigningCertificate`. |
| `includeSigningTimeAttribute` | `INCLUDE_SIGNING_TIME_ATTRIBUTE` | Booleano | `true` | Si es `true`, incluye el atributo PKCS#9 `SigningTime` (OID `1.2.840.113549.1.9.5`). Diseñado para desactivarse (`false`) cuando la firma CAdES se va a encapsular dentro de un PDF (PAdES). |
| `includeContentHintAttribute` | `INCLUDE_CONTENT_HINT_ATTRIBUTE` | Booleano | `true` | Si es `true`, incluye el atributo PKCS#9 `content-hint` (OID `1.2.840.113549.1.9.16.2.4`). Se desactiva (`false`) cuando la firma forma parte de un PAdES. |
| `includeMimeTypeAttribute` | `INCLUDE_MIMETYPE_ATTRIBUTE` | Booleano | `false` | Si es `true`, incluye el atributo de tipo MIME (OID `0.4.0.1733.2.1`). No aplica a contrafirmas. |
| `mimeType` | `CONTENT_MIME_TYPE` | Cadena | Auto-detectado | Tipo MIME de los datos firmados (usado cuando `includeMimeTypeAttribute=true`). |
| `contentTypeOid` | `CONTENT_TYPE_OID` | Cadena OID | Auto-detectado | OID que identifica el tipo de datos firmados (usado para `content-hint` o deducir el `mimeType`). |
| `contentDescription` | `CONTENT_DESCRIPTION` | Cadena | `null` | Descripción textual del contenido firmado. Requiere que se defina `contentTypeOid`. |
| `signerClaimedRoles` | `SIGNER_CLAIMED_ROLES` | Cadena | `null` | Lista de cargos atribuidos al firmante separados por `'\|'`. |
| `signatureProductionCity` | `SIGNATURE_PRODUCTION_CITY` | Cadena | `null` | Ciudad en la que se realiza la firma. |
| `signatureProductionPostalCode` | `SIGNATURE_PRODUCTION_POSTAL_CODE` | Cadena | `null` | Código postal en el que se realiza la firma. |
| `signatureProductionCountry` | `SIGNATURE_PRODUCTION_COUNTRY` | Cadena | `null` | País en el que se realiza la firma. |
| `policyIdentifier` | `POLICY_IDENTIFIER` | Cadena | `null` | Identificador de la política de firma (OID o URN de tipo OID). |
| `policyIdentifierHash` | `POLICY_IDENTIFIER_HASH` | Cadena Base64 | `null` | Huella digital de la política en Base64. |
| `policyIdentifierHashAlgorithm` | `POLICY_IDENTIFIER_HASH_ALGORITHM` *(errata)* | Cadena | `null` | Algoritmo de hash de la política. *Atención: en `CAdESExtraParams.java:121` la constante tiene el valor literal `"poliyIdentifierHashAlgorithm"` (sin 'c')*. |
| `policyQualifier` | `POLICY_QUALIFIER` | Cadena URL | `null` | URL hacia el documento descriptivo de la política. |
| `commitmentTypeIndications` | `COMMITMENT_TYPE_INDICATIONS` | Entero | `null` | Número de declaraciones de compromiso a añadir. |
| `commitmentTypeIndication`*n*`Identifier` | `COMMITMENT_TYPE_INDICATION_IDENTIFIER` | Cadena | Ninguno | Identificador de compromiso (`1`..`6`). |
| `commitmentTypeIndication`*n*`CommitmentTypeQualifiers` | `COMMITMENT_TYPE_INDICATION_QUALIFIERS` | Cadena | `null` | Lista de OIDs calificadores adicionales separados por `'\|'`. |
| `allowSignLTSignature` | `ALLOW_SIGN_LTS_SIGNATURES` | Booleano | `false` | Permite la cofirma o contrafirma sobre firmas de archivo (LTS) preexistentes a pesar de que sus sellos de tiempo queden invalidados (`SigningLTSException`). |

---

### 3.2 Compromisos en CAdES (`CommitmentTypeIndicationsHelper`)

En `CommitmentTypeIndicationsHelper.java:68-132`, las declaraciones de compromiso
se estructuran mediante el prefijo indexado `commitmentTypeIndication`*n*:

* El identificador textual `commitmentTypeIndication`*n*`Identifier` se mapea a los
  siguientes OIDs normalizados PKCS#9 (`líneas 29-37`):
  * `"1"` -> `1.2.840.113549.1.9.16.6.1` (*Proof of origin*)
  * `"2"` -> `1.2.840.113549.1.9.16.6.2` (*Proof of receipt*)
  * `"3"` -> `1.2.840.113549.1.9.16.6.3` (*Proof of delivery*)
  * `"4"` -> `1.2.840.113549.1.9.16.6.4` (*Proof of sender*)
  * `"5"` -> `1.2.840.113549.1.9.16.6.5` (*Proof of approval*)
  * `"6"` -> `1.2.840.113549.1.9.16.6.6` (*Proof of creation*)
* Calificadores adicionales: `commitmentTypeIndication`*n*`CommitmentTypeQualifiers`
  separa OIDs por el carácter `'|'`.
* **Particularidad del bucle**: en `línea 98`, el bucle se evalúa como `for (int i = 0; i <= nCtis; i++)`,
  por lo que con `commitmentTypeIndications=1` busca los índices `0` y `1`.

---

### 3.3 Sellado de tiempo en CAdES

CAdES soporta la generación de firmas con sello de tiempo (CAdES-T) mediante
`AOCAdESSigner.java:543-550`. Si se especifica `tsaURL`, delega la conexión en
`TsaParams` y `CMSTimestamper`:
* `tsaURL`: URL del servidor RFC 3161.
* `tsaPolicy`: OID de política de sellado (por defecto `0.4.0.2023.1.1`).
* `tsaHashAlgorithm`: algoritmo de resumen para el sello (por defecto `SHA-512`).
* `tsaRequireCert`: booleano (`true` por defecto).
* `tsaUsr` y `tsaPwd`: credenciales HTTP básicas para la TSA.
* `tsaSslPkcs12File`: fichero PKCS#12 para autenticación mTLS contra la TSA.
* `tsaSslPkcs12FilePassword`: contraseña del fichero PKCS#12 mTLS.

---

## 4. Parámetros de firmas XAdES

El firmador de XAdES se implementa en `AOXAdESSigner` (`afirma-crypto-xades/src/main/java/es/gob/afirma/signers/xades/AOXAdESSigner.java`)
y sus constantes se declaran en `XAdESExtraParams` (`afirma-crypto-xades/src/main/java/es/gob/afirma/signers/xades/XAdESExtraParams.java`).

### 4.1 Catálogo general de claves XAdES

| Clave en `properties` | Constante en `XAdESExtraParams` | Tipo | Valor por defecto | Descripción |
|---|---|---|---|---|
| `format` | `FORMAT` | Cadena | `"XAdES Detached"` | Formato de empaquetado XML: `"XAdES Detached"`, `"XAdES Externally Detached"`, `"XAdES Enveloped"`, `"XAdES Enveloping"`. |
| `uri` | `URI` | Cadena URI | `null` | URI del documento a firmar, obligatorio en `XAdES Externally Detached`. No aplica a contrafirmas. |
| `nodeToSign` | `NODE_TOSIGN` | Cadena | `null` | Identificador del nodo XML concreto a firmar (el nodo debe contener un atributo llamado estrictamente `Id`). Si no se indica, se firma el documento completo. |
| `avoidEnvelopedTransformWhenSigningNode` | `AVOID_ENVELOPED_TRANSFORM_WHEN_SIGNING_NODE` | Booleano | `false` | Omite la transformación `Enveloped` cuando se ha indicado `nodeToSign`. Solo aplica a firmas Enveloped. |
| `insertEnvelopedSignatureOnNodeByXPath` | `INSERT_ENVELOPED_SIGNATURE_ON_NODE_BY_XPATH` | Cadena XPath | `null` | Expresión XPath v1 que apunta al nodo bajo el cual debe insertarse el elemento `<ds:Signature>`. Si devuelve varios, se usa el primero. |
| `avoidXpathExtraTransformsOnEnveloped` | `AVOID_XPATH_EXTRA_TRANSFORMS_ON_ENVELOPED` | Booleano | `false` | Evita añadir la transformación XPath2 que elimina firmas para posibilitar cofirmas enveloped. |
| `useManifest` | `USE_MANIFEST` | Booleano | `false` | Si es `true`, introduce un elemento `<ds:Manifest>` con las referencias en lugar de firmarlas directamente. |
| `precalculatedHashAlgorithm` | `PRECALCULATED_HASH_ALGORITHM` | Cadena | `null` | Algoritmo de hash cuando se proporciona una huella digital en lugar de los datos reales (solo utilizable en firmas `Externally Detached`). |
| `referencesDigestMethod` | `REFERENCES_DIGEST_METHOD` | Cadena URL | SHA-256 | URL del algoritmo de digest para las referencias XML: `http://www.w3.org/2001/04/xmlenc#sha256` (defecto), `http://www.w3.org/2001/04/xmlenc#sha512` o `http://www.w3.org/2000/09/xmldsig#sha1` (deprecado). |
| `canonicalizationAlgorithm` | `CANONICALIZATION_ALGORITHM` | Cadena URL | `2001/10/xml-exc-c14n#` | Algoritmo de canonicalización XML (por defecto Canonical XML inclusivo o exclusivo con o sin comentarios). |
| `xadesNamespace` | `XADES_NAMESPACE` | Cadena URL | `v1.3.2#` | Espacio de nombres de XAdES: por defecto `http://uri.etsi.org/01903/v1.3.2#`. |
| `signedPropertiesTypeUrl` | `SIGNED_PROPERTIES_TYPE_URL` | Cadena URL | `#SignedProperties` | URL del tipo de `SignedProperties`: por defecto `http://uri.etsi.org/01903#SignedProperties`. |
| `outputXmlEncoding` | `OUTPUT_XML_ENCODING` | Cadena | Según entrada / UTF-8 | Juego de caracteres para serializar el XML de salida (si la entrada es XML se auto-detecta). |
| `encoding` | `CONTENT_ENCODING` | Cadena URI | `null` | Atributo `Encoding` del elemento `<ds:Object>` (por ejemplo `http://www.w3.org/2000/09/xmldsig#base64`). |
| `contentTypeOid` | `CONTENT_TYPE_OID` | Cadena OID | `null` | OID que identifica el tipo de datos a firmar. |
| `mimeType` | `CONTENT_MIME_TYPE` | Cadena | Auto-detectado | Tipo MIME de los datos firmados. |
| `ignoreStyleSheets` | `IGNORE_STYLE_SHEETS` | Booleano | `false` | Si es `true`, ignora y no firma las hojas de estilo externas enlazadas en el XML. |
| `avoidBase64Transforms` | `AVOID_BASE64_TRANSFORMS` | Booleano | `false` | Si es `true`, omite declarar transformaciones Base64 automáticas. |
| `keepKeyInfoUnsigned` | `KEEP_KEYINFO_UNSIGNED` | Booleano | `false` | Si es `true`, no añade una referencia firmada al nodo `<ds:KeyInfo>`. Por defecto el `KeyInfo` sí se firma. |
| `addKeyInfoKeyValue` | `ADD_KEY_INFO_KEY_VALUE` | Booleano | `true` | Si es `true`, incluye el elemento `<ds:KeyValue>` (clave pública RSA/DSA/EC) dentro de `<ds:KeyInfo>`. |
| `addKeyInfoKeyName` | `ADD_KEY_INFO_KEY_NAME` | Booleano | `false` | Si es `true`, incluye el elemento `<ds:KeyName>` en `<ds:KeyInfo>` (usado obligatoriamente por ASiC-S). |
| `addKeyInfoX509IssuerSerial` | `ADD_KEY_INFO_X509_ISSUER_SERIAL` | Booleano | `false` | Si es `true`, incluye `<ds:X509IssuerSerial>` dentro de `<ds:X509Data>`. |
| `includeOnlySignningCertificate` | `INCLUDE_ONLY_SIGNNING_CERTIFICATE` | Booleano | `false` | Si es `true`, incluye solo el certificado firmante; si es `false`, incluye la cadena completa en `<ds:X509Data>`. |
| `signerClaimedRoles` | `SIGNER_CLAIMED_ROLES` | Cadena | `null` | Lista de cargos del firmante separados por `'\|'`. |
| `signatureProductionCity` | `SIGNATURE_PRODUCTION_CITY` | Cadena | `null` | Ciudad de firma en `SignatureProductionPlace`. |
| `signatureProductionStreetAddress` | `SIGNATURE_PRODUCCTION_STREET_ADDRESS` | Cadena | `null` | Calle o dirección (solo se incorpora en perfiles baseline / V2). |
| `signatureProductionProvince` | `SIGNATURE_PRODUCTION_PROVINCE` | Cadena | `null` | Provincia de firma en `SignatureProductionPlace`. |
| `signatureProductionPostalCode` | `SIGNATURE_PRODUCTION_POSTAL_CODE` | Cadena | `null` | Código postal en `SignatureProductionPlace`. |
| `signatureProductionCountry` | `SIGNATURE_PRODUCTION_COUNTRY` | Cadena | `null` | País en `SignatureProductionPlace`. |
| `policyIdentifier` | `POLICY_IDENTIFIER` | Cadena | `null` | Identificador de la política (URL o URN OID). |
| `policyIdentifierHash` | `POLICY_IDENTIFIER_HASH` | Cadena Base64 | `null` | Huella digital de la política en Base64. |
| `policyIdentifierHashAlgorithm` | `POLICY_IDENTIFIER_HASH_ALGORITHM` | Cadena | `null` | Algoritmo de resumen de la política (`SHA-1`, `SHA-256`, `SHA-512`). |
| `policyDescription` | `POLICY_DESCRIPTION` | Cadena | `""` | Descripción textual de la política. Si es `null`, se fuerza a `""` para evitar un fallo interno de JXAdES (`XAdESCommonMetadataUtil.java:240-243`). |
| `policyQualifier` | `POLICY_QUALIFIER` | Cadena URL | `null` | URL con la descripción de la política. |
| `commitmentTypeIndications` | `COMMITMENT_TYPE_INDICATIONS` | Entero | `null` | Número de declaraciones de compromiso a añadir. |
| `commitmentTypeIndication`*n*`Identifier` | `COMMITMENT_TYPE_INDICATION_IDENTIFIER` | Cadena | Ninguno | Identificador de compromiso (`1`..`6`). |
| `commitmentTypeIndication`*n*`Description` | `COMMITMENT_TYPE_INDICATION_DESCRIPTION` | Cadena | `null` | Descripción textual del compromiso. |
| `commitmentTypeIndication`*n*`DocumentationReferences` | `COMMITMENT_TYPE_INDICATION_DOCUMENTATION_REFERENCE` | Cadena | `null` | URLs documentales separadas por `'\|'`. |
| `commitmentTypeIndication`*n*`CommitmentTypeQualifiers` | `COMMITMENT_TYPE_INDICATION_QUALIFIERS` | Cadena | `null` | Indicadores textuales / OIDs separados por `'\|'`. |
| `avoidAGEPolicyIncompatibilities` | `AVOID_AGE_POLICY_INCOMPATIBILITIES` | Booleano | `null` | Al cofirmar en modo *Enveloping* con política AGE (incompatible), si está a `true` adapta la configuración para evitar el fallo; si no se indica, lanza `AGEPolicyIncompatibilityException` (`XAdESCoSigner.java:501-512`). |
| `confirmDifferentProfile` | `CONFIRM_DIFFERENT_PROFILE` | Booleano | `false` | Solicita confirmación si se multifirma con un perfil distinto al de la firma original. |
| `allowSignLTSignature` | `ALLOW_SIGN_LTS_SIGNATURES` | Booleano | `false` | Permite la multifirma sobre firmas de archivo (LTS). |

---

### 4.2 Transformaciones XML a medida (`xmlTransforms`)

En `afirma-crypto-core-xml/src/main/java/es/gob/afirma/signers/xml/Utils.java:159-245`,
XAdES permite especificar una cadena ordenada de transformaciones criptográficas
personalizadas sobre el XML:

* `xmlTransforms`: entero con el número total de transformaciones a aplicar.
* Parámetros indexados desde `0` hasta `numTransforms - 1`:
  * `xmlTransform`*n*`Type`: URI del algoritmo según W3C.
  * `xmlTransform`*n*`Subtype`: subtipo requerido por ciertos algoritmos.
  * `xmlTransform`*n*`Body`: expresión o sentencia de transformación.

Combinaciones admitidas:
1. **Transformación XPATH**:
   * `Type`: `http://www.w3.org/TR/1999/REC-xpath-19991116`
   * `Subtype`: no tiene.
   * `Body`: expresión XPath v1 (ej. `/bookstore/book[1]/title`).
2. **Transformación XPATH2 (Filter 2.0)**:
   * `Type`: `http://www.w3.org/2002/06/xmldsig-filter2`
   * `Subtype`: `"subtract"` (resta), `"intersect"` (intersección) o `"union"` (unión).
   * `Body`: expresión XPath2.
3. **Transformación BASE64**:
   * `Type`: `http://www.w3.org/2000/09/xmldsig#base64`
   * `Subtype` y `Body`: nulos (los datos se decodifican desde Base64 antes de firmarse).
4. **Transformación ENVELOPED**:
   * `Type`: `http://www.w3.org/2000/09/xmldsig#enveloped-signature`
   * `Subtype` y `Body`: nulos.

---

### 4.3 Múltiples referencias mediante `Manifest`

Cuando se firma un conjunto de documentos externos con `useManifest=true`,
`XAdESSigner.java:1538-1568` procesa las referencias indexadas **comenzando desde 1**:
* `uri`*i*: URI o ruta del recurso externo.
* `md`*i*: huella digital de los datos en Base64 (obligatoria para cada `uri`*i*).
* `mimeType`*i*: tipo MIME del recurso *i*.
* `contentTypeOid`*i*: OID de tipo del recurso *i*.
* `encoding`*i*: codificación del recurso *i*.

El ciclo se detiene en el primer índice `i` correlativo que no exista en el diccionario
`properties`.

---

## 5. Parámetros de FacturaE (`AOFacturaESigner`)

El firmador de facturas electrónicas `AOFacturaESigner` (`afirma-crypto-xades/.../AOFacturaESigner.java`)
es un envoltorio estricto sobre `AOXAdESSigner` configurado de forma fija en formato
`XAdES Enveloped` con el flag interno `facturaeSign=true` (`líneas 89-93`).

### 5.1 Lista blanca estricta (`ALLOWED_PARAMS`)
A diferencia de los demás firmadores de AutoFirma, que toleran parámetros adicionales
desconocidos, `AOFacturaESigner.getFacturaEExtraParams` (`líneas 57-87, 224-228`)
**filtra y descarta silenciosamente cualquier parámetro** que no pertenezca a su lista blanca:

```java
ALLOWED_PARAMS.add(XAdESExtraParams.SIGNATURE_PRODUCTION_CITY);
ALLOWED_PARAMS.add(XAdESExtraParams.SIGNATURE_PRODUCTION_PROVINCE);
ALLOWED_PARAMS.add(XAdESExtraParams.SIGNATURE_PRODUCTION_POSTAL_CODE);
ALLOWED_PARAMS.add(XAdESExtraParams.SIGNATURE_PRODUCTION_COUNTRY);
ALLOWED_PARAMS.add(XAdESExtraParams.XADES_NAMESPACE);
ALLOWED_PARAMS.add(XAdESExtraParams.SIGNED_PROPERTIES_TYPE_URL);
ALLOWED_PARAMS.add(XAdESExtraParams.POLICY_IDENTIFIER);
ALLOWED_PARAMS.add(XAdESExtraParams.POLICY_IDENTIFIER_HASH);
ALLOWED_PARAMS.add(XAdESExtraParams.POLICY_IDENTIFIER_HASH_ALGORITHM);
ALLOWED_PARAMS.add(XAdESExtraParams.POLICY_DESCRIPTION);
ALLOWED_PARAMS.add(XAdESExtraParams.POLICY_QUALIFIER);
ALLOWED_PARAMS.add(XAdESExtraParams.SIGNER_CLAIMED_ROLES);
ALLOWED_PARAMS.add(XAdESExtraParams.BATCH_SIGNATURE_ID);
```

Cualquier otra propiedad (como `nodeToSign`, `canonicalizationAlgorithm`, `referencesDigestMethod`,
`includeOnlySignningCertificate`, etc.) **es eliminada** antes de pasar a `AOXAdESSigner`.

### 5.2 Restricciones de valores en FacturaE

1. **Rol del firmante (`signerClaimedRoles`)**:
   En `AOFacturaESigner.java:184-199`:
   * Si no se especifica, toma por defecto el valor `"emisor"`.
   * Valores admitidos (en minúsculas): `"emisor"`, `"receptor"`, `"tercero"`, `"supplier"`, `"customer"`, `"third party"`.
   * Cualquier otro valor lanza una excepción fatal `IllegalArgumentException("El papel '...' no es valido para una factura electronica")`.
2. **Políticas de firma soportadas**:
   En `AOFacturaESigner.java:203-222`:
   * Únicamente admite dos políticas oficiales:
     * **FacturaE 3.1** (`POLICY_FACTURAE_31`):
       * `policyIdentifier`: `http://www.facturae.es/politica_de_firma_formato_facturae/politica_de_firma_formato_facturae_v3_1.pdf`
       * `policyIdentifierHash`: `Ohixl6upD6av8N7pEvDABhEL6hM=`
       * `policyIdentifierHashAlgorithm`: `SHA1`
     * **FacturaE 3.0** (`POLICY_FACTURAE_30`):
       * `policyIdentifier`: `http://www.facturae.es/politica de firma formato facturae/politica de firma formato facturae v3_0.pdf`
       * `policyIdentifierHash`: `xmfh8D/Ec/hHeE1IB4zPd61zHIY=`
       * `policyIdentifierHashAlgorithm`: `SHA1`
   * Si no se define `policyIdentifier`, AutoFirma inyecta automáticamente la política **FacturaE 3.1** (`líneas 217-222`).
   * Si se define cualquier otra política diferente a estas dos, lanza `IllegalArgumentException("La politica no esta soportada (solo se soporta FacturaE 3.0 y 3.1)")`.
3. **Prohibición de multifirma**:
   Las llamadas a `cosign(...)` y `countersign(...)` lanzan siempre `UnsupportedOperationException("No se soporta la cofirma de facturas")` (`líneas 146, 156, 168`).

---

## 6. Parámetros de contenedores ASiC (`ASiC-S`)

Los contenedores asociados de firma ASiC-S empaquetan en un archivo comprimido ZIP el
documento firmado junto con una firma CAdES o XAdES desprendida (*detached*).

### 6.1 ASiC-S con firma CAdES (`AOCAdESASiCSSigner`)
En `afirma-crypto-cades/.../asic/AOCAdESASiCSSigner.java`:
* **Forzado de modo**: inyecta obligatoriamente `mode=explicit` (`línea 61`).
* **Nombre del objeto de datos interno**:
  * `asicsFilename`: nombre que tendrá el fichero de datos dentro del contenedor ZIP (`CAdESASiCExtraParams.ASICS_FILENAME`).
  * Si no se indica, usa `"dataobject"` con la extensión correspondiente deducida del tipo MIME o `".bin"` por defecto (`líneas 69, 193`).
* Todas las demás claves de `properties` se transfieren directamente al motor de firma `AOCAdESSigner`.

### 6.2 ASiC-S con firma XAdES (`AOXAdESASiCSSigner`)
En `afirma-crypto-xades/.../asic/AOXAdESASiCSSigner.java`:
* **Configuración obligatoria forzada** (`setASiCProperties`, `líneas 278-295`):
  * `format`: fijado a `"XAdES Externally Detached"`.
  * `keepKeyInfoUnsigned`: fijado a `"true"`.
  * `addKeyInfoKeyName`: fijado a `"true"`.
  * `uri`: toma el nombre del fichero de datos (`asicsFilename` o `"dataobject.bin"`).
  * `RootXmlNodeName`: fijado a `"asic:XAdESSignatures"`.
  * `RootXmlNodeNamespace`: fijado a `"http://uri.etsi.org/02918/v1.2.1#"`.
  * `RootXmlNodeNamespacePrefix`: fijado a `"xmlns:asic"`.
* **Nombre del objeto interno**:
  * `asicsFilename`: nombre del fichero dentro del ZIP (`XAdESASiCExtraParams.ASICS_FILENAME`).
* El resto de propiedades de XAdES se entregan al motor `AOXAdESSigner`.

---

## Lo que el código no aclara

1. **Errata tipográfica en la constante de algoritmo de hash de política CAdES**:
   En `CAdESExtraParams.java:121`, el literal de la constante contiene un error ortográfico histórico:
   ```java
   public static final String POLICY_IDENTIFIER_HASH_ALGORITHM = "poliyIdentifierHashAlgorithm";
   ```
   Falta la letra `'c'` en `"poliy"`. Por el contrario, en `AdESPolicy.buildAdESPolicy` (`AdESPolicy.java:163`),
   el cargador de políticas lee la clave bien escrita: `extraParams.getProperty("policyIdentifierHashAlgorithm")`.
   Si un invocador utiliza la constante de CAdES o escribe `"poliyIdentifierHashAlgorithm"`, `AdESPolicy`
   no la encuentra, provocando que la firma se genere sin algoritmo de hash para la política o lance
   una excepción de incongruencia.
2. **Errata tipográfica en la constante de calle de XAdES**:
   En `XAdESExtraParams.java:292`:
   ```java
   public static final String SIGNATURE_PRODUCCTION_STREET_ADDRESS = "signatureProductionStreetAddress";
   ```
   El identificador en código Java tiene una doble `'c'` (`PRODUCCTION`), mientras que el literal
   de la clave de propiedad en la cadena es correcto (`"signatureProductionStreetAddress"`).
3. **Contradicción entre documentación y código en `signatureRotation` (PAdES)**:
   El Javadoc oficial en `PdfExtraParams.java:25-28` afirma textualmente:
   > *«Si se indica `true` el texto de la firma se rota 90 grados en sentido positivo. Si se indica `false` o no se indica, no se rota nada.»*
   
   Sin embargo, la implementación real en `PdfSessionManager.java:103-104` ejecuta:
   ```java
   Integer.parseInt(extraParams.getProperty(PdfExtraParams.SIGNATURE_ROTATION, DEFAULT_SIGNATURE_ROTATION));
   ```
   Si una aplicación web envía `signatureRotation=true` confiando en la documentación oficial,
   `Integer.parseInt` lanza una excepción no capturada `NumberFormatException`, abortando
   la firma. La aplicación únicamente tolera grados numéricos enteros (`"0"`, `"90"`, `"180"`, `"270"`),
   tal y como documenta el manual de línea de órdenes (`LineaComandos.html:605`).
4. **Parámetro no documentado `signaturePage=append`**:
   Ni `PdfExtraParams.java` ni `doc-files/extraparams.html` documentan la posibilidad de crear una
   página nueva en blanco. No obstante, en `PdfUtil.java:62, 712` y `PdfSessionManager.java:395`, si
   `signaturePage` contiene el valor `"append"`, AutoFirma inserta automáticamente una página en blanco
   al final del documento PDF con las medidas de la primera página y estampa allí la firma visible.
5. **Erratas en el Javadoc de macros de texto en PAdES**:
   En `PdfExtraParams.java:429-431`, el Javadoc documenta erróneamente la macro de organización como:
   ```java
   <dt><i><b>$$SUBJECTCN$$</b></i></dt>
   <dd>Organización (O, Organization) dentro del X.500 Principal...</dd>
   ```
   Duplicando `$$SUBJECTCN$$`. La implementación en `PdfVisibleAreasUtils.java:64, 313` demuestra que
   la macro real implementada es `$$ORGANIZATION$$`.
6. **Macros de Capa 2 no documentadas**:
   El fichero oficial de documentación `extraparams.html` de PAdES no menciona las macros
   `$$PSEUDONYM$$` (OID `2.5.4.65`), `$$OU$$` (primera unidad organizativa), `$$OUS$$` (todas las OUs
   separadas por coma) ni `$$TITLE$$` (cargo profesional), a pesar de estar plenamente implementadas
   en `PdfVisibleAreasUtils.java:55-58, 275-295`.
7. **Error *off-by-one* en el bucle de `commitmentTypeIndications`**:
   Tanto en `CommitmentTypeIndicationsHelper.java:98` (CAdES) como en `XAdESUtil.java:326` (XAdES),
   el bucle que lee los compromisos está programado como:
   ```java
   for (int i = 0; i <= nCtis; i++)
   ```
   Utilizando `<=` en lugar de `<`. Si se declara `commitmentTypeIndications=1`, el código busca tanto
   el índice `0` como el índice `1`, procesando hasta dos compromisos si ambos están definidos en las
   propiedades.
8. **Inconsistencia de origen en índices de colecciones**:
   Mientras que `xmlTransforms` en XML y `commitmentTypeIndications` en CAdES/XAdES comienzan a numerar
   sus elementos desde cero (`xmlTransform0...`, `commitmentTypeIndication0...`), la lectura de referencias
   múltiples para el `Manifest` de XAdES en `XAdESSigner.java:1538` arranca forzosamente desde uno
   (`int i = 1; while (extraParams.containsKey("uri" + i))`).
9. **Eliminación silenciosa y opaca en FacturaE**:
   `AOFacturaESigner.java:224-228` elimina cualquier parámetro que no esté en `ALLOWED_PARAMS` sin emitir
   ningún mensaje de advertencia ni registro en el log. Si un integrador intenta aplicar opciones
   legítimas de XAdES (como canonicalización personalizada o firma de nodo específico `nodeToSign`),
   estas desaparecen sin dejar rastro en la traza de ejecución.
10. **Tratamiento silencioso de errores de TSA en XAdES**:
    En `XAdESTspUtil.java:76-81`, al generar un sello de tiempo para XAdES, si la instanciación de
    `TsaParams` falla por parámetros incompletos o malformados, el bloque `catch` captura la excepción
    y devuelve el XML original sin sellar, sin emitir ningún error ni advertencia al usuario ni al servidor.
11. **Fallo estructural de JXAdES con `policyDescription` nula**:
    En `XAdESCommonMetadataUtil.java:240-242`, figura el comentario:
    ```java
    // Error en JXAdES. Si la descripcion es nula toda la firma falla.
    final String desc = description != null ? description : "";
    spi.setDescription(desc);
    ```
    Si no se proporciona `policyDescription`, AutoFirma se ve obligada a inyectar una cadena vacía `""`
    para evitar que la biblioteca subyacente JXAdES sufra una excepción de puntero nulo al componer
    las propiedades firmadas.
12. **Inutilización de validaciones baseline vía protocolo**:
    Como se demostró en el capítulo 11 (§3), `ProtocolInvocationLauncherSign.java:153` y
    `ProtocolInvocationLauncherSignAndSave.java:150` eliminan incondicionalmente la clave `profile`
    al inicio de la invocación. Por ello, las comprobaciones de perfiles *baseline* codificadas dentro
    de `AOPDFSigner.java:714`, `AOCAdESSigner.java:488` y `AOXAdESSigner.java` nunca llegan a ejecutarse
    cuando la invocación proviene del protocolo `afirma://`.
