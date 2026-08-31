# Mecanismo PKCS#11 para firmar los bytes `PRE` (firma trifásica)

Investigación contra el código fuente original de `clienteafirma` (ruta local `/home/sergio/Developer/SideProjects/clienteafirma`, módulos `afirma-crypto-cadestri-client`, `afirma-crypto-padestri-client` y `afirma-core`) y contra la documentación del crate `cryptoki` en docs.rs. Responde a las cinco preguntas del issue [#8](https://github.com/sgomez/rfirma/issues/8), que depende del resultado del issue #3 (documentado en `docs/research/pades-triphase-contract.md`), del que parte: `TriphaseData["PRE"] = Base64(DER(SignedAttributes CAdES))`.

Todas las citas de código Java son `fichero:línea` relativas a la raíz de `clienteafirma`.

## 1. Qué algoritmo aplica el cliente original sobre los bytes de `PRE` (confirmado con código)

Los módulos `afirma-crypto-padestri-client` y `afirma-crypto-cadestri-client` **no contienen ninguna llamada a `Signature.getInstance`**; solo transportan el buffer `PRE` por HTTP hacia el servicio de prefirma/postfirma. La firma local ocurre en `afirma-core`, invocada desde `afirma-crypto-cadestri-client`:

- `afirma-crypto-cadestri-client/src/main/java/es/gob/afirma/signers/cadestri/client/AOCAdESTriPhaseSigner.java:72-96` — el flujo de firma delega en `TriphaseDataSigner` (import línea 39) y `AOPkcs1Signer` (import línea 33).

- `afirma-core/src/main/java/es/gob/afirma/core/signers/TriphaseDataSigner.java`, método `doSign(...)`:
  - `:28` — `PROPERTY_NAME_PRESIGN = "PRE"`.
  - `:31` — `PROPERTY_NAME_PKCS1_SIGN = "PK1"`.
  - `:66` — `base64PreSign = signConfig.getProperty(PROPERTY_NAME_PRESIGN);`
  - `:74` — `preSign = Base64.decode(base64PreSign);` (bytes DER crudos, **sin hashear**).
  - `:78` — `signatureAlgorithm = AOSignConstants.composeSignatureAlgorithmName(algorithm, key.getAlgorithm());`
  - `:80-86`:
    ```java
    final byte[] pkcs1sign = signer.sign(
        preSign,
        signatureAlgorithm,
        key,
        certChain,
        extraParams
    );
    ```
  - `:89` — `signConfig.addProperty(PROPERTY_NAME_PKCS1_SIGN, Base64.encode(pkcs1sign));`

- `afirma-core/src/main/java/es/gob/afirma/core/signers/AOPkcs1Signer.java`, método `sign(...)`:
  - `:85` — `algorithmName = AOSignConstants.composeSignatureAlgorithmName(algorithm, keyType);`
  - `:87` — `sig = p != null ? Signature.getInstance(algorithmName, p) : Signature.getInstance(algorithmName);`
  - `:93` — `sig.initSign(key);`
  - `:100` — `sig.update(data);` — `data` es directamente `preSign`, es decir **los bytes DER completos de `PRE`, sin hash previo**.
  - `:108` — `signature = sig.sign();`

**Conclusión (con cita literal, no inferida):** el proveedor JCA recibe un nombre de algoritmo compuesto (típicamente `"SHA256withRSA"`, construido por `AOSignConstants.composeSignatureAlgorithmName`, `afirma-core/.../AOSignConstants.java:365-388`) y llama a `sig.update(preSign)` + `sig.sign()` sobre el buffer DER completo. El proveedor hashea internamente esos bytes DER como si fueran "los datos a firmar"; no se construye ningún `DigestInfo` a mano en el cliente ni se pasa un hash ya calculado.

## 2. Mecanismo `cryptoki` correspondiente

Es **`CKM_SHA256_RSA_PKCS`** (o el equivalente con el hash que corresponda a `signAlgorithm`; SHA-256 es el caso previsible con los certificados FNMT/DNIe habituales), no `CKM_RSA_PKCS`.

Razonamiento, directamente derivado del punto 1:

- `sig.update(preSign)` + `sig.sign()` con `algorithmName = "SHA256withRSA"` en JCA es, por definición, un mecanismo **compuesto**: el proveedor calcula `SHA-256(preSign)`, construye el `DigestInfo` ASN.1 internamente y aplica el padding PKCS#1 v1.5 antes de elevar a la clave privada. Ese es exactamente el contrato de `CKM_SHA256_RSA_PKCS` en PKCS#11: recibe el buffer de datos completo y hace hash+relleno+firma en una sola operación.
- `CKM_RSA_PKCS`, en cambio, firma el bloque que se le pasa **tal cual**, sin hashear ni envolver en `DigestInfo`. Si Rust invocase `CKM_RSA_PKCS` sobre `preSign` sin construir antes el `DigestInfo` DER (`SEQUENCE { AlgorithmIdentifier, OCTET STRING hash }` según PKCS#1 v1.5 / RFC 8017 §9.2), el resultado sería una firma RSA matemáticamente válida pero sobre un bloque de bytes que **ningún verificador CAdES/PAdES reconoce como el `DigestInfo` esperado** — el validador recalcula `SHA-256(SignedAttributes DER)`, lo envuelve en su propio `DigestInfo`, y compara tras descifrar la firma con la clave pública; si Rust firmó los bytes DER "en crudo" con `CKM_RSA_PKCS` (sin ese envoltorio, o peor, firmando un hash de `preSign` sin decir de qué algoritmo es), el resultado descifrado no coincidirá byte a byte con lo que el validador reconstruye. El PDF "parecería" firmado (misma estructura, mismo `ByteRange`) pero la firma criptográfica sería inválida.
- Usar `CKM_RSA_PKCS` sería además una reimplementación innecesaria y frágil del propio `DigestInfo` en Rust, cuando el HSM/token ya sabe hacerlo de forma estándar con el mecanismo compuesto — y es exactamente lo que hace el cliente Java de referencia.

En el crate `cryptoki` (docs.rs, verificado por WebFetch): el enum `Mechanism` expone `Mechanism::Sha256RsaPkcs` (→ `CKM_SHA256_RSA_PKCS`) y `Mechanism::RsaPkcs` (→ `CKM_RSA_PKCS`) como variantes distintas. La firma es la que corresponde al primero.

## 3. Codificación exacta de entrada y salida

- **Entrada a `Session::sign(&mechanism, key_handle, data)`:** `data` = los bytes DER crudos de `SignedAttributes` (el resultado de `Base64.decode(TriphaseData["PRE"])`), **sin hashear previamente**. Es decir, Rust debe decodificar Base64 del campo `PRE` y pasar ese buffer completo, tal como hace `TriphaseDataSigner.java:74` con `preSign`.
- **Firma del crate:**
  ```rust
  pub fn sign(&self, mechanism: &Mechanism<'_>, key: ObjectHandle, data: &[u8]) -> Result<Vec<u8>>
  ```
  Con `mechanism = Mechanism::Sha256RsaPkcs`, `data` es ese buffer sin hashear (comportamiento estándar PKCS#11 para un mecanismo compuesto).
- **Salida:** el `Vec<u8>` devuelto por `sign()` son los bytes crudos de la firma RSA-PKCS#1 v1.5 (mismo tamaño que el módulo RSA, p. ej. 256 bytes para una clave de 2048 bits). Rust debe codificarlos en **Base64** — igual que `TriphaseDataSigner.java:89` (`Base64.encode(pkcs1sign)`) — y depositarlos en el campo **`PK1`** del mismo `TriSign` (mismo `Id`) del `TriphaseData`, junto con `PRE`, `PID` y `TIME` sin modificar, tal como quedó documentado en el issue #3.

## 4. Qué cambia con un certificado de curva elíptica

El propio `afirma-core` contempla ECDSA en el mismo punto de composición de algoritmo:

- `afirma-core/src/main/java/es/gob/afirma/core/signers/AOSignConstants.java:381-383`:
  ```java
  } else if (keyType.startsWith("EC")) {
      suffix = "withECDSA";
  }
  ```
  El flujo posterior es idéntico al de RSA: `sig.update(preSign)` + `sig.sign()` con `"SHA256withECDSA"` (o el hash que corresponda), es decir, también mecanismo **compuesto** sobre el buffer DER completo sin hash previo.

En `cryptoki` existen las variantes compuestas equivalentes: `Mechanism::EcdsaSha256` (y `EcdsaSha1`, `EcdsaSha224`, `EcdsaSha384`, `EcdsaSha512`), análogas a `Sha256RsaPkcs`, frente a `Mechanism::Ecdsa` (→ `CKM_ECDSA` puro, que firma sobre un hash ya calculado, sin hashear internamente — el equivalente EC de `CKM_RSA_PKCS`). Por tanto la decisión tomada aquí (mecanismo compuesto, buffer sin hashear) **no cierra la puerta a ECDSA**: se generaliza sustituyendo `Sha256RsaPkcs` por `EcdsaSha256` sin cambiar la forma de invocar `sign()`.

**Aviso importante, no verificado contra el código de este repo ni contra la documentación fetcheada (marcado explícitamente como conocimiento del estándar PKCS#11, a confirmar cuando se implemente):** el formato de salida de `CKM_ECDSA` y de las variantes compuestas `CKM_ECDSA_SHAxxx` en PKCS#11 es la concatenación cruda de `r` y `s` (cada uno de longitud fija, del tamaño del orden de la curva), **no** la codificación `SEQUENCE { INTEGER r, INTEGER s }` en DER que exige `SignatureValue` en CMS/CAdES (RFC 5480 / X9.62). Si esto es así (habría que confirmarlo con una prueba real contra el token/HSM disponible antes de dar por cerrado el soporte EC), el bridge Rust necesitaría un paso adicional de reempaquetado DER para `r`/`s` que **no existe** en el camino RSA. Esto no bloquea la decisión de mecanismo tomada en la pregunta 2, pero sí es trabajo extra pendiente si se soporta EC. **No determinado en esta investigación**: si el certificado FNMT/DNIe de pruebas disponible es RSA o EC — si es RSA, este punto queda fuera del camino crítico inmediato.

## 5. Comprobación ejecutable y barata antes de tener el recorrido completo montado

Objetivo: verificar que la firma cruda producida por PKCS#11 (o por cualquier prototipo de firma) es válida contra la clave pública del certificado, **sin** necesidad de tener montado el ensamblado CAdES/PAdES completo ni un validador de PDF.

Con RSA, la propia operación de verificación de OpenSSL reproduce el mismo mecanismo compuesto (hashea internamente y compara tras aplicar la clave pública), así que es la contrapartida exacta de `CKM_SHA256_RSA_PKCS`:

```sh
# 1. Extraer la clave pública del certificado del firmante
openssl x509 -in firmante.pem -pubkey -noout > pub.pem

# 2. Tener a mano:
#    - presign.der  → los bytes decodificados de Base64(TriphaseData["PRE"])
#    - firma.bin     → los bytes crudos de la firma devuelta por Session::sign(...)
#                       (antes de volver a codificarlos en Base64 para meterlos en PK1)

# 3. Verificar: OpenSSL calcula SHA-256(presign.der), aplica la clave pública
#    a firma.bin y compara — exactamente lo que hará cualquier validador CAdES
#    al comprobar la firma sobre los SignedAttributes.
openssl dgst -sha256 -verify pub.pem -signature firma.bin presign.der
# → "Verified OK" si el mecanismo, el buffer de entrada y la clave son correctos.
```

Si el certificado es EC, el mismo comando `openssl dgst -sha256 -verify` funciona igual (detecta el tipo de clave desde `pub.pem`), **siempre que `firma.bin` ya esté en formato DER `SEQUENCE{r,s}`** — lo cual, según el aviso de la pregunta 4, puede requerir el reempaquetado antes de pasarlo a OpenSSL si el token devuelve `r||s` concatenados en crudo.

Este comando es deliberadamente barato: no requiere el `TriphaseData` completo, ni el PDF, ni el servidor de prefirma/postfirma — solo el buffer `PRE` ya decodificado (que puede capturarse de una prefirma real o simularse) y la firma cruda que produzca el prototipo de Rust. Si `openssl dgst -verify` no dice `Verified OK`, el mecanismo, el buffer de entrada o el orden de codificación están mal **antes** de gastar tiempo depurando el ensamblado PAdES completo.

## Resumen de lo no determinado

- La longitud exacta de línea de `AOPkcs1Signer.java` y `TriphaseDataSigner.java` se ha citado a partir de una lectura por `grep`/`sed` dirigida a las llamadas relevantes, no de una lectura completa del fichero; los números de línea reportados corresponden a lo observado en esta investigación.
- No se ha confirmado si el servidor de prefirma (fuera de este repositorio cliente) genera en producción un `PRE` compatible con firmas EC — solo se ha confirmado el soporte en el código cliente genérico (`afirma-core`).
- El formato de salida (`r||s` crudo vs. DER `SEQUENCE`) de los mecanismos ECDSA de PKCS#11 no se ha verificado contra la documentación de `cryptoki` fetcheada en esta investigación ni contra código de este repo; es conocimiento general del estándar PKCS#11 pendiente de confirmar con una prueba real.
- Qué certificado (RSA o EC) es el disponible para pruebas (FNMT/DNIe) no se ha determinado en esta investigación; condiciona si el punto anterior es bloqueante a corto plazo.
