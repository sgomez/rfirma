# La clave privada nunca cruza a Java: firma trifásica con la firma en Rust

Reimplementamos AutoFirma reutilizando su motor criptográfico Java, pero el caso
de uso principal es el **DNIe**, cuya clave es no exportable: solo puede usarse
delegando la operación en la propia tarjeta a través de su módulo PKCS#11 del
sistema. Por eso partimos la firma en tres etapas y repartimos las
responsabilidades: **Java hace la prefirma y la postfirma** (calcular lo que hay
que firmar y ensamblar el documento final en CAdES/PAdES/XAdES/FacturaE), y la
**firma la ejecuta Rust** contra PKCS#11 / CNG / Keychain según el sistema
operativo.

## Consequences

- Ninguna clave privada, PIN ni handle de sesión de la tarjeta debe pasar al
  isolate de Java bajo ninguna circunstancia. Cualquier API del puente nativo
  que lo permitiera es un fallo de diseño, no una optimización.
- La frontera FFI transporta datos a firmar y firmas ya hechas, nunca material
  de clave.
- Cuando la sede pide una firma trifásica contra su propio servidor
  (`serverUrl`) con `CAdEStri`, `PAdEStri`, `XAdEStri` o `FacturaEtri`, la
  prefirma y la postfirma las hace ese servidor y no el puente, pero cada
  `PRE` lo sigue firmando Rust con el token en PKCS#1: la clave tampoco sale
  hacia la sede. El formulario de cada formato es el de su firmador trifásico
  de AutoFirma 1.9.2 (`format`, `cop` y los `extraParams` que retira), y la
  firma que se entrega es la que el servidor devuelve tras `OK NEWID=`.
- Renunciamos a las rutas de firma monofásica que la suite Java ofrece: aunque
  funcionarían para certificados en software, tener dos caminos distintos según
  el origen del certificado duplicaría la superficie a probar y haría fácil que
  el camino equivocado acabase usándose con una tarjeta.
