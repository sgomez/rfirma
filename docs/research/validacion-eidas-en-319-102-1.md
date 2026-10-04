# Validación de firmas frente a eIDAS y ETSI EN 319 102-1

## Resumen

Comparo la validación de firmas de rFirma, que es en el fondo la de AutoFirma 1.9.2, con la norma de validación de eIDAS: **ETSI EN 319 102-1 V1.4.1**. La completan EN 319 142-1 (PAdES), TS 119 312 (algoritmos) y el Reglamento de Ejecución (UE) 2025/1945.

**Conclusión.** AutoFirma no valida firmas en el sentido de la norma.
- **Qué comprueba:** la integridad (con agujeros) y la vigencia del certificado a la hora actual.
- **Qué no hay:** cadena hasta un ancla de confianza, revocación, sellos de tiempo, prueba de existencia (POE) ni política criptográfica.
- **Cómo responde:** con dos valores («válida» / «no válida», más «caducada» en lo nuestro). Eso no encaja con los tres de la norma: `TOTAL-PASSED`, `TOTAL-FAILED`, `INDETERMINATE`.

La investigación ha encontrado además **un defecto nuestro, grave**: en `checkSignatures` de la sede, una CAdES manipulada sale como válida (§1). La norma lo prohíbe expresamente.

**Fuentes.** Todo lo normativo está contrastado con los textos oficiales, con cláusula y página:
- ETSI EN 319 102-1 V1.4.1 (2024-06), la validación.
- ETSI EN 319 142-1 V1.2.1 (2024-01), PAdES.
- ETSI TS 119 312 V2.1.1 (2026-06), suites criptográficas.
- Reglamento de Ejecución (UE) 2025/1945, de 29-09-2025 (DO L de 30-09-2025).
- Código: `clienteafirma` en el tag `v1.9.2` (`b4fe147c`) y nuestro `main`.

Lo que sigue sin texto oficial va marcado **[sin verificar]**.

---

## 1. Defecto: `checkSignatures` no verifica la criptografía de CAdES/CMS

