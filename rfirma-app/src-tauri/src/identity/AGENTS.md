# Mapa de `identity`: quién firma

El contexto de **identidad**: los certificados que hay, cuál eligió la ventana,
cuál se recordó y cómo entra o sale un `.p12`. Su adaptador de verdad es
`adapters/pkcs11/`, la única parte del backend que habla con el token, y los
casos de uso la ven solo por el puerto `Token` de `ports.rs` (#453). El mismo
fichero ofrece `NssHost` —la carga compartida de `libnss3.so` y el turno del
token—, que consume `site/adapters/nss.rs` (RD-08), y `CertificateMemory`, el
certificado recordado, que sirve `signing/adapters/memory.rs`. El ciclo de
firma no ve el token: ve el `Signer` de `signing/ports.rs`, que la raíz le
presta con `signer()`.

Rutas relativas a `src/identity/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera, y los casos de uso de otro contexto solo por su raíz
(`<contexto>/mod.rs`); lo vigila `tests/module_directions.rs`. Para situarte
en un fichero, `just outline <ruta>`; las pruebas de cada módulo viven en su
hermano `tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Qué es |
|---|---|
| `mod.rs` | La raíz: `IdentityRoot`, con el token, los almacenes, el listado vivo y la memoria; la fachada que usan los vecinos —`certificates`, `rows_of`, `usable`, `remember_the_certificate`, `signer`— y el `Signer` sobre cualquier `Token`. |
| `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | Solo `pub mod`: el reparto de cada capa. |
| `application/tests.rs` | Los andamios de la grada A que comparten todos los contextos: `NoToken`, `NoMemory`, los certificados de prueba (`a_certificate`, `a_usable_certificate`) y `listed_from`. Solo en pruebas. |
| `adapters/pkcs11/mod.rs` | La capa PKCS#11, y `RealToken`, el único adaptador de producción del puerto `Token`. |
| `adapters/pkcs11/nss.rs` | Cómo entra un `.p12` en un almacén NSS propio: el descodificador de PKCS#12 de `libsmime3` por FFI, sin criptografía propia y dentro del turno del token (ID-192, ID-193, ID-194). Declara aquí el rasgo `NssHost` —la carga compartida de `libnss3.so` y el turno del token— y su `RealNssHost`, porque su único comprador es otro adaptador, `site/adapters/nss.rs`. Pruebas en `adapters/pkcs11/nss/tests.rs`. |
| `adapters/pkcs11/stores.rs` | Dónde se buscan los certificados, incluidos los `.p12` instalados (ID-192). Pruebas en `adapters/pkcs11/stores/tests.rs`. |
| `adapters/failures.rs` | La única traducción de las situaciones del token, del secreto en el lector y del `.p12` a la vista de la ventana y al código de la sede (ADR-0009). Pruebas en `adapters/failures/tests.rs`. |
| `adapters/tauri.rs` | Las tres órdenes de identidad: listar certificados, instalar y quitar un `.p12`. |
| `adapters/views.rs` | `CertificateView` —desde `ListedCertificate`—, `StatusView` —desde `CertificateStatus`—, `SecretView` y el nombre en inglés de cada clase de almacén. Pruebas en `adapters/views/tests.rs`. |
| `application/certificates.rs` | Qué certificados hay, cuál eligió la ventana, cuál se recordó —por `CertificateMemory`— e instalar o quitar un `.p12` (ID-192, ID-197). `ListedCertificates` es `Handles<CertificateRef>` (de `documents/domain/handles.rs`): el último listado tras sus asas. Devuelve `TokenError` o `InstallError`, nunca una vista. Pruebas en `application/certificates/tests.rs`. |
| `domain/certificate.rs` | El certificado tal y como sale del token, y `ListedCertificate`: la fila con su asa. Pruebas en `domain/certificate/tests.rs`. |
| `domain/error.rs` | Situaciones del token (ID-29, ADR-0009) y el aviso de que `libnss3.so` no está. Pruebas en `domain/error/tests.rs`. |
| `domain/holder.rs` | Quién es el titular, leído del nombre distinguido (RFC 4514): el `CN`, el número, el emisor, el seudónimo y `StampedHolder`, lo que el recuadro estampa. Pruebas en `domain/holder/tests.rs`. |
| `domain/secret.rs` | Cómo se le pide el secreto a cada almacén: sin sesión, por pantalla o en el teclado del lector, que se rechaza (ID-189, ID-191). Pruebas en `domain/secret/tests.rs`. |
| `domain/store.rs` | Un almacén: la ruta de su módulo, cómo se abre y de qué clase es, sin abrirlo. Sus pruebas siguen en `adapters/pkcs11/stores/tests.rs`. |
| `ports.rs` | **Los tres puertos**: `Token` —listar, cómo pide el secreto, firmar e importar un `.p12`, con el listado de varios almacenes ya resuelto sobre el de uno—, `InstalledFolder` —la carpeta de cada `.p12` instalado, con sus permisos— y `CertificateMemory`. Ningún otro contexto los importa. |
| `adapters/folder.rs` | `RealInstalledFolder`: crear, restringir al dueño y borrar la carpeta de un `.p12` instalado. |
