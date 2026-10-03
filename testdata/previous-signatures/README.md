# Muestras de firmas previas

PDF y CAdES ya firmados que la cofirma lee para saber quién firmó antes, cuándo y en
qué estado está cada firma. Se fabrican una vez y se versionan porque no se
pueden fabricar en el momento de la prueba. Todos los certificados son del
kit de pruebas de la FNMT (`testdata/fnmt/`); el certificado personal del
titular no se usa en ningún punto del proyecto.

| Fichero | Qué es |
| --- | --- |
| `pades-long-term-expired.pdf` | Una firma PAdES de perfil longevo (`PAdES-T`: sello de tiempo de firma) hecha con `expired-rsa.p12`, caducado desde 2020. Con `checkCert=true` el validador del original le da dos veredictos: `CERTIFICATE_EXPIRED` (KO) y `SIGN_PROFILE_NOT_CHECKED` (UNKNOWN). |
| `pades-long-term-active.pdf` | La misma firma `PAdES-T`, hecha con `active-rsa.p12`, en vigor hasta 2028. El validador del original le da solo `SIGN_PROFILE_NOT_CHECKED` (UNKNOWN), que rFirma no cuenta (ADR-0043). La prueba de grada C le añade un `/DSS` en una revisión posterior para que el original la lea como `PAdES-LT`. |
| `cades-expired.csig` | Una firma CAdES implícita de `rfirma: contenido firmado en CAdES` hecha con `expired-rsa.p12`. Íntegra, sale caducada; la prueba de grada C le cambia un byte del contenido para que salga no válida, modificada después de firmarse (ADR-0043). |
| `cades-countersigned-by-expired.csig` | El mismo contenido firmado en CAdES implícito con `active-rsa.p12` y contrafirmado (`tree`) con `expired-rsa.p12`: la firma sale válida y su contrafirma, caducada. |

## Cómo se regenera

```
./rfirma-native-bridge/testbench/make-previous-signature-samples.sh
```

Firma con `AOPDFSigner` y `AOCAdESSigner` del original 1.9.2, consumido desde Maven local
(ADR-0002) a través de `rfirma-native-bridge/testbench/reference-signer/`, y
sella con una TSA de OpenSSL que levanta en el bucle local
(`testbench/openssl-tsa.py`, la misma receta que las pruebas de sello de la
grada C, ADR-0030). Necesita `mvn`, `javac`, `python3` y `openssl`.

El sello no lo pone `AOPDFSigner`: en el 1.9.2 su `PdfTimestamper` no llega a
sellar y devuelve la firma sin sello con solo un `WARNING`. La receta sella el
CMS con `CMSTimestamper` del original y lo vuelve a escribir en su hueco de
`/Contents`, que queda fuera del `ByteRange`, así que la firma sigue
correspondiendo con los datos.

No es determinista: el PDF de entrada, el instante de firma y el sello cambian
en cada regeneración. El sello es posterior a la caducidad del certificado,
porque la TSA sella con la hora del momento.

## Cómo se comprueba

`PreviousSignatureSamplesTest`, en las pruebas de JUnit del puente
(`just test-java`), comprueba que la firma lleva sello de tiempo, que su perfil
es longevo, que sigue correspondiendo con los datos y que su certificado es el
caducado del kit.