- `rfirma-app/src-tauri/src/signing/adapters/engines.rs:62`: `ValidationEngine::verdict_of` fija `check_certificates: false`.
- `ValidationBridge` lo pasa a `ValidateBinarySignature.validate(byte[], boolean)`.
- En AutoFirma (`afirma-crypto-validation/.../ValidateBinarySignature.java:226-241`), **todo** `signer.verify(...)` está dentro de `if (checkCertificates) { cert.checkValidity(); ... }`. Con `false` no se verifica ni el valor de firma ni el `messageDigest`, y el resultado es OK.
- **El original no lo hace así.** `ProtocolInvocationLauncherSign.java:423` llama `validate(data, extraParams)`, y en CAdES y XML esa sobrecarga fuerza `checkCertificates=true` (`ValidateBinarySignature.java:64-67`). Es una divergencia introducida al separar el booleano (relacionado con #1345).
- `ValidationBridgeTest` no tiene ninguna CAdES alterada con `checkCertificates=false`.
- **La norma lo prohíbe.** EN 319 102-1 §5.1.4.1 (p. 43): «The set of validation constraints used for validation shall not force the SVA not to check a constraint that, when checked, would […] lead to a TOTAL-FAILED result». Además, la criptografía tiene prioridad sobre el resultado X.509 (§5.3.4 paso 5, p. 59): una firma alterada es `TOTAL-FAILED` sea cual sea el estado del certificado.

En XAdES, el mismo `false` quita la comprobación de caducidad que el original sí hacía. En PAdES coincide con el original.

**Propuesta:** separar «verificar integridad» de «comprobar vigencia», o llamar a `verifySign` con `true` y tratar la caducidad aparte, como ya hace `PreviousSignaturesBridge.integrityOf`. El primer paso es un test rojo con una CAdES implícita de un byte cambiado.

---

## 2. Qué piden las normas

### 2.1 Indicaciones (EN 319 102-1 §5.1.1, p. 31; §5.1.3, tablas 5-7, pp. 34-42)

- **`TOTAL-PASSED`:** pasan la criptografía y todas las restricciones, y por tanto «the signing certificate consequently has been found trustworthy» (tabla 5, p. 35). No lleva subindicación.
- **`TOTAL-FAILED`:** se da en tres casos, y lleva subindicación obligatoria.
  - falla la criptografía;
  - «it is proven that the signing certificate was invalid at the time of generation of the signature»;
  - la firma no es conforme a su norma base.
- **`INDETERMINATE`:** «the results of the performed checks do not allow to ascertain the signature to be TOTAL-PASSED or TOTAL-FAILED». Lleva una subindicación de la tabla 6 o un diagnóstico propio. La aceptación la decide «the DA, or beyond, by the user» (p. 32).
- **Subindicaciones (tabla 6):**
  - de `TOTAL-FAILED`: `FORMAT_FAILURE`, `HASH_FAILURE`, `SIG_CRYPTO_FAILURE`, `REVOKED`, `EXPIRED`, `NOT_YET_VALID`;
  - de `INDETERMINATE`: `SIG_CONSTRAINTS_FAILURE`, `CHAIN_CONSTRAINTS_FAILURE`, `CERTIFICATE_CHAIN_GENERAL_FAILURE`, `CRYPTO_CONSTRAINTS_FAILURE(_NO_POE)`, `POLICY_PROCESSING_ERROR`, `SIGNATURE_POLICY_NOT_AVAILABLE`, `TIMESTAMP_ORDER_FAILURE`, `NO_SIGNING_CERTIFICATE_FOUND`, `NO_CERTIFICATE_CHAIN_FOUND(_NO_POE)`, `REVOKED_NO_POE`, `REVOKED_CA_NO_POE`, `OUT_OF_BOUNDS_NOT_REVOKED`, `OUT_OF_BOUNDS_NO_POE`, `REVOCATION_OUT_OF_BOUNDS_NO_POE`, `NO_POE`, `TRY_LATER`, `SIGNED_DATA_NOT_FOUND`, `CUSTOM`.
- **La tabla 7 (p. 42)** dice cuáles admiten reintento.

### 2.2 Procesos y bloques (EN 319 102-1)

- **Bloques:**
  - formato (§5.2.2);
  - identificación del certificado firmante (§5.2.3);
  - contexto y política (§5.2.4);
  - frescura de la revocación (§5.2.5);
  - X.509 (§5.2.6);
  - criptografía (§5.2.7);
  - aceptación, SAV (§5.2.8);
  - presentación (§5.2.9).
- **Procesos:**
  - básico (§5.3);
  - sellos de tiempo (§5.4);
  - con tiempo y material LT (§5.5);
  - de archivo, LTA (§5.6), con PCV, VTS, extracción de POE y PSV (§5.6.2.1-4).
- **El mínimo es el proceso básico §5.3** (§5.1.2, p. 33), pero ya incluye cadena hasta un ancla y revocación. Por defecto se empieza por §5.6.3 (p. 31). LT y LTA son opcionales para el SVA.
- **§5.2.6, X.509** (pp. 48-52):
  - Es PKIX según RFC 5280 §6.1, salvo el modelo de validez y la vigencia del certificado firmante, que va en el paso 7.
  - **Las anclas de confianza son una entrada obligatoria** (tabla 12, p. 49).
  - Sin cadena: `NO_CERTIFICATE_CHAIN_FOUND` (paso 2a).
  - Revocado: `REVOKED_NO_POE` (4b).
  - CA revocada: `REVOKED_CA_NO_POE`, y se prueba otra cadena (4d).
  - Suspendido: `TRY_LATER` (4c).
  - Revocación poco fresca: `TRY_LATER` (4a).
  - Otro fallo de la ruta: `CERTIFICATE_CHAIN_GENERAL_FAILURE` (4e).
  - Restricciones de cadena: `CHAIN_CONSTRAINTS_FAILURE` (5).
  - Criptografía: `CRYPTO_CONSTRAINTS_FAILURE_NO_POE` (6).
  - Fuera de vigencia: `OUT_OF_BOUNDS_NOT_REVOKED` si se sabe que no está revocado; si no, `OUT_OF_BOUNDS_NO_POE` (7).
  - Emisor de la revocación fuera de vigencia: `REVOCATION_OUT_OF_BOUNDS_NO_POE` (8).
- **§5.2.3:** si ninguna referencia `signing-certificate(-v2)` casa por digest, `NO_SIGNING_CERTIFICATE_FOUND` (paso 2). Si no casa el `IssuerSerial`, solo hay aviso (paso 3, p. 45).
- **§5.2.7** (p. 53): `SIGNED_DATA_NOT_FOUND` (INDETERMINATE), `HASH_FAILURE`, `SIG_CRYPTO_FAILURE`.
- **§5.4, sellos** (pp. 59-60): un sello «is a Basic Signature». Se valida entero con §5.3 (cadena de la TSA, revocación, criptografía) contra «a trust anchor list applicable for validating time-stamps». El imprint se comprueba en §5.5.4 paso 3a. Un sello que no valida se descarta, salvo que la política lo exija.

### 2.3 La hora de la firma se prueba, no se declara

- **Definición** (EN 319 102-1 §3, p. 11): «claimed signing time: […] on its own does not provide independent evidence of the actual signing time».
- **`best-signature-time`** (§5.5.4 paso 1, p. 61) vale lo que aporte la DA como hora de existencia y, si no aporta nada, **la hora actual**. Solo la hacen retroceder las POE: sellos válidos, evidence records y, en LTA, la `/DSS` y los DocTimeStamp.
- **El `signing-time` declarado solo interviene en tres casos:**
  - **la DA o la política deciden tomarlo como hora real** (§5.5.2 notas 1-2, p. 61; §5.1.1 nota 2, p. 32);
  - **la restricción de retardo del sello** (§5.5.4 paso 5, p. 63);
  - **restricciones de SAV sobre el atributo** (§5.2.8.4.2.2, p. 55).
- **La norma no compara en ningún sitio el `signing-time` con la vigencia del certificado.**
- **En PAdES el atributo `signing-time` está prohibido.** EN 319 142-1 tabla 1 (p. 19): «shall not be present» en los cuatro niveles. La única hora declarada es `/M`, obligatoria (§6.3 g), p. 21).

