# El DNIe en Linux: detección, certificados y PIN por OpenSC

Lo que hace de verdad un DNIe 3.0 o posterior con OpenSC en Linux, medido con una
tarjeta real y con AutoFirma como referencia. Sostiene las decisiones del
[mapa de firma con tarjeta y DNIe](https://github.com/sgomez/rfirma/issues/278) y la
enmienda del [spec del PIN](https://github.com/sgomez/rfirma/issues/899).

Las mediciones las hizo el titular con **su propio DNIe**, en las sondas de
[`card-probes/`](card-probes/). Ninguna imprime datos del certificado: solo etiquetas
de una lista blanca (`Cert*`, `Kpriv*`, `DNI electrónico`), flags, usos de clave,
el emisor y tiempos. Los PIN erróneos se enviaron a propósito y con su permiso.

## Entorno

| Qué | Valor |
| --- | --- |
| Sistema | Ubuntu, `pcscd` 2.4.1 por socket con `--auto-exit`, `libccid` 1.7.1 |
| OpenSC | 0.27.0-rc1, driver `dnie` interno y activo, `opensc.conf` vacío |
| Lector | Alcor Micro AU9540, CCID, sin teclado |
| Tarjeta | DNIe 3.0 o posterior (byte de versión del ATR ≥ `0x04`), antes y después de renovarlo |
| Sondas | Python con `pyscard`, `PyKCS11` y `cryptography`; uso en la cabecera de cada una |

## Por dónde lo ve rFirma

**rFirma ya lista el DNIe**: OpenSC se registra en p11-kit, y rFirma lee los módulos
de p11-kit. Los perfiles NSS no lo ven: rFirma los abre con la softoken, que no lee
sus `pkcs11.txt`. Cada fila sale con el chip de la clase `card`.

## Certificados sin PIN

`C_FindObjects` sin login devuelve tres certificados públicos y ninguna clave privada:

| Etiqueta | Emisor | Uso de clave |
| --- | --- | --- |
| `CertAutenticacion` | `AC DNIE 006` (antes de renovar), `AC DNIE 004` (después) | `digitalSignature` |
| `CertFirmaDigital` | el mismo | `nonRepudiation` |
| `CertCAIntermediaDGP` | `AC RAIZ DNIE 2` | `keyCertSign`, `CA:TRUE` |

Cada certificado lleva un único uso, así que filtrar por uso de clave y emisor
`AC DNIE *` es fiable. El número de la CA cambia al renovar.

**Leer la tarjeta cuesta unos 3 s** al meterla: 2,6–3,2 s dentro de `C_GetSlotList`
o de `C_Initialize`. Con la tarjeta ya leída, `C_Initialize` tarda 1,5 s y cada
operación sin login, menos de 3 ms.

## Detección en caliente

Guiada por `hotplug_probe.py`: desenchufar el lector, enchufarlo con la tarjeta,
sacar la tarjeta y meterla.

| Capa | Lector | Tarjeta |
| --- | --- | --- |
| PC/SC, `SCardGetStatusChange` con `\\?PnP?\Notification` | Al instante, en los dos sentidos | Al instante: `EMPTY` y `PRESENT` |
| PKCS#11, `C_GetSlotList` con un solo `C_Initialize` | La ranura se queda, sin token | Al instante al sacarla; 2,6 s al meterla |
| PKCS#11, `C_WaitForSlotEvent` bloqueante | Evento | Evento |

Ninguna capa necesita reinicializar OpenSC. `C_WaitForSlotEvent` solo dice qué ranura
cambió, y para pararlo hace falta `C_Finalize`. La opción `plug_and_play` ya no existe
en OpenSC 0.27.

## El PIN

### Lo que da OpenSC

- **Ningún aviso por adelantado.** El driver no implementa `SC_PIN_CMD_GET_INFO`, así
  que ni los flags de `C_GetTokenInfo` ni `pkcs15-tool --list-pins` dicen los intentos.
- **`FINAL_TRY` y `LOCKED` solo existen en el proceso que acaba de fallar**, sacados del
  `63Cx` de su `VERIFY`. Un proceso nuevo no los ve, y después de un `CKR_PIN_LOCKED`
  tampoco aparecen. `COUNT_LOW` no se encendió nunca, ni con dos intentos restantes.
- **El intento que bloquea devuelve `CKR_PIN_INCORRECT`**, con `LOCKED` en los flags de
  ese proceso. Los siguientes devuelven `CKR_PIN_LOCKED`, y llegan a la tarjeta.
- **Una tarjeta bloqueada se lista igual que una sana.** Solo se sabe al intentarlo.
- **El PIN cacheado.** OpenSC fuerza su caché del PIN con el DNIe y lo vuelve a
  verificar antes de cada firma. La caché vive hasta el logout; su memoria está
  bloqueada con `mlock`, pero no se pone a cero al liberarla.
- **El `C_Login` con PIN nulo** que hace rFirma al listar se rechaza en local por
  longitud, sin hablar con la tarjeta, en un lector sin teclado.

### AutoFirma, como referencia

jmulticard **lee los intentos restantes sin gastar ninguno**, con su propio APDU por el
canal CWA-14890. Enseñó 3 con la tarjeta intacta y 1 tras dos fallos, y con la tarjeta
bloqueada dice «almacén bloqueado» antes de pedir el PIN. rFirma, por OpenSC, pide el
PIN y responde «La tarjeta está bloqueada» con `CKR_PIN_LOCKED (C_Login)`.

### Las sesiones y otros programas

Con `pin_probe.py --recover`, `--interference` y `--sign-interference`, sin gastar más
que un intento cada vez y recuperándolo con el PIN correcto:

| Caso | `C_Login` o `C_Sign` en la misma sesión |
| --- | --- |
| PIN erróneo y después el correcto, sin nada en medio | Aceptado |
| Sesión sin login, **otro proceso con OpenSC** o **`pkcs15-tool`** en medio, con o sin fallo previo | `CKR_USER_NOT_LOGGED_IN` en 11 ms, **sin llegar a la tarjeta**; en otra sesión, aceptado |
| Sesión sin login, consultas del mismo proceso en medio | Aceptado |
| Sesión autenticada, otro proceso o `pkcs15-tool` en medio, también con la confirmación abierta | `C_Sign` correcto |

Lo que rompe una sesión sin login es que **otro programa abra su canal seguro** con la
tarjeta. Basta con abrir otra sesión: no hace falta `C_CloseAllSessions` ni
reinicializar. Un `C_Login` que llega a la tarjeta tarda 0,8–1,9 s; uno que se queda en
local, 11 ms.

## La firma

El rFirma actual firma con el DNIe de punta a punta. Tras el diálogo del PIN de rFirma,
OpenSC enseña **su propia ventana de confirmación** («Signature Requested», por
`pinentry`) en cada firma con `KprivFirmaDigital`. AutoFirma no la enseña, porque
jmulticard no la implementa. Firmar con `CKM_SHA256_RSA_PKCS` sobre la clave de firma
devuelve 256 bytes.

## Lo que no se ha medido

- **El lote**: cuántas ventanas de confirmación salen al firmar N documentos.
- **Windows y macOS**: las sondas corren en las tres plataformas con OpenSC, pero en
  Windows rFirma firma por CNG con el minidriver del DNIe, que no miden.
- **El DNIe por NFC**: OpenSC no implementa PACE ni CAN para el DNIe.
- **El lector con teclado**: no hay hardware, y con el PIN alfanumérico del DNIe no
  tiene sentido.

## Leer el contador sin gastar intentos

Es posible, como hace jmulticard, y se aparcó para después de la primera versión con
DNIe. El camino conocido es compilar jmulticard con Native Image y darle un transporte
de APDU propio en Rust, porque `javax.smartcardio` necesita una biblioteca nativa del
JDK que el ADR-0004 no admite. Solo valdría para leer el contador: firmar por jmulticard
llevaría el PIN y la operación con la clave a Java, contra el ADR-0001. Y hay que
coordinarlo con OpenSC, porque los dos canales seguros sobre la misma tarjeta se
interfieren.
