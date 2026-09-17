# Banco de referencia CAdES/XAdES/FacturaE

Firmas producidas por los firmadores monofasicos del original 1.9.2
(`AOCAdESSigner`, `AOXAdESSigner`, `AOFacturaESigner`), no por rfirma: es lo
que la firma trifasica de rfirma tiene que igualar en cada ticket de formato.
Certificado de pruebas de la FNMT (`testdata/fnmt/active-rsa.p12`, PIN `1234`);
el certificado personal del titular no se usa en ningún punto del proyecto.

## Las entradas

| Fichero | Qué es |
| --- | --- |
| `challenge.bin` | 64 bytes fijos (`bytes(range(64))`), el reto que firman los CAdES. |
| `document.xml` | XML pequeño y fijo, el documento que firman los XAdES. |
| `invoice.xml` | Factura mínima válida de esquema FacturaE 3.2 (`SchemaVersion` `3.2`, ambas partes personas jurídicas ficticias, sin datos personales). |

## Las firmas

| Fichero | Formato | Sobre |
| --- | --- | --- |
| `cades-implicit.p7s` | CAdES implícito | `challenge.bin` |
| `cades-explicit.p7s` | CAdES explícito (detached) | `challenge.bin` |
| `cades-implicit.cosign.p7s` | Cofirma CAdES | `cades-implicit.p7s` |
| `cades-implicit.countersign-tree.p7s` | Contrafirma CAdES, `target=tree` | `cades-implicit.p7s` |
| `cades-implicit.countersign-leafs.p7s` | Contrafirma CAdES, `target=leafs` | `cades-implicit.p7s` |
| `xades-detached.xml` | XAdES Detached | `document.xml` |
| `xades-enveloping.xml` | XAdES Enveloping | `document.xml` |
| `xades-enveloped.xml` | XAdES Enveloped | `document.xml` |
| `xades-enveloping.cosign.xml` | Cofirma XAdES | `xades-enveloping.xml` |
| `xades-enveloping.countersign-tree.xml` | Contrafirma XAdES, `target=tree` | `xades-enveloping.xml` |
| `xades-enveloping.countersign-leafs.xml` | Contrafirma XAdES, `target=leafs` | `xades-enveloping.xml` |
| `facturae.xsig` | FacturaE | `invoice.xml` |

## Cómo se regenera

```
./rfirma-native-bridge/testbench/make-reference-signatures.sh
```

Resuelve el clasepath desde Maven local (ADR-0002,
`rfirma-native-bridge/testbench/reference-signer/pom.xml`), compila
`ReferenceSigner.java` con `javac` y firma ejecutando la clase resultante.
Determinista salvo la
fecha de firma: `challenge.bin`, `document.xml` e `invoice.xml` son fijos,
pero CAdES y XAdES incrustan el instante de firma en cada regeneración, así
que la huella de las firmas cambia aunque el contenido firmado no lo haga. Al
regenerar, actualiza también las huellas de
`rfirma-app/src-tauri/tests/reference_signatures_kit.rs`.

## Cómo se valida

```
./rfirma-native-bridge/testbench/validate.sh testdata/reference/<fichero>
```

Es el oráculo de la grada C
(`SignValiderFactory` de `afirma-crypto-validation`, consumido igual desde
Maven local): imprime `VALID` o `INVALID <motivo>` y sale con 0 o 1. Las
CAdES también las acepta `openssl cms -verify -noverify` (con `-binary
-content challenge.bin` para las explícitas, al ser detached); las XML,
`xmllint --noout`.