### 2.4 Casos concretos

| Caso | Norma |
| --- | --- |
| Certificado caducado hoy, sin sello | `INDETERMINATE / OUT_OF_BOUNDS_NO_POE`, u `OUT_OF_BOUNDS_NOT_REVOKED` si se sabe que no está revocado (EN 319 102-1 §5.2.6 paso 7, p. 52). Nunca `TOTAL-FAILED` sin prueba |
| Caducado hoy, con sello de firma anterior al `notAfter` | Con §5.5 **no basta** salvo con `OUT_OF_BOUNDS_NOT_REVOKED` (4d). Con `OUT_OF_BOUNDS_NO_POE`, 4b devuelve la misma indicación (p. 62). Hace falta LTA con PSV (§5.6.2.4), que exige revocación del firmante con POE. Nota 9 (p. 64): el sello protege «against the revocation […] but not always against expiration» |
| *content-time-stamp* válido posterior al `notAfter` | `TOTAL-FAILED / EXPIRED` (§5.3.4 paso 4, p. 58) |
| `best-signature-time` anterior al `notBefore` | `TOTAL-FAILED / NOT_YET_VALID` (§5.5.4 4a.i, 4b, 4d; PSV) |
| Certificado aún no válido **hoy** | En §5.3: `OUT_OF_BOUNDS_*`. En §5.5 y §5.6, que son el camino por defecto, `best-signature-time` es la hora actual: **`TOTAL-FAILED / NOT_YET_VALID`** |
| `signing-time` declarado fuera de la vigencia | No es un fallo por sí solo. Puede dar `NOT_YET_VALID` si la política toma la hora declarada como real |
| Revocado hoy, sin POE | `INDETERMINATE / REVOKED_NO_POE`. CA revocada: `REVOKED_CA_NO_POE` (p. 51) |
| Revocado, con POE anterior a la revocación | `TOTAL-PASSED` solo si la POE es además anterior al `notAfter` (4a.ii). Si no, `OUT_OF_BOUNDS_NOT_REVOKED` (4a.iii) |
| *content-TS* posterior a la revocación | `TOTAL-FAILED / REVOKED` (p. 58) |
| Suspendido | `TRY_LATER`, salvo POE anterior a la suspensión (pp. 51, 63) |
| Revocación poco fresca | `TRY_LATER` (§5.2.5). Para QES y AdES-QC, el Reglamento 2025/1945 fija **24 h** para el certificado firmante (§5) |
| Sin cadena hasta un ancla | `INDETERMINATE / NO_CERTIFICATE_CHAIN_FOUND`, pero si la criptografía falla manda `TOTAL-FAILED` (§5.3.4 paso 5) |
| `signing-certificate(-v2)` no casa | `INDETERMINATE / NO_SIGNING_CERTIFICATE_FOUND`. En PAdES, proteger el certificado firmante es obligatorio en todos los niveles (EN 319 142-1 tabla 1, p. 19) |
| Hash de datos o valor de firma | `TOTAL-FAILED / HASH_FAILURE` o `SIG_CRYPTO_FAILURE` |
| Datos firmados no disponibles | `INDETERMINATE / SIGNED_DATA_NOT_FOUND` |
| Algoritmo débil | `CRYPTO_CONSTRAINTS_FAILURE(_NO_POE)`. Las fechas las pone la política (§5.1.4.3; TS 119 172-1 tabla A.2 fila p). TS 119 312 da los valores (§2.6) |
| Sellos desordenados | `TIMESTAMP_ORDER_FAILURE` (RFC 3161 §2.4.2; p. 63) |
| Política de firma inaccesible o con digest erróneo | `SIGNATURE_POLICY_NOT_AVAILABLE` / `POLICY_PROCESSING_ERROR` (pp. 38, 47) |
| Contrafirmas | EN 319 102-1 §5.2.8.4.2.6 (p. 56): se validan sobre el valor de firma de la madre. **Solo cuentan si la política lo pide.** Si no: «shall not consider the signature validation to having failed if the countersignature cannot be successfully validated». En PAdES baseline no caben (§2.5) |
| Cofirmas (paralelas) | Ninguna norma lo dice. Validarlas por separado es una inferencia razonable. En PAdES cada firma es una firma PDF propia con un solo `SignerInfo` |
| Nivel o perfil (B, T, LT, LTA) | No puede alterar el veredicto (anexo C.2, p. 84): «An SVA indicating a TOTAL-FAILED or INDETERMINATE result just because of a failed conformance check will not be conformant» |

