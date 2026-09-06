# Mapa de `identity`: quién firma

El contexto de **identidad**: los certificados que hay, cuál eligió la ventana,
cuál se recordó y cómo entra o sale un `.p12`. Su adaptador de verdad es
`adapters/pkcs11/`, la única parte del backend que habla con el token.
`ports.rs` ofrece `NssHost` —la carga compartida de `libnss3.so` y el turno del
token— y lo consume `site/adapters/nss.rs` (RD-08).

Rutas relativas a `src/identity/`. La capa es la carpeta: `domain/` no nombra nada
del crate fuera de sí mismo, `application/` solo `domain/` y `ports.rs`,
`adapters/` lo que quiera; lo que hoy se salta eso está en
`tests/module_directions_debt.txt` y solo mengua. Para situarte en un fichero,
`just outline <ruta>`; las pruebas de cada módulo viven en su hermano
`tests.rs` y se leen solo para tocarlas.

## Dónde vive qué

| Módulo | Líneas | Qué es |
|---|---|---|
| `mod.rs`, `domain/mod.rs`, `application/mod.rs`, `adapters/mod.rs` | | Solo `pub mod`: el reparto del contexto y el de cada capa. |
| `adapters/pkcs11/mod.rs` | 432 | La capa PKCS#11. |
| `adapters/pkcs11/nss.rs` | 336 | Cómo entra un `.p12` en un almacén NSS propio: el descodificador de PKCS#12 de `libsmime3` por FFI, sin criptografía propia y dentro del turno del token (ID-192, ID-193, ID-194). Su adaptador `RealNssHost` del puerto `NssHost` de `ports.rs` para la carga compartida de `libnss3.so`. Pruebas en `adapters/pkcs11/nss/tests.rs` (28). |
| `adapters/pkcs11/secret.rs` | 63 | Cómo se le pide el secreto a cada almacén: sin sesión, por pantalla o en el teclado del lector, que se rechaza (ID-189, ID-191). Pruebas en `adapters/pkcs11/secret/tests.rs` (68). |
| `adapters/pkcs11/stores.rs` | 298 | Dónde se buscan los certificados, incluidos los `.p12` instalados (ID-192). Pruebas en `adapters/pkcs11/stores/tests.rs` (239). |
| `adapters/tauri.rs` | 52 | Las tres órdenes de identidad: listar certificados, instalar y quitar un `.p12`. |
| `adapters/views.rs` | 61 | `CertificateView`, `SecretView` y el nombre en inglés de cada clase de almacén. Pruebas en `adapters/views/tests.rs`. |
| `application/certificates.rs` | 348 | Qué certificados hay, cuál eligió la ventana, cuál se recordó, qué estampa el recuadro, y instalar o quitar un `.p12` (ID-192, ID-197). Pruebas en `application/certificates/tests.rs` (242). |
| `application/listed.rs` | 55 | Los certificados listados en esta sesión: del asa opaca a la referencia. Pruebas en `application/listed/tests.rs` (75). |
| `domain/certificate.rs` | 183 | El certificado tal y como sale del token. Pruebas en `domain/certificate/tests.rs` (140). |
| `domain/error.rs` | 120 | Situaciones del token (ID-29, ADR-0009). Pruebas en `domain/error/tests.rs` (78). |
