# Kit FNMT de pruebas

Material de clave **público por diseño**: lo publica la propia FNMT, con la
contraseña incluida, en
<https://www.sede.fnmt.gob.es/documents/10445900/10649507/Certificados_pruebas_todas_CAs.rar>.
Versionarlo aquí es lo que decidió el [ADR-0014](../../docs/adr/0014-gradas-de-prueba-y-puerta-de-calidad.md):
son casos que **no podríamos fabricar** —revocado de verdad contra un OCSP en
vivo, caducado de verdad— porque exigen una CA real.

**El certificado FNMT personal del titular no se usa en ningún punto del
proyecto.** No se importa, no se exporta y no aparece en ningún fixture.

El kit completo (155 ficheros, incluida la rama ECC) sigue viviendo fuera del
repositorio, en `~/.local/share/rfirma-test-certs`. Aquí solo están los tres
que las pruebas necesitan. Detalle del entorno en
[`docs/research/token-pkcs11-pruebas.md`](../../docs/research/token-pkcs11-pruebas.md).

## Los tres ficheros

| Fichero | Contraseña | Papel | `notAfter` |
| --- | --- | --- | --- |
| `active-rsa.p12` | `1234` | Camino feliz. RSA 2048, OCSP `good`. Es el que está en el token SoftHSM `rfirma-test`. | **2028-10-30 10:06:59 GMT** |
| `revoked-rsa.p12` | `1234` | Revocado de verdad, no caducado: la firma se construye pero la validación debe rechazarla. OCSP `revoked`, motivo `superseded`, desde 2024-10-30. | 2028-10-30 09:58:12 GMT |
| `expired-rsa.p12` | `G5cp,fYC9gje` | Caducado: el rechazo debe ocurrir **antes** de pedir el PIN. | 2020-11-08 12:48:35 GMT |

Los tres son `C=ES, CN=EIDAS CERTIFICADO PRUEBAS - 99999999R`, emitidos por
`C=ES, O=FNMT-RCM, OU=Ceres, CN=AC FNMT Usuarios`.

> `Caducados/password.txt` del kit original nombra un `PF_ACTIVO_EIDAS.p12` que
> no existe, y la contraseña que anuncia para él es en realidad la de
> `PF_CADUCADO_EIDAS.p12` (de donde sale `expired-rsa.p12`).

## Huellas

SHA-256 del fichero `.p12` tal cual está aquí:

```
6e0cad97b78be2918ed54a64a0dd4f3f6e4c16e01b405ef0836fb91b77a3ffb4  active-rsa.p12
a8ff78c1a7b13bcdc12347f683dd5395b6e0ac1d9c3cad23e3668823ae2b1425  revoked-rsa.p12
901df49ac10cceb0524c8cb50833d1407d0974f42f9d45a5b4b71c0eefa4e91f  expired-rsa.p12
```

SHA-256 del certificado de titular (DER), para contrastar contra `openssl`:

```
activo-rsa    27:82:59:D1:09:89:98:C4:45:E1:5F:C0:11:A5:21:1C:3F:41:10:96:FB:57:FE:41:B9:48:95:7C:F9:16:A8:ED
revocado-rsa  26:FA:9C:9C:C4:2B:06:E5:A5:A6:AB:B1:F6:69:6A:E4:16:1C:51:E6:16:DC:94:33:76:CF:EE:FD:10:4A:34:69
caducado-rsa  71:BD:C8:89:E9:F4:68:90:99:9B:47:66:52:59:E1:0B:97:CF:65:4E:03:A9:47:4B:6A:24:AB:03:F4:55:14:2E
```

Los `.p12` de la FNMT usan cifrado antiguo, así que OpenSSL 3 exige `-legacy`:

```bash
openssl pkcs12 -in active-rsa.p12 -passin pass:1234 -clcerts -nokeys -legacy -nodes \
  | openssl x509 -noout -dates -fingerprint -sha256
```

## La bomba de relojería

`active-rsa.p12` **caduca el 2028-10-30**. La guardia va partida a propósito
(ADR-0014):

- la **prueba dura** vive en `rfirma-app/src-tauri/tests/fnmt_kit.rs`, es de
  **grada A** y corre en el **carril rápido**: el día que el certificado caduque
  falla nombrando el fichero, la fecha y el enlace a STCERES;
- el **aviso a 90 días** vive en el **cron semanal** de
  `.github/workflows/ci.yml`, que abre una issue.

Avisar en el carril rápido rompería todos los PRs a la vez, un día cualquiera de
2028. **Sin congelar el reloj**: escondería fallos reales de cadena.

Cuando llegue el día, el kit nuevo se descarga del enlace de arriba —el mismo
que imprime la prueba al fallar—, se sustituyen los ficheros, y las huellas y
las fechas de este README y de `fnmt_kit.rs` se actualizan a la vez. La huella
está justamente para que no se pueda hacer una cosa sin la otra.

## Escáner de secretos

`.github/secret_scanning.yml` excluye este directorio. Son claves privadas de
verdad, pero de una CA de pruebas y con la contraseña publicada por su emisor:
alertar sobre ellas es ruido que enseña a ignorar las alertas de verdad.