### 2.5 PAdES (EN 319 142-1 V1.2.1)

- **Ámbito:** la validación está fuera, porque es de EN 319 102-1 (§1, p. 6). Lo que sigue son requisitos de formato que un validador ha de comprobar (`FORMAT_FAILURE`).
- **`/ByteRange`** (§6.3 k), p. 21): «The ByteRange shall cover the entire file, including the Signature Dictionary but excluding the PDF Signature itself (the entry with key Contents)».
  - Se refiere al fichero en la revisión firmada; las revisiones incrementales posteriores están previstas.
  - Para el DocTimeStamp, lo mismo (§5.4.3, p. 14).
  - El resto del manejo de firmas, en ISO 32000-1 §12.8 (§4.1 b)).
- **Un solo firmante por firma PDF** (§4.1 a), p. 9): «There shall only be a single signer (i.e. one single component of SignerInfo type …)».
- **Sin contrafirmas CMS:** los atributos de EN 319 122-1 que no están en la tabla 1 «shall not be present» (§6.3, p. 18).
- **SubFilter:**
  - firma: `ETSI.CAdES.detached`, obligatorio en todos los niveles (§6.3 l));
  - DocTimeStamp de B-LTA: `ETSI.RFC3161` (§6.3 y)).
  - Los subfiltros PKCS#1 y PKCS#7 de ISO 32000-1 «shall not be used» (§6.3 i)), así que `adbe.pkcs7.detached` no es PAdES baseline.
- **MD5 prohibido** (§6.2.1, p. 16): «In addition MD5 algorithm shall not be used as digest algorithm».
- **Revisiones posteriores:**
  - EN 319 142-1 no regula cómo se juzgan, y no menciona FieldMDP. Solo exime de DocMDP las revisiones de DSS/VRI (§5.4.2.3, p. 13: «DocMDP restrictions … shall not apply to incremental updates … containing a DSS dictionary») y los DocTimeStamp (§5.4.3, p. 14).
  - La nota de §5.4.2.3 aclara que P=1 «does not allow any change but allows Document Time-stamps».
  - La detección de cambios es de ISO 32000-1 §12.8.2.2.
- **B-LT y B-LTA:** el diccionario DSS es obligatorio, con todo el material de revocación (tabla 1, p. 20; §6.3 t), x)).

### 2.6 Algoritmos (TS 119 312 V2.1.1, 2026-06)

Es la primera versión con fechas de fin concretas (§1, p. 7). Notación (§3.4):
- **R** = recomendado, sin fecha de fin;
- **L** = legacy hasta el 31-12-2034;
- **L[aaaa]** = legacy hasta el 31-12 de ese año.

