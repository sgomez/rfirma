# Mapa de `identity`: quién firma

El contexto de la **identidad**: qué certificados hay, cuál se usa y cómo entra o
sale un `.p12`. Aquí no se firma ningún documento, solo se dice quién puede.
Su adaptador de verdad es `adapters/pkcs11/`, la única parte del backend que
habla con el token. Rutas relativas a `src/identity/`.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz: `IdentityRoot`, la fachada que usan los vecinos y el `Signer` de `signing/ports.rs` sobre cualquier `Token`. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `application/tests.rs` | Los andamios de la grada A que comparten todos los contextos: `NoToken`, `NoMemory`, `a_certificate`, `a_usable_certificate` y `listed_from`. Solo en pruebas. |
| `adapters/pkcs11/mod.rs` | La capa PKCS#11, y `RealToken`, el único adaptador de producción del puerto `Token`. |
| `adapters/pkcs11/nss.rs` | Cómo entra un `.p12` en un almacén NSS propio, con el PKCS#12 de `libsmime3`. Declara `NssHost` y `RealNssHost`, para `site/adapters/nss.rs`. Pruebas en `adapters/pkcs11/nss/tests.rs`. |
| `adapters/pkcs11/stores.rs` | Dónde se buscan los certificados, incluidos los `.p12` instalados. Pruebas en `adapters/pkcs11/stores/tests.rs`. |
| `adapters/failures.rs` | La única traducción de lo que va mal en identidad a la vista de la ventana y al código de la sede (ADR-0009). Pruebas en `adapters/failures/tests.rs`. |
| `adapters/tauri.rs` | Las tres órdenes de identidad: listar certificados, instalar y quitar un `.p12`. |
| `adapters/views.rs` | Lo que cruza a la ventana: `CertificateView`, `StatusView` y `SecretView`. Pruebas en `adapters/views/tests.rs`. |
| `application/certificates.rs` | Qué certificados hay, cuál se recordó e instalar o quitar un `.p12`; `ListedCertificates` es el último listado, con las asas de `documents/domain/handles.rs`. Pruebas en `application/certificates/tests.rs`. |
| `domain/algorithm.rs` | El algoritmo de firma que se pide por su nombre, la clase de clave que exige y el mecanismo PKCS#11 con el que se cumple. Pruebas en `domain/algorithm/tests.rs`. |
| `domain/certificate.rs` | El certificado tal y como sale del token, y `ListedCertificate`, la fila con su asa. Pruebas en `domain/certificate/tests.rs`. |
| `domain/ecdsa.rs` | Lo que la curva elíptica exige y RSA no: el resumen que firma el mecanismo crudo y el `r`/`s` del token reempaquetado en DER. Pruebas en `domain/ecdsa/tests.rs`. |
| `domain/error.rs` | Las situaciones del token (ADR-0009) y el aviso de que falta `libnss3.so`. Pruebas en `domain/error/tests.rs`. |
| `domain/holder.rs` | Quién es el titular, leído del nombre distinguido (RFC 4514), y `StampedHolder`, lo que estampa el recuadro. Pruebas en `domain/holder/tests.rs`. |
| `domain/secret.rs` | Cómo se le pide el secreto a cada almacén: sin sesión, por pantalla o en el teclado del lector. Pruebas en `domain/secret/tests.rs`. |
| `domain/store.rs` | Un almacén: la ruta de su módulo, cómo se abre y de qué clase es, sin abrirlo. Sus pruebas siguen en `adapters/pkcs11/stores/tests.rs`. |
| `ports.rs` | **Los tres puertos**, que no importa ningún otro contexto: `Token`, `InstalledFolder` y `CertificateMemory`, que sirve `signing/adapters/memory.rs`. |
| `adapters/folder.rs` | `RealInstalledFolder`: la carpeta de cada `.p12` instalado. |
