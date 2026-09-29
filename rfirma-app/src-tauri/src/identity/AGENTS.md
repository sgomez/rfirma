# Mapa de `identity`: quién firma

El contexto de la **identidad**: qué certificados hay, cuál se usa y cómo entra o
sale un `.p12`. Aquí no se firma ningún documento, solo se dice quién puede.
Su adaptador de verdad es `adapters/pkcs11/`, la única parte del backend que
habla con el token; en Windows, también `adapters/windows_store/`. Rutas relativas a `src/identity/`.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz: `IdentityRoot`, la fachada que usan los vecinos y el `Signer` de `signing/ports.rs` sobre cualquier `Token`. Pruebas en `tests.rs`. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `application/tests.rs` | Los andamios de la grada A que comparten todos los contextos: `NoToken`, `NoMemory`, `a_certificate`, `a_usable_certificate` y `listed_from`. Solo en pruebas. |
| `adapters/pkcs11/mod.rs` | La capa PKCS#11, y `RealToken`, el adaptador de producción del puerto `Token` en Linux. |
| `adapters/pkcs11/listing.rs` | Recorre las ranuras de un almacén y filtra los certificados con clave privada emparejada. |
| `adapters/pkcs11/mechanism.rs` | Elige el mecanismo de firma que ofrece la ranura y firma con la clave privada. |
| `adapters/pkcs11/session.rs` | Abre el módulo PKCS#11, cachea su contexto y localiza ranura y clave privada. |
| `adapters/pkcs11/nss.rs` | Cómo entra un `.p12` en un almacén NSS propio, con el PKCS#12 de `libsmime3`, y los símbolos NSS de bajo nivel que comparte con `adapters/pkcs11/removal.rs`. Declara `NssHost` y `RealNssHost`, para `site/adapters/nss.rs`. Pruebas en `adapters/pkcs11/nss/tests.rs`. |
| `adapters/pkcs11/removal.rs` | Cómo se borra de un almacén NSS un certificado y su clave por su `CKA_ID`, sin tocar los demás. |
| `adapters/pkcs11/stores.rs` | Dónde se buscan los certificados, incluidos los `.p12` instalados, y qué módulo descubierto es la biblioteca que nombra la sede (ADR-0022). Pruebas en `adapters/pkcs11/stores/tests.rs`. |
| `adapters/pkcs11/p11kit.rs` | Los módulos PKCS#11 que la instalación registra en p11-kit, leídos de sus ficheros `.module`; no carga ninguno. Pruebas en `adapters/pkcs11/p11kit/tests.rs`. |
| `adapters/failures.rs` | La única traducción de lo que va mal en identidad a la vista de la ventana y al código de la sede (ADR-0009). Pruebas en `adapters/failures/tests.rs`. |
| `adapters/tauri.rs` | Las tres órdenes de identidad: listar certificados, instalar y quitar un `.p12`. |
| `adapters/views.rs` | Lo que cruza a la ventana: `CertificateView`, `StatusView` y `SecretView`. Pruebas en `adapters/views/tests.rs`. |
| `application/certificates.rs` | Qué certificados hay, cuál se recordó e instalar o quitar un `.p12`; `ListedCertificates` es el último listado, con las asas de `documents/domain/handles.rs`. Pruebas en `application/certificates/tests/mod.rs` y `application/certificates/tests/removal.rs`. |
| `application/certificates/tests/copies.rs` | Las pruebas de la fila única de un certificado que está en varios almacenes y de la copia con la que firma. |
| `domain/algorithm.rs` | El algoritmo de firma que se pide por su nombre, la clase de clave que exige y el mecanismo PKCS#11 con el que se cumple. Pruebas en `domain/algorithm/tests.rs`. |
| `domain/certificate.rs` | El certificado tal y como sale del token, y `ListedCertificate`, la fila con su asa. Pruebas en `domain/certificate/tests.rs`. |
| `domain/chain.rs` | Los emisores que acompañan al firmante en la cadena de certificación que viaja dentro de la firma. Pruebas en `domain/chain/tests.rs`. |
| `domain/ecdsa.rs` | Lo que la curva elíptica exige y RSA no: el resumen que firma el mecanismo crudo y el `r`/`s` del token reempaquetado en DER. Pruebas en `domain/ecdsa/tests.rs`. |
| `domain/error.rs` | Las situaciones del token (ADR-0009) y el aviso de que falta `libnss3.so`. Pruebas en `domain/error/tests.rs`. |
| `domain/holder.rs` | Quién es el titular, leído del nombre distinguido (RFC 4514), y `StampedHolder`, lo que estampa el recuadro. No sirve para el `Display` de `x509_cert::Name`: eso se lee del DER con `TokenCertificate`. Pruebas en `domain/holder/tests.rs`. |
| `domain/keyring.rs` | `generate_pin`, el PIN aleatorio y largo del Almacén de rFirma, y `KeyringError` (ADR-0034). Pruebas en `domain/keyring/tests.rs`. |
| `domain/protected_secret.rs` | Secreto protegido en memoria con bloqueo físico y borrado seguro en drop. Pruebas en `domain/protected_secret/tests.rs`. |
| `domain/secret.rs` | Cómo se le pide el secreto a cada almacén: sin sesión, por pantalla o en el teclado del lector. Pruebas en `domain/secret/tests.rs`. |
| `domain/store.rs` | Un almacén: la ruta de su módulo, cómo se abre y de qué clase es, sin abrirlo. Sus pruebas siguen en `adapters/pkcs11/stores/tests.rs`. |
| `ports.rs` | `Token`, `InstalledFolder`, `CertificateMemory` (que sirve `signing/adapters/memory.rs`) y `Keyring`, el PIN del Almacén de rFirma (ADR-0034); y `SecretPrompter`, el diálogo interactivo del secreto, con su reintento genérico `prompted_until_accepted`. Pruebas en `ports/tests.rs`. |
| `adapters/folder.rs` | `RealInstalledFolder`: la carpeta del Almacén de rFirma y el directorio desechable donde se prueba un `.p12` (ADR-0034). |
| `adapters/keyring.rs` | `RealKeyring`: el adaptador de `Keyring` sobre `oo7`, el portal de secretos o Secret Service (ADR-0034); solo en Linux. Pruebas en `adapters/keyring/tests.rs`. |
| `adapters/windows_credential_manager.rs` | `WindowsCredentialManager`: el `Keyring` de Windows, una credencial genérica del Administrador de credenciales (ADR-0035). Pruebas en `adapters/windows_credential_manager/tests.rs`. |
| `adapters/windows_store.rs` | `WindowsToken`, el adaptador de `Token` en Windows: el almacén del usuario por CNG y los módulos PKCS#11 por `RealToken`, y dónde se buscan esos módulos (ADR-0035). Pruebas en `adapters/windows_store/tests.rs`. |
| `adapters/windows_store/cng.rs` | `CurrentUser\MY` leído con CryptoAPI y el resumen firmado con `NCryptSignHash`; Windows pide el PIN, modal sobre la ventana de rFirma. |

## Trampas

* **`CERTCertificate` no empieza por `derCert`**: sus primeros campos son el
  arena y dos punteros a cadena, así que leer el DER por el principio de la
  estructura revienta con SIGSEGV. El acceso soportado es
  `CERT_GetCertificateDer(cert, &item)`, que `libnss3` exporta desde NSS 3.44.