| Algoritmo | Estado | Dónde |
| --- | --- | --- |
| SHA-256/384/512, SHA3 | R | Tabla 1, p. 13 |
| SHA-224 | L[2028] en la tabla 1; «discontinued, < 2026-01-01» en la tabla D.1. **Las dos tablas se contradicen** | pp. 13, 35 |
| SHA-1 | Retirado. Restricción sugerida para validar: < 2012-08-01 | Tabla D.1, p. 35 |
| RIPEMD-160 | Retirado, < 2014-08-01 | Tabla D.1 |
| MD5 | No figura: los rotos «are not listed» (A.1, p. 27) | — |
| **RSA ≥ 1 900 y < 3 000 (RSA-2048)** | **L[2026]**: «shall not be used to issue new certificates after 2026-12-31». Los emitidos hasta esa fecha pueden durar hasta el 2028-12-31 | Tabla 6, §8.4, p. 20 |
| RSA < 3 000, para validar | Restricción sugerida «< 2026-01-01». **Contradice la tabla 6** | Tabla D.2, p. 36 |
| RSA ≥ 3 000 | R, sin fecha | Tabla 6 |
| RSA 1024 / 1536 | Retirados, < 2019-10-01 | Tabla D.2 |
| ECDSA P-256/384/521, brainpool | R | Tabla 3, p. 15 |
| ECDSA 224 | Retirado, < 2021-10-01 | Tabla D.2 |
| EdDSA | R | §6.2.2.4 |
| RSA-PSS | R | Tabla 2, p. 14 |
| **PKCS#1 v1.5** (sha256-with-rsa) | L (2034) | Tablas 2 y 4 |
| Validez más allá de 2030 | Híbrido o postcuántico | §6.2.2, p. 15 |

- **Para validar** (anexo D, p. 35), las fechas sirven como restricciones criptográficas de la política (TS 119 172-1 tabla A.2 fila p) «for the purpose of validating electronic signatures in the past, typically based on proof-of-existence information». Un algoritmo retirado solo vale con POE anterior a su fecha.
- **Un verificador** «shall support RSA-PKCS1v1_5, RSA-PSS, DSA, EdDSA, ECDSA» y SHA-256/384/512 (tablas A.1 y A.2, p. 28).
- **No es obligatoria por sí sola:** «There is no normative requirement on selection among the alternatives» (§1). Lo que manda es la política (anexo B).
- **Formato legible por máquina:** JSON y XML en `forge.etsi.org/rep/esi/x19_312_crypto_suites` (anexo C). TS 119 322 no se cita.

### 2.7 Informe y presentación (EN 319 102-1)

- **Contenido mínimo** (§5.1.3, p. 34): la indicación, la política, la hora de validación con los datos usados, y el proceso aplicado (5.3, 5.5 o 5.6.3).
- **Lo omitido se declara** (§5.1.4.1, p. 42): «the SVA shall return, in its final report to the DA, the list of checks that were disabled due to the policy».
- **A la DA se le entregan** todos los atributos firmados y sin firmar, también los no procesados (§5.3.4 paso 7).
- **Presentación** (§5.2.9, «should»): los datos firmados, el firmante, la hora de validación, qué va firmado y qué no, la política, el estado y su motivo, y en `INDETERMINATE` qué falta.
- **Atributo mal formado:** se trata como ausente (§5.2.8.4.1).
- **La revocación la aporta la DA** (nota 7, p. 51). No se obliga a consultar en línea, pero sí a usar la revocación más reciente que se tenga.
- **TS 119 102-2** (el informe XML) es un ejemplo informativo en EN 319 102-1, y el Reglamento 2025/1945 lo lista como norma de referencia (§5). No hay ninguna obligación directa de emitirlo.

---

## 3. Lo que hacemos hoy (AutoFirma + puente)

Tenemos tres caminos:

- **El veredicto de la sede** (`autofirma_validate_signatures`), con `check_certificates=false`.
- **`verify` sin `-v`**, que da los textos del original con `checkCertificates=true`.
- **El informe de firmas previas** (`PreviousSignaturesBridge`, ADR-0043).

