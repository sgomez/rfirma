# El DNIe en Windows: almacén, PC/SC y KSP del minidriver

Lo que hace de verdad un DNIe con el minidriver de la DGP en Windows, medido con una
tarjeta real. Es la contraparte de [`dnie-en-linux.md`](dnie-en-linux.md) y sostiene
la recarga en caliente del [spec de Windows con tarjetas](https://github.com/sgomez/rfirma/issues/1798)
(ID-482).

Las mediciones las hizo el titular con **su propio DNIe**, con
[`card-probes/minidriver_store_probe.ps1`](card-probes/minidriver_store_probe.ps1). La
sonda no firma ni pide PIN: no abre la clave por el almacén, y todo lo que habla con
el KSP lleva `NCRYPT_SILENT_FLAG`. Solo imprime el papel de cada certificado, el
nombre de su CA, el proveedor de la clave, nombres de lector y de tipo de tarjeta,
códigos de error y tiempos.

## Entorno

| Qué | Valor |
| --- | --- |
| Sistema | Windows 10 22H2 (10.0.19045) |
| Servicios | `SCardSvr` y `CertPropSvc` en marcha, inicio manual; `ScDeviceEnum` parado; directiva `CertProp` sin configurar |
| Lector | Generic EMV Smartcard Reader, USB, sin teclado |
| Tarjeta | DNIe con certificados de `AC DNIE 004` |
| Minidriver | Tipo de tarjeta `DNIeCM`: `DNIeCMx64.dll`, KSP `Microsoft Smart Card Key Storage Provider`, CSP `Microsoft Base Smart Card Crypto Provider` |

## Qué copia Windows a su almacén

| Almacén | Certificado | Clave |
| --- | --- | --- |
| `CurrentUser\My` | Autenticación, de `AC DNIE 004` | `Microsoft Smart Card Key Storage Provider` |
| `CurrentUser\My` | Firma, de `AC DNIE 004` | el mismo |
| `CurrentUser\Root` (vista lógica) | CA `AC DNIE 004`, emitida por `AC RAIZ DNIE 2` | — |

`CERT_KEY_PROV_INFO` nombra el KSP con tipo de proveedor 0 y `keySpec` 0, así que el
proveedor de la clave se lee sin abrirla. La CA intermedia no está en
`CurrentUser\CA`; la sonda no distingue si la puso la propagación u otro programa.

## Meter y sacar la tarjeta

**Windows nunca retira los certificados del DNIe de `My`.** En dos ciclos de sacar y
meter la tarjeta y uno de desconectar y conectar el lector, con 45 s de vigilancia
tras cada evento, ni el almacén reabierto en cada vuelta ni uno abierto desde el
principio cambiaron en nada. Los dos certificados, con su clave en el KSP de
tarjeta, siguen en `My` con el lector vacío y con el lector desconectado.

Por eso la sonda **no pudo medir cuánto tarda la copia** al meter la tarjeta: la copia
ya estaba antes de empezar y no desaparece. Tampoco distingue si un almacén abierto
refleja los cambios sin reabrirlo, porque no hubo ninguno.

## PC/SC por `winscard.dll`

| Evento | Lo que ve `SCardGetStatusChange` |
| --- | --- |
| Meter la tarjeta | `PRESENT` con ATR al instante; el ATR identifica el tipo `DNIeCM` y su minidriver. Durante el segundo siguiente, `INUSE` intermitente mientras el sistema la lee |
| Sacar la tarjeta | `EMPTY` al instante |
| Desconectar el lector, el único | Ningún `UNAVAILABLE` del lector: la llamada devuelve `SCARD_E_SERVICE_STOPPED` y el contexto deja de valer. Un `SCardEstablishContext` 0,4 s después funciona, con cero lectores |
| Conectar el lector con la tarjeta | La notificación PnP (`\\?PnP?\Notification`) trae el lector, y el lector llega ya `PRESENT\|INUSE` |

Al desconectar el lector, el `SCARD_E_SERVICE_STOPPED` llegó 18 s después de la
instrucción, con un `PRESENT|UNPOWERED` a los 1,3 s. La sonda no registra cuándo se
desenchufó el lector, así que no mide el retraso entre desenchufarlo y el error.

Un vigilante en Windows tiene que tratar `SCARD_E_SERVICE_STOPPED` como «sin
lectores» y volver a abrir el contexto, cosa que en Linux no hace falta.

## Listar por el KSP de tarjeta sin PIN

`NCryptEnumKeys` sobre `Microsoft Smart Card Key Storage Provider` lista las dos claves
del DNIe sin pedir nada, y `NCryptGetProperty(SmartCardKeyCertificate)` da el
certificado de cada una:

| Clave | `dwLegacyKeySpec` | Certificado |
| --- | --- | --- |
| RSA | 1 (`AT_KEYEXCHANGE`) | Autenticación; el mismo que el de `My` |
| RSA | 2 (`AT_SIGNATURE`) | Firma; el mismo que el de `My` |

| Situación | Resultado |
| --- | --- |
| Tarjeta ya leída | 2 claves en 200–500 ms |
| Justo al meterla | 2 claves 0,56 s después del `PRESENT` |
| Sin tarjeta | `SCARD_E_NO_SMARTCARD` en 1 ms |

Las llamadas llevaron `NCRYPT_SILENT_FLAG`, y ninguna falló por `NTE_SILENT_CONTEXT`:
listar no necesita interfaz. Sin la bandera no se ha medido.

## Conclusión para la recarga en caliente

**La recarga no puede fiarse del Almacén de Windows; tiene que listar por el KSP**
(la rama alternativa de ID-482). El almacén dice qué tarjeta se ha metido alguna vez,
no cuál está dentro: con el lector vacío conserva los certificados con su clave en el
KSP de tarjeta, y la clase tarjeta no puede salir de él.

El KSP sí lo dice, y deprisa: sin tarjeta falla al instante con
`SCARD_E_NO_SMARTCARD`, y con ella da las claves con sus certificados en menos de un
segundo desde el `PRESENT`. Un certificado de `My` con la clave en el KSP de tarjeta que
el KSP no lista en ese momento es la copia de una tarjeta que no está.

## Lo que no se ha medido

- **Cuánto tarda la propagación** en copiar a `My` los certificados de una tarjeta
  nueva: haría falta borrarlos antes del almacén.
- **Dos tarjetas a la vez**, y si `NCryptEnumKeys` sin ámbito las lista todas.
- **Listar sin `NCRYPT_SILENT_FLAG`.**
- **La firma**: está en el [#1817](https://github.com/sgomez/rfirma/issues/1817).
