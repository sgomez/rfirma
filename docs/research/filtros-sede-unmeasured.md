# Cuatro filtros de sede sin medir: `dnie:`, `pseudonym:`, `qualified:` y `ssl:`

Investigación técnica para el [issue #701](https://github.com/sgomez/rfirma/issues/701).

---

## 1. Contexto y objeto de la investigación

En `rfirma-app/src-tauri/src/site/domain/protocol/filters.rs`, la constante
`UNMEASURED_CRITERIA` mantiene anotados cuatro criterios de filtro que una sede
electrónica puede solicitar en el parámetro `filters` de la URL `afirma://`:

```rust
pub const UNMEASURED_CRITERIA: &[&str] = &["dnie:", "pseudonym:", "qualified:", "ssl:"];
```

Estos cuatro criterios forman parte de la lista blanca `ACCEPTED_CRITERIA` (los 17
criterios reconocidos del original), por lo que rFirma no los rechaza con `SAF_03`,
sino que los acepta y los traslada íntegros al motor de filtrado en GraalVM
(`autofirma_filter_certificates`). Sin embargo, en el estado actual del repositorio:

1. En Rust, la única guarda existente (`the_four_unmeasured_criteria_are_still_in_the_measured_catalogue`
   en `site/domain/protocol/filters/tests.rs`) se limita a comprobar que los cuatro
   literales siguen contenidos en el array `ACCEPTED_CRITERIA`.
2. En Java, la prueba `the_four_unmeasured_criteria_are_accepted_even_without_coverage_of_their_verdict`
   en `rfirma-native-bridge/src/test/java/es/gob/afirma/nativebridge/FilterBridgeTest.java`
   se diseñó únicamente para constatar que el motor parsea las cadenas sin abortar
   (`assertTrue(selected.length < listing.size())`), pero no comprueba que el veredicto
   sea el correcto ni discrimina certificados porque se invocó con argumentos
   especulativos (`"qualified:2.16.724.1.3.5.3.2"`, `"ssl:true"`) o sin certificados
   adecuados en la entrada (`TestFixtures.activeCertificate()` y `expiredCertificate()`).

El objetivo de esta investigación es desentrañar con rigor contra las fuentes primarias
(el código fuente de AutoFirma en `afirma-keystores-filters` y los certificados reales
del kit de pruebas) qué evalúa exactamente cada filtro, qué material existe para
probarlos, si pueden salir de `UNMEASURED_CRITERIA` y cuál es el plan exacto de resolución.

---

## 2. Arquitectura del filtrado y punto de evaluación

En rFirma, el filtrado de certificados opera bajo una estricta separación de
responsabilidades (ID-252, ADR-0016):

1. **En Rust (`rfirma-app`):**
   - `SiteFilter` (`site/domain/protocol/filters.rs`) parsea los parámetros `filter`,
     `filters` o `filters.N` de la petición web y los formatea como un bloque
     `java.util.Properties`.
   - `keep_what_the_site_accepts` (`site/application/filtering.rs`) toma los certificados
     detectados en el almacén o token (`TokenCertificate`), extrae su codificación DER,
     los codifica en Base64 separados por `;` y delega en el puerto `FilterEngine`.
   - **Rust no inspecciona campos X.509 para estos filtros**: delega toda la criba
     criptográfica en el motor importado de AutoFirma vía FFI (`autofirma_filter_certificates`).

2. **En Java / GraalVM (`rfirma-native-bridge`):**
   - La función nativa CEntryPoint `autofirma_filter_certificates` invoca `FilterBridge.select(Properties filterProperties, List<X509Certificate> certificates)`.
   - `FilterBridge.select` instancia `CertFilterManager(filterProperties).getFilters()`,
     que devuelve una lista de objetos `es.gob.afirma.keystores.CertificateFilter`.
   - Para cada certificado de la lista de entrada, `FilterBridge` ejecuta:
     ```java
     for (final CertificateFilter filter : engine) {
         if (filter.matches(certificate)) {
             selected.add(Integer.valueOf(i));
             break;
         }
     }
     ```
   - Devuelve un array de índices (`int[]`) con los certificados aceptados.

**Implicación crítica de diseño**: `FilterBridge` evalúa `filter.matches(X509Certificate)`,
un método puramente funcional y sin estado que recibe el certificado X.509 en memoria.
No interactúa con `KeyStoreManager` ni con la interfaz de usuario Swing de AutoFirma.

---

## 3. Análisis técnico exhaustivo de los 4 filtros

Se ha desensamblado e inspeccionado el bytecode de `afirma-keystores-filters-1.9.2.jar`
(fuente primaria en `~/.m2/repository/es/gob/afirma/afirma-keystores-filters/1.9.2/`),
contrastado con la documentación de referencia de `docs/afirma/1.9.2/11-extraparams-y-filtros.md`.

### 3.1. `pseudonym:` (`PseudonymFilter.java`)

#### Implementación en AutoFirma
En `CertFilterManager.java:723-781`:
```java
if (filter.toLowerCase().startsWith("pseudonym:")) {
    String value = "andothers";
    if (!filter.toLowerCase().equals("pseudonym:")) {
        value = filter.substring("pseudonym:".length());
    }
    filters.add(new PseudonymFilter(value));
}
```
En `PseudonymFilter.java`:
```java
public boolean matches(X509Certificate cert) {
    return AOUtil.isPseudonymCert(cert);
}
```
En `es.gob.afirma.core.misc.AOUtil.java:315-319` (`afirma-core-1.9.2.jar`):
```java
public static boolean isPseudonymCert(X509Certificate cert) {
    return getRDNvalueFromLdapName("2.5.4.65", cert.getSubjectX500Principal().getName("RFC2253")) != null;
}
```

#### Campos inspeccionados
- **Campo X.509**: `Subject` (Distinguished Name del titular).
- **Atributo RDN**: OID `2.5.4.65` (`id-at-pseudonym`, estándar X.520 / RFC 4519).
- **Lógica**: Devuelve `true` si y solo si el Subject contiene el atributo `pseudonym` (OID `2.5.4.65`).

#### Particularidad de rFirma frente a Swing
En la GUI Swing de AutoFirma existía una sobrecarga `matches(String[] aliases, KeyStoreManager ksm)`
donde `pseudonym:only` mostraba exclusivamente certificados de seudónimo y `pseudonym:andothers`
ocultaba certificados normales únicamente si existía un seudónimo equivalente (mismo emisor,
mismo KeyUsage, caducidad en ±60s). Pero en `FilterBridge`, el método invocado es
`matches(X509Certificate)`, el cual **ignora el argumento** (`only`, `true`, etc.) y
evalúa siempre si el certificado tiene o no el RDN de seudónimo.

#### Material en el kit de pruebas
- **Disponible en `~/.local/share/rfirma-test-certs`**:
  `Claves RSA/AC Sector Público/Empleado Público con Seudónimo/Antiguo perfil/Activo/SP_Empleado_público_Seudonimo.p12` (contraseña `1234`).
- **Inspección OpenSSL**:
  `Subject: C=ES, O=FNMT-RCM PRUEBAS, OU=CERTIFICADO ELECTRONICO DE EMPLEADO PUBLICO CON SEUDONIMO, OU=OU TEST, pseudonym=TEST-0000, title=PUESTO TEST, CN=PUESTO TEST - TEST-0000 - FNMT-RCM PRUEBAS`
  Contiene `pseudonym=TEST-0000` (`2.5.4.65`).
  En cambio, `active-rsa.p12` tiene `Subject: C=ES, serialNumber=IDCES-99999999R, GN=PRUEBAS, SN=EIDAS CERTIFICADO, CN=EIDAS CERTIFICADO PRUEBAS - 99999999R` (sin seudónimo).
- **Veredicto medido experimentalmente**:
  `FilterBridge.select(filters("filters=pseudonym:true"), List.of(active, pseudonym))` devuelve exactamente `[1]`, discriminando el certificado de seudónimo.

---

### 3.2. `qualified:` (`QualifiedCertificatesFilter.java`)

#### Hallazgo crítico: Discrepancia nominal
A pesar de su nombre, `QualifiedCertificatesFilter` **no evalúa la cualificación jurídica eIDAS del certificado ni examina la extensión `qcStatements`** (OID `1.3.6.1.5.5.7.1.3`, ETSI TS 101 862). (La comprobación de dispositivo cualificado seguro en AutoFirma corresponde al filtro `sscd:`, implementado en `SscdFilter.java`, que busca el OID `0.4.0.1862.1.4` `id-etsi-qcs-QcSSCD`).

En `QualifiedCertificatesFilter.java:22-38`:
```java
public QualifiedCertificatesFilter(String serialNumber) {
    this.serialNumber = prepareSerialNumber(serialNumber);
}

public boolean matches(X509Certificate cert) {
    String sn = getCertificateSN(cert);
    if (sn != null) {
        return prepareSerialNumber(sn).equalsIgnoreCase(this.serialNumber);
    }
    return false;
}
```
Donde:
- `getCertificateSN(cert)` obtiene `cert.getSerialNumber()` y lo convierte a cadena hexadecimal en mayúsculas (`FilterUtils.bigIntegerToHex`).
- `prepareSerialNumber(sn)` limpia espacios en blanco, almohadillas `#` y ceros (`'0'`) a la izquierda.

#### Origen del nombre en AutoFirma
El nombre proviene de los trámites web donde el usuario se identificaba con un certificado
de autenticación, y el servidor web remitía a AutoFirma el número de serie de dicho certificado
bajo la etiqueta `qualified:<serialHex>` para que AutoFirma buscara en el almacén local
el certificado de **firma cualificada** correspondiente al mismo titular (`searchQualifiedSignatureCertificate`).
Sin embargo, cuando `FilterBridge` ejecuta `matches(X509Certificate)`, el filtro se comporta
estrictamente como un **filtro por número de serie hexadecimal**.

En `FilterBridgeTest.java`, el test original pasaba erróneamente `"qualified:2.16.724.1.3.5.3.2"`
asumiendo que esperaba un OID de política, cuando en realidad el motor lo interpretaba como
un número de serie inexistente.

#### Campos inspeccionados
- **Campo X.509**: `serialNumber` (número de serie del certificado).
- **Lógica**: Comparación de igualdad hexadecimal (insensible a mayúsculas y omitiendo ceros iniciales)
  entre el argumento del filtro y el número de serie del certificado.

#### Material en el kit de pruebas
- **Disponible en `testdata/fnmt/`**: Todos los certificados del kit poseen un número de serie.
  - `active-rsa.p12`: `05a4759f2a8f92ea672205444144fcf3` (normalizado: `5a4759f2a8f92ea672205444144fcf3`).
  - `expired-rsa.p12`: `0902999f8486caa55821c9a36bfaa499` (normalizado: `902999f8486caa55821c9a36bfaa499`).
- **Veredicto medido experimentalmente**:
  - `FilterBridge.select(filters("filters=qualified:5a4759f2a8f92ea672205444144fcf3"), List.of(active, expired))` devuelve `[0]`.
  - Con el número de serie del caducado devuelve `[1]`.
  - Con un número de serie inexistente devuelve `[]`.

---

### 3.3. `ssl:` (`SSLFilter.java`)

#### Hallazgo crítico: Discrepancia nominal
Similar a `qualified:`, `SSLFilter` **no evalúa si el certificado es de tipo SSL/TLS (no comprueba ExtendedKeyUsage `serverAuth` ni `clientAuth`)**.

En `SSLFilter.java:26-37`:
```java
public SSLFilter(String serialNumber) {
    this.serialNumber = prepareSerialNumber(serialNumber);
    this.authenticationDnieCertFilter = new AuthenticationDNIeFilter();
    this.signatureDnieCertFilter = new SignatureDNIeFilter();
}

public boolean matches(X509Certificate cert) {
    return prepareSerialNumber(getCertificateSN(cert)).equalsIgnoreCase(this.serialNumber);
}
```

#### Origen del nombre en AutoFirma
En conexiones web con autenticación mutua TLS (mTLS), el servidor web extrae de la sesión
SSL el número de serie del certificado de cliente (variable de entorno web `SSL_CLIENT_M_SERIAL`).
Posteriormente, al solicitar la firma, envía a AutoFirma `ssl:<serialHex>` para obligar a que
el firmante utilice el mismo certificado con el que negoció la sesión SSL (o, si era el
certificado de autenticación del DNIe, AutoFirma en su GUI resolvía la pareja de firma del DNIe
mediante `getAssociatedCertAlias`).

En `FilterBridge` (`matches(X509Certificate)`), la evaluación es exactamente idéntica
a `QualifiedCertificatesFilter`: una **comparación de igualdad sobre el número de serie
hexadecimal**.

En `FilterBridgeTest.java`, el test original pasaba erróneamente `"ssl:true"`. El motor
buscaba un certificado cuyo número de serie hexadecimal fuera `"true"`, resultando en una
lista vacía.

#### Campos inspeccionados
- **Campo X.509**: `serialNumber` (número de serie del certificado).
- **Lógica**: Comparación de igualdad hexadecimal entre el argumento y el número de serie.

#### Material en el kit de pruebas
- **Disponible en `testdata/fnmt/`**: Idéntico a `qualified:`. Los certificados ya versionados
  en `testdata/fnmt/` permiten comprobar la selección exacta por número de serie.
- **Veredicto medido experimentalmente**:
  - `FilterBridge.select(filters("filters=ssl:5a4759f2a8f92ea672205444144fcf3"), List.of(active, expired))` devuelve `[0]`.
  - Con un número de serie no coincidente devuelve `[]`.

---

### 3.4. `dnie:` (`SignatureDNIeFilter.java`)

#### Implementación en AutoFirma
En `CertFilterManager.java:40-62`:
```java
if (filter.startsWith("dnie:")) {
    filters.add(new SignatureDNIeFilter());
}
```
Cualquier argumento posterior a `dnie:` es ignorado (`dnie:true`, `dnie:false` o `dnie:` producen la misma instancia).

En `SignatureDNIeFilter.java:18-32`:
```java
public SignatureDNIeFilter() {
    this.rfc2254Filter = new RFC2254CertificateFilter(null,
        "(&(cn=AC DNIE *)(ou=DNIE)(o=DIRECCION GENERAL DE LA POLICIA)(c=ES))");
    this.keyUsageFilter = new KeyUsageFilter(KeyUsageFilter.SIGN_CERT_USAGE);
}

public boolean matches(X509Certificate cert) {
    return this.keyUsageFilter.matches(cert) && this.rfc2254Filter.matches(cert);
}
```
Donde:
- `KeyUsageFilter.SIGN_CERT_USAGE` exige que el bit 1 (`nonRepudiation` / `contentCommitment`) de la extensión `KeyUsage` sea `true`.
- `RFC2254CertificateFilter` comprueba que el Distinguished Name del **Issuer** cumpla la expresión LDAP:
  - `cn` comience por `AC DNIE ` (comodín `*`)
  - `ou=DNIE`
  - `o=DIRECCION GENERAL DE LA POLICIA`
  - `c=ES`

#### Por qué no hay material en el kit de la FNMT
El kit descargado en `~/.local/share/rfirma-test-certs` procede íntegramente de la Fábrica
Nacional de Moneda y Timbre (`Certificados_pruebas_todas_CAs.rar`).
La FNMT y la Dirección General de la Policía (DGP) son dos autoridades de certificación
totalmente distintas e independientes:
- La FNMT emite bajo `CN=AC FNMT Usuarios`, `CN=AC Sector Público`, `CN=AC Representación`, etc.
- El DNI electrónico es emitido exclusivamente por la **Dirección General de la Policía**
  bajo la jerarquía `AC RAIZ DNIE` y las sub-CAs subordinadas `AC DNIE 001`, `AC DNIE 002`,
  `AC DNIE 003`, etc.
Ningún certificado del kit de la FNMT contiene en su emisor `DIRECCION GENERAL DE LA POLICIA` ni `ou=DNIE`.

#### De dónde procede el material oficial de DNIe
La Dirección General de la Policía publica en el portal oficial del DNI electrónico:
- **Origen**: [dnielectronico.es](https://www.dnielectronico.es) -> Área de Descargas -> **Set de Certificados de Prueba**.
- **Contenido del set**: Paquete de certificados de pruebas emitidos por la autoridad de pruebas del DNIe, incluyendo pares de autenticación y firma.
- **Alternativa sintética para pruebas unitarias de Grada A**:
  Dado que `FilterBridge.select` opera sobre objetos `X509Certificate` sin validar la firma
  criptográfica contra la raíz ni consultar OCSP en este punto, un certificado X.509
  generado con `Issuer: C=ES, O=DIRECCION GENERAL DE LA POLICIA, OU=DNIE, CN=AC DNIE 001` y
  `KeyUsage: nonRepudiation` es seleccionado deterministamente por `SignatureDNIeFilter`
  (comprobado experimentalmente: devuelve `[1]` frente al certificado activo de la FNMT).

---

## 4. Cuadro comparativo de resolución

| Criterio | Argumento real esperado | Qué campo X.509 inspecciona | ¿Material en el repo actual? | ¿Se puede resolver? | Cómo resolverlo |
|---|---|---|---|---|---|
| `pseudonym:` | Cualquiera (lo ignora) | `Subject` OID `2.5.4.65` (`pseudonym`) | No en `testdata/fnmt/`, sí en `~/.local/share/rfirma-test-certs` | **SÍ** | Incorporar `SP_Empleado_público_Seudonimo.p12` (o su `.cer`) a las pruebas y verificar que selecciona el índice del seudónimo y excluye el normal. |
| `qualified:` | Número de serie hexadecimal (`<sn>`) | `serialNumber` (hexadecimal, insensible a mayúsculas/ceros) | **SÍ** (en `testdata/fnmt/`) | **SÍ** | Probar con el número de serie de `active-rsa.p12` (`5a4759f2a8f92ea672205444144fcf3`), verificando que selecciona el activo y excluye los demás. |
| `ssl:` | Número de serie hexadecimal (`<sn>`) | `serialNumber` (hexadecimal, insensible a mayúsculas/ceros) | **SÍ** (en `testdata/fnmt/`) | **SÍ** | Probar con el número de serie de `active-rsa.p12`, verificando que selecciona el activo y excluye los demás. |
| `dnie:` | Cualquiera (lo ignora) | `Issuer` LDAP `(&(cn=AC DNIE *)(ou=DNIE)(o=DIRECCION GENERAL DE LA POLICIA)(c=ES))` + `KeyUsage` `nonRepudiation` | **NO** (es de la DGP, no de la FNMT) | **Condicional** | No hay material en el kit FNMT. Procede del "Set de Certificados de Prueba" de `dnielectronico.es`, o mediante un espécimen X.509 de prueba con dicho Issuer y KeyUsage. |

---

## 5. Plan de acción concreto para el issue #701

Para cerrar el issue #701 de forma limpia y rigurosa con la política del repositorio:

1. **Añadir el certificado de seudónimo al material de pruebas:**
   - Copiar `SP_Empleado_público_Seudonimo.p12` desde el kit FNMT a `testdata/fnmt/pseudonym-rsa.p12` (o exponer su certificado en `TestFixtures`).
   - Actualizar `testdata/fnmt/README.md` documentando el nuevo fichero, su contraseña (`1234`), su `notAfter` (2027-02-05) y sus huellas SHA-256.

2. **Actualizar `FilterBridgeTest.java` (Grada A en `rfirma-native-bridge`):**
   - Reemplazar la prueba `the_four_unmeasured_criteria_are_accepted_even_without_coverage_of_their_verdict()`
     por pruebas específicas con cobertura real del veredicto:
     * `pseudonym_criterion_matches_only_pseudonym_certificates`: evalúa `pseudonym:true` sobre una lista con `[active, pseudonym]`, comprobando que solo pasa el índice `1`.
     * `qualified_criterion_matches_certificate_by_hex_serial_number`: evalúa `qualified:<active_sn>` y `qualified:<other_sn>`, comprobando que discrimina por número de serie.
     * `ssl_criterion_matches_certificate_by_hex_serial_number`: evalúa `ssl:<active_sn>`, comprobando que discrimina por número de serie.
     * Para `dnie:`, documentar en el test que no hay certificado en el kit FNMT por pertenecer a la PKI de la Dirección General de la Policía (o evaluarlo con un espécimen X.509 de prueba con el emisor del DNIe si se autoriza dicho espécimen).

3. **Actualizar `filters.rs` y las pruebas de Rust:**
   - En `rfirma-app/src-tauri/src/site/domain/protocol/filters.rs`:
     Retirar `"pseudonym:"`, `"qualified:"` y `"ssl:"` de `UNMEASURED_CRITERIA`, dejando únicamente:
     ```rust
     pub const UNMEASURED_CRITERIA: &[&str] = &["dnie:"];
     ```
   - Si se decide incluir un fixture para `dnie:`, `UNMEASURED_CRITERIA` pasaría a quedar vacío `&[]`.

---

## 6. Referencias primarias

- `afirma-keystores-filters-1.9.2`:
  - `es.gob.afirma.keystores.filters.CertFilterManager`
  - `es.gob.afirma.keystores.filters.PseudonymFilter`
  - `es.gob.afirma.keystores.filters.QualifiedCertificatesFilter`
  - `es.gob.afirma.keystores.filters.SSLFilter`
  - `es.gob.afirma.keystores.filters.SignatureDNIeFilter`
  - `es.gob.afirma.keystores.filters.rfc.KeyUsageFilter`
- `afirma-core-1.9.2`:
  - `es.gob.afirma.core.misc.AOUtil.isPseudonymCert` (OID `2.5.4.65`)
- Documentación interna del repositorio:
  - `docs/afirma/1.9.2/11-extraparams-y-filtros.md` (§7: catálogo exhaustivo de filtros).
  - `docs/research/token-pkcs11-pruebas.md` (detalle de los tokens y el kit de la FNMT).
  - `testdata/fnmt/README.md` (certificados FNMT versionados).
  - `testdata/softhsm/certs.sh` (instalación del token de desarrollo `rfirma-kit`).
- Normativas y estándares:
  - ITU-T X.520 / RFC 4519: `id-at-pseudonym` (OID `2.5.4.65`).
  - RFC 2254 / RFC 4515: Representación en cadena de filtros de búsqueda LDAP.
  - Portal oficial DNIe: `https://www.dnielectronico.es` (Área de Descargas -> Set Certificados de Prueba).