| Comprobación | CAdES/CMS | PAdES | XAdES/FacturaE |
| --- | --- | --- | --- |
| Valor de firma y atributos firmados | **No en la sede** (§1). Sí en firmas previas y en `verify` | Sí (`PdfPKCS7.verify`) | Sí (núcleo de JSR-105 y cada `Reference`) |
| Vigencia del certificado | `checkValidity()` contra **ahora**; caducado y aún no válido dan KO | Contra ahora; con sello válido, contra `genTime` (solo en firmas previas) | Contra ahora, de **todos** los certificados del `KeyInfo`, CA incluidas |
| Certificado frente a `signing-time` | Solo de rebote: BouncyCastle lo rechaza → `CANT_VALIDATE_CERT` → «dañada» | No | No |
| Sello de tiempo | No | Solo en firmas previas: imprint y firma del token. **La TSA no se comprueba**: `pades-stamped-while-in-force.pdf`, sellada por una TSA hecha a mano, sale válida | No |
| Cadena y anclas de confianza | **No** | **No** | **No** |
| OCSP/CRL | **No** | **No** | **No** |
| `signing-certificate(-v2)` ligado al certificado | No | No | No (solo se mira si existe) |
| Clave ligada al certificado mostrado | Sí (SID) | Parcial (solo número de serie, sin emisor) | **No**: `KeyValueKeySelector` toma el primer `KeyValue` **o** el primer certificado [ver §4.3] |
| Cofirmas | `SignerInfo` de primer nivel | Todas las del AcroForm | Todos los `ds:Signature` |
| Contrafirmas | **No en la sede**; sí en firmas previas | No aplica | Sí, como `ds:Signature` anidados |
| `/ByteRange` cubre el documento | — | **No** (nadie llama `signatureCoversWholeDocument`) | — |
| SubFilter | — | Se aceptan `adbe.*` y, por tanto, PKCS#7 | — |
| Cambios posteriores a la firma | — | Heurística: formulario más render de las 10 primeras páginas (PSA) o comparación de flujos de contenido | — |
| DocMDP | — | Solo P=1, a firmas posteriores; los DocTimeStamp se saltan. P=2/3 y FieldMDP no | — |
| Algoritmos | Sin política: se aceptan SHA-1 y cualquier tamaño RSA | Igual, y además **MD5** (tabla de `PdfPKCS7`) | Solo lo que imponga la política segura del JDK **[sin verificar en GraalVM 25]** |
| Política de firma (EPES) | Solo se detecta | Solo se detecta | Solo se detecta |
| LT/LTA, `/DSS`, sellos de archivo | `SIGN_PROFILE_NOT_CHECKED`, que ignoramos | Igual | Igual |
| Resultado | 2-3 valores: válida / caducada / no válida. No hay lista de comprobaciones omitidas | | |

---

## 4. En qué nos diferenciamos

### 4.1 Diferencias de modelo: el veredicto puede ser engañoso

1. **«Válida» frente a `TOTAL-PASSED`.** `TOTAL-PASSED` exige un certificado encontrado fiable, con cadena hasta un ancla obligatoria (tablas 5 y 12). Hoy ponemos «Válida» a cualquier firma íntegra con un certificado vigente, **aunque sea autofirmado o esté revocado**. Con la norma, sin anclas ni revocación no se pasa de `INDETERMINATE`. Para AdES-QC, el Reglamento 2025/1945 exige además `TOTAL-PASSED` (§5).
2. **«Caducada» o «no válida» frente a `INDETERMINATE`.**
   - Un certificado caducado hoy y sin POE es `INDETERMINATE`, y su aceptación la decide la persona usuaria (p. 32). En la sede, un `CERTIFICATE_EXPIRED` de AutoFirma acaba como `INVALID`.
   - El caso contrario: un certificado **aún no válido hoy** sí es `TOTAL-FAILED / NOT_YET_VALID` en el camino por defecto. Ahí coincidimos con la norma.
3. **El sello de tiempo como prueba.** Medimos la vigencia contra *ahora*. El único sitio donde usamos el sello (PAdES en firmas previas) no exige que la TSA sea de confianza, cuando §5.4 pide validar el sello entero contra anclas de TSA: un sello antedatado «rescata» un certificado caducado. Además, la norma solo rescata un caducado con POE cuando se sabe que no está revocado, o con LTA y PSV.
4. **El `signing-time` declarado.** En CAdES, BouncyCastle lo trata como si fuera de confianza (rechaza si cae fuera de la vigencia) y acabamos en «dañada». La norma no le da ese valor, y «dañada» no es ninguna indicación suya.
5. **Comprobaciones apagadas sin decirlo.** §5.1.4.1 prohíbe apagar comprobaciones que darían `TOTAL-FAILED`, como hace §1, y obliga a declarar las que se omiten. No declaramos que no miramos ni la cadena ni la revocación.
6. **Contrafirmas.** En firmas previas, una contrafirma inválida baja la nota del documento. La norma dice que, salvo que la política lo pida, **no** hace fallar la firma madre. El veredicto de la sede no entra en las contrafirmas CAdES.
7. **Perfil.** Ignorar `SIGN_PROFILE_NOT_CHECKED` es correcto (anexo C.2). Lo que no basta es validar una LT o LTA como si fuera B.

