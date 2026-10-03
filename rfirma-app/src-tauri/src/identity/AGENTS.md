# Mapa de `identity`: quién firma

El contexto de la **identidad**: qué certificados hay, cuál se usa y cómo entra o
sale un `.p12`. Aquí no se firma ningún documento, solo se dice quién puede.
Su adaptador de verdad es `adapters/pkcs11/`, la única parte del backend que
habla con el token; en Windows, también `adapters/windows_store/`. Rutas relativas a `src/identity/`.

## Dónde vive qué

Cada módulo dice qué es en su cabecera `//!`, y `just outline
rfirma-app/src-tauri/src/identity/` las junta en un índice; acótalo a un
subdirectorio (`identity/adapters/pkcs11/`) si ya sabes la capa.

## Trampas

* **`CERTCertificate` no empieza por `derCert`**: sus primeros campos son el
  arena y dos punteros a cadena, así que leer el DER por el principio de la
  estructura revienta con SIGSEGV. El acceso soportado es
  `CERT_GetCertificateDer(cert, &item)`, que `libnss3` exporta desde NSS 3.44.