### 4.2 Comprobaciones que la norma exige y no tenemos

- **Cadena PKIX hasta anclas de confianza** (EN 319 102-1 §5.2.6). Para QES y AdES-QC, las anclas son las listas de confianza de la UE: TS 119 612 V2.3.1, referencia normativa de TS 119 172-4 según el Reglamento 2025/1945.
- **Revocación** con frescura, y la distinción entre revocado, CA revocada y suspendido (§5.2.5, §5.2.6). Para QES y AdES-QC: `eitherCheck` y 24 h.
- **Validación completa de sellos de tiempo** y de su orden (§5.4, §5.5.4).
- **`signing-certificate(-v2)`** contra el certificado (§5.2.3).
- **Política criptográfica** (§5.1.4.3) con las fechas de TS 119 312 V2.1.1 (§2.6):
  - SHA-1 y RSA-1024 solo con POE anterior a su fecha;
  - MD5 nunca (prohibido en PAdES por EN 319 142-1 §6.2.1);
  - y el caso de **RSA-2048**, el de los certificados de la FNMT. Es legacy hasta el 31-12-2026 para emitir certificados, que pueden durar hasta el 2028-12-31. Pero la tabla D.2 sugiere «< 2026-01-01» para validar: **la propia TS se contradice**, y hay que decidir qué fecha aplicamos.
- **Política de firma** declarada (§5.2.4).
- **Informe** con el contenido mínimo de §5.1.3 y la lista de comprobaciones omitidas.
- **Opcional:** LT/LTA y POE (§5.5, §5.6).
- **Cualificación** (QESig, AdESig-QC), solo si quisiéramos decirlo: TS 119 172-4 con las adaptaciones del Reglamento 2025/1945. TS 119 615 no aparece en el Reglamento.

### 4.3 Agujeros de integridad y de formato

- **§1:** CAdES sin verificar en la sede.
- **PAdES:** no se comprueba que el `/ByteRange` cubra el fichero entero, que EN 319 142-1 §6.3 k) exige.
- **PAdES:** se acepta MD5, que §6.2.1 prohíbe.
- **PAdES:** no se exige `ETSI.CAdES.detached`.
- **PAdES:** el orden de `getSignatureNames` viene de un `HashMap`, y `ValidatePdfSignature` toma `get(0)` como la última firma. Con varias firmas puede comparar con la revisión equivocada. `PreviousSignaturesBridge.latestRevisionName` ya lo corrige; el veredicto de la sede no.
- **PAdES:** `checkPdfShadowAttack` atrapa cualquier excepción y devuelve «sin sospecha». **[Inferencia]** Las páginas añadidas o los cambios que no se pintan no se detectan. **[Sin verificar]** Tampoco sabemos cómo se comporta en la imagen nativa: `rfirma-native-bridge/AGENTS.md:105` dice que no se llama.
- **PAdES:** cualquier control nuevo de revisiones debe respetar las exenciones de §5.4.2.3 y §5.4.3: DSS y DocTimeStamp no rompen un DocMDP P=1.
- **XAdES [inferencia, falta reproducir]:** como la clave no está ligada al certificado, un `KeyValue` del atacante delante del certificado de otra persona en `X509Data` verificaría, y se mostraría a esa persona como firmante.
- **XAdES:** `CustomUriDereferencer` puede descargar por HTTP las URI `http(s)://` durante la validación. Choca con ADR-0041 («sin red»).
- **CAdES detached sin datos:** en firmas previas, el `CMSSignerDigestMismatchException` se descarta y no se verifica nada. La norma pide `SIGNED_DATA_NOT_FOUND`, aunque §5.2.7 nota 2 deja abierto que esa comprobación la haga la DA.

---

## 5. Contexto legal: Reglamento de Ejecución (UE) 2025/1945

- **Base y vigor:**
  - Base: arts. 32(3), 32a(3), 40 y 40a de 910/2014, modificado por 2024/1183.
  - Entra en vigor a los 20 días de su publicación (art. 2), que da el 20-10-2025.
  - **Anexo I**: QES y QESeal (arts. 32(3) y 40). **Anexo II**: AdES y AdESeal basadas en certificado cualificado (arts. 32a(3) y 40a).
- **Normas listadas** (pp. 3 y 5): solo **TS 119 172-4 V1.1.1** y **TS 119 102-2 V1.4.1**.
  - EN 319 102-1 V1.4.1 entra como referencia normativa [1] de TS 119 172-4: «All references to 'ETSI TS 119 102-1 [1]' shall be understood as references to 'ETSI EN 319 102-1 [1]'».
  - También entran TS 119 612 V2.3.1 (listas de confianza) [2] y TS 119 101 V1.1.1 [13].
- **Adaptaciones** (todas sobre TS 119 172-4):
  - **Revocación** (REQ-4.2-03 c)): `eitherCheck`, y frescura de «a maximum value of 24 hours for the signing certificate». No aplica si el certificado es ancla de confianza.
  - **REQ-4.3-02:** «Signature validation applications shall be compliant with ETSI TS 119 101».
  - **REQ-4.4.2-03:** si falla la comprobación de cualificación, la firma queda «technically determined as indeterminate».
  - **Anexo II, REQ-4.4.2-06:** es AdES-QC si el certificado es cualificado «at the best signature time» y el resultado es `TOTAL-PASSED`.
- **A quién obliga.** Es una **presunción de conformidad**, no una obligación (considerando 2): se presume que cumplen los arts. 32(1), 32a y 40/40a «where the processes for the validation […] comply with the technical standards set out in this Regulation». No nombra destinatarios.
  - **rFirma no queda obligada por este texto.**
  - Pero es la vara de medir si algún día quisiéramos decir «firma cualificada» o «válida» en sentido eIDAS. Usa el mismo vocabulario de tres valores que propone §6.
- **[Sin verificar]:**
  - CIR 2025/1942, servicios cualificados de validación (TS 119 441).
  - CIR 2026/248, que derogaría la Decisión 2015/1506 y fijaría los formatos EN para el sector público desde el 23-02-2027.
  - El texto de los arts. 32, 32a y 33.
  - La Política AGE v1.9 y la NTI de 2016.

---

## 6. Decisiones que propongo abrir

1. **Ya:** arreglar §1, con un test rojo de CAdES alterada en `checkSignatures`. Lo exige EN 319 102-1 §5.1.4.1.
2. **Ya, en PAdES:**
   - `/ByteRange` sobre todo el fichero de la revisión firmada (EN 319 142-1 §6.3 k));
   - rechazo de MD5 (§6.2.1);
   - orden de revisiones en el veredicto de la sede.
3. Reproducir el caso `KeyValue` de XAdES y, si se confirma, ligar la clave al certificado.
4. **Vocabulario de tres valores** (un ADR que enmiende ADR-0043):
   - `TOTAL-FAILED` → «No válida», con su motivo;
   - `INDETERMINATE` → «No se puede confirmar», con qué falta;
   - «Válida» solo si hay cadena y revocación. Mientras tanto, «Íntegra» o «Firma intacta; certificado sin comprobar».
   - Y declarar en el informe las comprobaciones que no hacemos (§5.1.4.1).
5. **Contrafirmas:** que una inválida no tumbe a la madre (§5.2.8.4.2.6), sino que se informe aparte.
6. **Política criptográfica de validación** declarada (§5.1.4.3), tomando como fuente el JSON del anexo C de TS 119 312 V2.1.1. Decidir de forma explícita las contradicciones de RSA-2048 (tabla 6 frente a D.2) y SHA-224 (tabla 1 frente a D.1). Si no, toda firma FNMT sin POE anterior a 2026 saldría `CRYPTO_CONSTRAINTS_FAILURE_NO_POE`.
7. **Alcance:**
   - ¿Delegamos en un validador externo (@firma/VALIDe, o DSS en la imagen nativa) o implementamos cadena, revocación y anclas?
   - Si apuntamos a «aplicación de validación» en el sentido del Reglamento 2025/1945, hay que leer **TS 119 101** (REQ-4.3-02) y **TS 119 172-4**.
   - ¿Damos una salida de §5.1.3, o la de TS 119 102-2, en `verify -json`?

## Pendiente de leer

- **TS 119 172-4 V1.1.1** y **TS 119 101 V1.1.1**, las normas que el Reglamento 2025/1945 aplica.
- **TS 119 172-1**, tabla A.2: el formato de las restricciones de la política.
- **ISO 32000-1 §12.8.2.2**: DocMDP y FieldMDP.
- **CIR 2025/1942, CIR 2026/248 y los arts. 32, 32a y 33** de eIDAS consolidado.
- **La Política AGE v1.9 y la NTI de 2016.**
