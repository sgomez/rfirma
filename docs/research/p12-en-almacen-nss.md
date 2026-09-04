# Cómo entra un `.p12` en un almacén NSS sin criptografía propia

Sondeo del [#244](https://github.com/sgomez/rfirma/issues/244), hijo del mapa
[#217](https://github.com/sgomez/rfirma/issues/217). Resuelve la incógnita que
dejó abierta la resolución de la **ficha 17a**
([#230](https://github.com/sgomez/rfirma/issues/230)): un `.p12` instalado se
convierte en un almacén NSS propio por fichero, pero faltaba decidir **cómo
entra el fichero en ese almacén** sin que rfirma incorpore criptografía propia.

**Respuesta corta: gana el camino (1), la API de PKCS#12 de NSS por FFI, pero
no por `libnss3.so` ni con `NSS_Init`.** Las dos cosas que el enunciado daba
por supuestas son falsas y las dos se han medido:

1. Los símbolos de PKCS#12 **no están en `libnss3.so`**: están en
   `libsmime3.so`, que sale del mismo paquete y también está en el runtime.
2. **`NSS_Init` y el `C_Initialize` de `cryptoki` sobre `libsoftokn3.so` no
   pueden convivir**: con softoken ya inicializado, cualquier `NSS_*_Init`
   falla con `SEC_ERROR_UNKNOWN_PKCS11_ERROR`. El riesgo que el ticket señalaba
   como «lo que puede tumbar el camino (1)» es real.

Lo que salva el camino (1) es que **no hace falta `NSS_Init` sobre un
`configdir`**. La combinación `NSS_NoDB_Init` + `SECMOD_OpenUserDB` abre el
almacén del `.p12` como una ranura suelta, sin base de datos por omisión, y el
descodificador de PKCS#12 importa ahí dentro. Medido de punta a punta, dentro
del bundle instalado, con un `.p12` real del kit de pruebas de la FNMT: crea el
almacén, le pone **la contraseña del propio fichero** y mete el certificado y
su clave privada. La restricción de forma del #230 se cumple.

El coste de empaquetado es **cero** en los tres canales. El camino (2)
(`pk12util`) costaría un binario más en el flatpak y una dependencia nueva en
el `.deb` y el `.rpm`; el camino (3) traería el primer crate de criptografía a
un árbol que se ha negado a tenerlo a propósito. Ninguno de los dos compra
nada que el (1) no dé ya.

---

## Qué se midió y cómo

### El método

Un programa desechable de un solo fichero en C (`probe.c`, ~230 líneas) que
hace **exactamente las mismas llamadas que `pk12util`** —comprobado contra
`cmd/pk12util/pk12util.c` del árbol de NSS, ver *Fuentes*— y, en el otro
extremo, **exactamente las mismas que `rfirma-app`**: `C_Initialize` sobre
`libsoftokn3.so` con la cadena de init args que construye `Store::nss`.

Se eligió C y no Rust a propósito: la pregunta es sobre el ciclo de vida de dos
bibliotecas de C en el mismo proceso, y meter `cryptoki` y un binding de NSS por
medio añadiría capas que podrían explicar un fallo que no es suyo.

El programa tiene un modo por experimento, y el orden de las llamadas es el
experimento:

| Modo | Qué hace |
|---|---|
| `two-softokn` | dos `C_Initialize` seguidos, dos `configdir` distintos |
| `two-softokn-fin` | igual, pero con `C_Finalize` en medio (el bucle real de `list_certificates_across`) |
| `softokn-then-nss` | `C_Initialize` y, con él vivo, `NSS_InitReadWrite` |
| `nss-then-softokn` | al revés |
| `import` | `NSS_InitReadWrite` + descodificador de PKCS#12 |
| `userdb` | `NSS_NoDB_Init` + `SECMOD_OpenUserDB` + descodificador |
| `userdb-after-softokn` | lo anterior con softoken ya inicializado por `cryptoki` |

### El entorno

- **Anfitrión**: Ubuntu 26.04.1 LTS, `libnss3` y `libnss3-tools` 2:3.120-1ubuntu2.1.
- **Sandbox**: `me.sgomez.rfirma` 0.1.0 instalado, sobre
  `org.gnome.Platform/x86_64/50`. `NSS_GetVersion()` dentro del bundle
  devuelve **`3.101.4`**.
- **El fichero**: `SP_Empleado_publico_activo.p12` del kit de pruebas de la
  FNMT (`docs/research/token-pkcs11-pruebas.md`), contraseña `1234`, clave RSA.
  Se probó también una copia reexportada con `openssl pkcs12 -legacy`, para
  descartar que el cifrado antiguo del fichero cambiase algo. No cambia nada.
- **El certificado del titular no interviene en ninguna medición.** Todos los
  nombres de esta página son especímenes de la FNMT.

Las rutas de las salidas pegadas aquí van recortadas a `<DIR>` y `<HOME>`.

---

## 1. Qué hay en el runtime, medido dentro del bundle

`flatpak run --command=python3 me.sgomez.rfirma -` con un script por la entrada
estándar, que es la forma que describe `CLAUDE.md` (dentro no hay `strings` ni
`busctl`):

```
libnss3.so*      -> ['/usr/lib/x86_64-linux-gnu/libnss3.so']
libsoftokn3.so*  -> ['/usr/lib/x86_64-linux-gnu/libsoftokn3.so']
libsmime3.so*    -> ['/usr/lib/x86_64-linux-gnu/libsmime3.so']
libnssutil3.so*  -> ['/usr/lib/x86_64-linux-gnu/libnssutil3.so']
libfreebl3.so*   -> ['/usr/lib/x86_64-linux-gnu/libfreebl3.so']
libnssckbi.so*   -> ['/usr/lib/x86_64-linux-gnu/libnssckbi.so']

PATH: /app/bin:/usr/bin
pk12util  -> NO
certutil  -> NO
modutil   -> NO
openssl   -> ['/usr/bin/openssl']
```

| | En `org.gnome.Platform//50` |
|---|---|
| `libnss3.so` | **sí** |
| `libsmime3.so` | **sí** |
| `libsoftokn3.so` | **sí** (lo que el ID-15 ya daba por hecho) |
| `pk12util` | **no** |
| `certutil`, `modutil` | **no** |

Y en el SDK, que es lo que decide si el camino (1) cuesta algo al construir:

```
$ flatpak run --command=sh --devel me.sgomez.rfirma -c 'pkg-config --modversion nss nspr; pkg-config --cflags nss'
3.101.4
4.37.0
-I/usr/include/nss
```

`org.gnome.Sdk//50` trae cabeceras y `pkg-config` de NSS. **El camino (1) no
añade ni un módulo al manifiesto del flatpak.**

## 2. Dónde viven de verdad los símbolos de PKCS#12

El enunciado del ticket, y la resolución del #230, hablan de «la API de PKCS#12
de `libnss3.so`». **No está ahí.** Cargando cada biblioteca de NSS del runtime
por `ctypes.CDLL` y preguntando por cada símbolo:

```
/usr/lib/x86_64-linux-gnu/libnss3.so     -> ninguno
/usr/lib/x86_64-linux-gnu/libnssutil3.so -> ninguno
/usr/lib/x86_64-linux-gnu/libsoftokn3.so -> ninguno
/usr/lib/x86_64-linux-gnu/libssl3.so     -> ninguno
/usr/lib/x86_64-linux-gnu/libsmime3.so   -> ['SEC_PKCS12DecoderStart',
   'SEC_PKCS12DecoderUpdate', 'SEC_PKCS12DecoderVerify',
   'SEC_PKCS12DecoderValidateBags', 'SEC_PKCS12DecoderImportBags',
   'SEC_PKCS12DecoderFinish', 'SEC_PKCS12DecoderGetCerts',
   'SEC_PKCS12DecoderRenameCertNicknames',
   'SEC_PKCS12DecoderSetTargetTokenCAs', 'SEC_PKCS12CreateExportContext',
   'SEC_PKCS12AddCertAndKey', 'SEC_PKCS12Encode',
   'SEC_PKCS12AddPasswordIntegrity']
```

El reparto es limpio y hay que respetarlo al enlazar:

| Cabecera | Símbolos | Biblioteca |
|---|---|---|
| `p12.h` | `SEC_PKCS12Decoder*` | **`libsmime3.so`** |
| `nss.h` | `NSS_NoDB_Init`, `NSS_Shutdown`, `NSS_IsInitialized` | `libnss3.so` |
| `pk11pub.h` | `PK11_InitPin`, `PK11_NeedUserInit`, `PK11_Authenticate`, `PK11_SetPasswordFunc`, `PK11_FreeSlot` | `libnss3.so` |
| `secmod.h` | `SECMOD_OpenUserDB`, `SECMOD_CloseUserDB` | `libnss3.so` |

Los diez de `libnss3.so` se comprobaron uno a uno dentro del bundle y están
todos. `pkg-config --libs nss` ya devuelve `-lnss3 -lnssutil3 -lsmime3 -lssl3
-lplds4 -lplc4 -lnspr4`, así que la corrección no cambia nada del `build.rs`
salvo no escribir `-lnss3` a mano y quedarse corto.

## 3. El riesgo central: `NSS_Init` frente al `C_Initialize` de `cryptoki`

Esta es la pregunta principal del ticket y la respuesta es dura.

Antes de las cifras, **una trampa que invalidó la primera tanda de medidas** y
que conviene dejar escrita, porque cualquiera que rehaga esto la pisa:

> `CK_C_INITIALIZE_ARGS` **no es la misma estructura** en la cabecera de NSS que
> en la estándar. NSS (`/usr/include/nss/pkcs11t.h:1736-1750`) añade un campo
> propio, `LibraryParameters`, **antes** de `pReserved`, y ahí es donde softoken
> busca la cadena de init args. El sexto campo de la estructura estándar
> —`pReserved`, que es donde lo pone `cryptoki`, y bien— es el
> `LibraryParameters` de NSS. Un programa escrito contra la cabecera de NSS que
> rellene `pReserved` **no falla: el `configdir` se ignora en silencio** y
> softoken abre un almacén vacío que igualmente devuelve `CKR_OK` y dos ranuras
> con nombre plausible. El comentario de `pkcs11/mod.rs:308` ya avisa de que un
> `configdir` que no lleva a ningún sitio «no siempre falla»; esto es otra
> puerta al mismo pozo.

Con el campo correcto, los cuatro experimentos:

| # | Secuencia | Resultado |
|---|---|---|
| 1 | `C_Initialize(A)` → `C_Initialize(B)` | segundo `rv=0x191` (**`CKR_CRYPTOKI_ALREADY_INITIALIZED`**). Las ranuras **no cambian**: siguen siendo las de A. El `configdir` de B se descarta. |
| 2 | `C_Initialize(A)` → `C_Finalize` → `C_Initialize(B)` | los dos `rv=0x0`, y las ranuras **sí** cambian: `flags=0x60d` (A, con contraseña) pasa a `0x609` (B, sin contraseña). |
| 3 | `C_Initialize(A)` vivo → `NSS_InitReadWrite(B)` | **`SECFailure`, `PR_GetError() = -8018`**, que es `SEC_ERROR_UNKNOWN_PKCS11_ERROR` (`secerr.h:203`). `NSS_IsInitialized()` devuelve 0. Idéntico con `NSS_NoDB_Init(NULL)`. |
| 4 | `NSS_InitReadWrite(A)` → `C_Initialize(B)` | `rv=0x191`. Las ranuras que ve el `cryptoki` son **las de A**, el almacén que abrió NSS, no las de B. |

Salida literal del experimento 3, que es el que decide:

```
C_Initialize(configdir=<DIR>/storeA) rv=0x0
[softoken A]   slot 2 label='NSS Certificate DB' flags=0x60d
NSS_InitReadWrite(sql:<DIR>/storeB) -> SECFailure (PR_GetError=-8018)
NSS_IsInitialized=0
```

Tres consecuencias, y las tres importan:

- **El conflicto es temporal, no estructural.** Ninguna de las dos bibliotecas
  estorba a la otra si no se solapan: basta que softoken no esté inicializado
  cuando NSS arranca, y que NSS haya hecho `NSS_Shutdown` cuando softoken
  vuelve. Los experimentos 2 y 4 lo confirman en los dos sentidos.
- **El experimento 4 es un modo de fallo silencioso que rfirma ya tolera.**
  `initialized()` (`pkcs11/mod.rs:558-561`) traga
  `CKR_CRYPTOKI_ALREADY_INITIALIZED` a propósito y con buen motivo, pero el
  precio es que si algo del proceso hiciera `NSS_Init` y no lo cerrara, rfirma
  listaría **el almacén de NSS creyendo que lista el que pidió**, sin ningún
  error. No es hipotético: es exactamente lo que pasaría si la importación del
  `.p12` se escribe sin cuidado.
- **rfirma ya tiene la pieza que hace falta para el cuidado.** El comentario de
  `context()` (`pkcs11/mod.rs:481-501`) dice que un almacén con init args
  **no se cachea** —se abre, se lee y se cierra, y el `Drop` de `Pkcs11` llama a
  `C_Finalize`— y que por eso todo lo que abre un almacén pasa por
  `with_token_turn` (`pkcs11/mod.rs:371`). Fuera de ese turno, y con la única
  excepción que se anota abajo, **`libsoftokn3.so` no está inicializado en el
  proceso**. Ese es el hueco donde la importación cabe sin tocar nada.

  *La excepción*: `RFIRMA_PKCS11_MODULE` apuntando a `libsoftokn3.so` **sí**
  entraría en el mapa `MODULES` (no lleva init args) y dejaría softoken
  inicializado para siempre. Es la escotilla de las pruebas de grada B y no la
  vía de nadie más, pero la importación debe dar un error legible en ese caso en
  vez de un `SEC_ERROR_UNKNOWN_PKCS11_ERROR` a la cara.

## 4. El camino que funciona: `NSS_NoDB_Init` + `SECMOD_OpenUserDB`

`NSS_Init` sobre el `configdir` del `.p12` funciona (se midió, modo `import`)
pero es la variante mala: convierte ese almacén en la base de datos **por
omisión** del proceso y arrastra todo el estado global de NSS. La variante
buena es la que usa Chromium para lo mismo: inicializar NSS **sin** base de
datos y abrir el almacén como una ranura suelta.

```c
NSS_NoDB_Init(NULL);
PK11SlotInfo *slot = SECMOD_OpenUserDB(
    "configDir='sql:<dir>' certPrefix='' keyPrefix='' "
    "tokenDescription='rfirma p12' flags=readWrite");
if (PK11_NeedUserInit(slot))
    PK11_InitPin(slot, NULL, password);      /* la contraseña del .p12 */
PK11_Authenticate(slot, PR_TRUE, password);
/* ... SEC_PKCS12Decoder{Start,Update,Verify,ValidateBags,ImportBags,Finish} */
SECMOD_CloseUserDB(slot);
NSS_Shutdown();
```

Medido **dentro del bundle instalado**, contra la NSS 3.101.4 del runtime, con
un `--filesystem` temporal sólo para meter el binario de prueba (en producción
el destino es el directorio de datos de la propia aplicación y no hace falta
ningún permiso nuevo):

```
$ flatpak run --filesystem=<HOME>/.rfirma-nss-probe --command=sh me.sgomez.rfirma \
    -c 'cd <HOME>/.rfirma-nss-probe && ./probe userdb ./storeG ./legacy.p12 1234'
SECMOD_OpenUserDB OK token='rfirma p12' NeedUserInit=1
PK11_InitPin -> OK
IMPORTADO OK (SECMOD_OpenUserDB) en <HOME>/.rfirma-nss-probe/storeG
NSS_Shutdown -> OK
```

Dos detalles que sólo aparecen midiendo:

- **El almacén queda con dos ficheros y ninguno más**: `cert9.db` y `key4.db`.
  No hay `pkcs11.txt` ni `secmod.db`, y no hacen falta: `certutil -d sql:<dir>`
  y el `C_Initialize` de rfirma con `configdir='sql:<dir>'` lo leen igual.
- **Las CA de la cadena no entran por omisión.** Con `NSS_NoDB_Init` van a la
  base de datos por omisión, que es la de memoria, y se pierden al cerrar: el
  almacén queda con el certificado de firma y nada más.
  `SEC_PKCS12DecoderSetTargetTokenCAs(dcx, SECPKCS12TargetTokenAllCAs)` las mete
  en el almacén del fichero —medido: pasan de una entrada a tres—, **pero
  entran con el apodo `(NULL)`**. Si rfirma necesita la cadena o no es decisión
  de quien implemente; lo que este sondeo deja escrito es que **es una llamada
  de una línea y que hay que tomarla a propósito**, porque el silencio también
  decide.

## 5. La restricción de forma: la contraseña del `.p12` es la del almacén

Es la condición que el #230 impone y el ticket manda verificar. Se cumple, y se
verificó desde fuera del programa que lo creó:

```
$ certutil -L -d sql:<DIR>/storeE
<espécimen FNMT>                                             u,u,u

$ certutil -K -d sql:<DIR>/storeE            # sin contraseña
certutil: could not authenticate to token NSS Certificate DB.:
          SEC_ERROR_BAD_PASSWORD

$ certutil -K -d sql:<DIR>/storeE -f pw.txt  # con la contraseña del .p12
< 0> rsa  dfe8…3fd0   <espécimen FNMT>

$ certutil -K -d sql:<DIR>/storeE -f mal.txt # con otra
Incorrect password/PIN entered.
```

Que es, exactamente, la tercera fila de la tabla de la resolución del #230: se
lista sin sesión, y la clave privada no sale sin la contraseña. `PK11_InitPin`
sobre un almacén recién creado **crea** el almacén con esa contraseña; no hace
falta que exista antes, y `PK11_NeedUserInit` es el que distingue los dos casos.
El camino (1) cumple la restricción de forma sin ningún rodeo.

Una anotación para quien lea trazas: `PK11_InitPin` devuelve `SECSuccess`
dejando `PR_GetError()` en `-8186` (`SEC_ERROR_INVALID_ALGORITHM`). Es basura de
una llamada anterior —NSS sólo garantiza el código de error cuando la función
falla—, y confunde bastante si se registra el error sin mirar el retorno.

## 6. Lo que cuesta cada camino en el empaquetado

| | Flatpak | `.deb` | `.rpm` |
|---|---|---|---|
| **(1) FFI a `libsmime3`/`libnss3`** | 0: están en el runtime, y las cabeceras en el SDK | `Depends: libnss3` **automático** por `dpkg-shlibdeps` | `Requires: libsmime3.so()(64bit)` automático por los *auto-provides* del rpm |
| **(2) `pk12util`** | un módulo nuevo en el manifiesto para compilar o copiar `nss-tools`, más su verificación en `verifica.sh` | `libnss3-tools` (medido: `apt-file search /bin/pk12util` → `libnss3-tools`) | `nss-tools` (`nss.spec:875`, sección `%files tools`) |
| **(3) crate de PKCS#12 en Rust** | 0 de empaquetado, y el precio en otra moneda | 0 | 0 |

Sobre el camino (2) y su cruce con el
[#228](https://github.com/sgomez/rfirma/issues/228): allí se decidió que las
dependencias de OpenSC y PC/SC van como **`recommends`**, con este argumento —
*«rfirma firma sin OpenSC»*, y quien tenga el certificado en Firefox, en
`~/.pki/nssdb` o en un `.p12` no lo necesita. **Ese argumento no vale para
`pk12util`**: sin él, la ficha 17a no funciona en absoluto, así que la
dependencia tendría que ser **`depends`**, no `recommends`. Y sería la primera
dependencia dura del proyecto sobre un **binario ejecutable** de otro paquete,
con el modo de fallo clásico de esa clase: la ruta cambia entre distribuciones,
la salida se analiza por texto y hay una contraseña que pasar por el
descriptor. `pk12util` la lee de fichero (`-k`, `-w`) o del terminal; los
`-K`/`-W` de línea de mandato la dejarían **en la tabla de procesos**, que en
una aplicación de firma no es defendible. Es más trabajo, no menos.

Y en el flatpak, que es el canal principal (ADR-0004, `flatpak-canal-unico.md`),
el camino (2) es directamente el peor: `pk12util` **no está** en
`org.gnome.Platform//50`, así que habría que meterlo, y el manifiesto tiene una
verificación —`verifica.sh`, la invariante del ADR-0012— que existe justamente
para vigilar lo que se cuela dentro del bundle.

## 7. Si acabase ganando el camino (3): los crates

No gana, pero el ticket pide los datos y aquí están, consultados en la API de
crates.io:

| Crate | Última versión | Fecha | Quién | Qué cubre |
|---|---|---|---|---|
| `pkcs12` | **0.2.0-pre.0** | 2026-01-12 | RustCrypto (`RustCrypto/formats`) | las **estructuras** de RFC 7292; el descifrado lo delega en `pkcs5`/`cms` |
| `p12` | 0.6.3 | **2022-02-18** | un solo mantenedor (`hjiayz/p12`) | PKCS#12 completo, sin actividad desde hace cuatro años |

Ninguno de los dos basta solo. El de RustCrypto está **en pre-release desde
hace más de dos años** y para descifrar de verdad hay que traer además `pkcs5`,
`cms`, y los cifrados que los `.p12` de la administración usan de hecho —`rc2`,
`des` (3DES)— más `sha1` para el PBE antiguo. Es decir: **entre cuatro y siete
crates de criptografía**, varios de ellos implementando algoritmos que se
consideran heredados, para leer un fichero que **la persona usuaria elige** y
que por tanto es entrada no confiable. Es la peor combinación posible: parseo
de ASN.1 y descifrado de algoritmos viejos sobre datos ajenos, en el mismo
proceso que sostiene la única parte de rfirma que toca la clave privada.

Y aunque el descifrado saliera bien, quedaría lo peor: la clave privada en
claro en memoria de Rust, para meterla luego con `C_CreateObject`. Eso rompe
literalmente la frase que abre `pkcs11/mod.rs` y el comentario de `cryptoki` en
el `Cargo.toml`. Con el camino (1) **la clave nunca pasa por código nuestro**:
entra del fichero al `key4.db` dentro de NSS.

Nota de paso, por si alguien busca el atajo: **no hay crate de enlace a NSS
utilizable**. `nss-gk-api` (Mozilla, 0.3.0, junio de 2023) es el binding de TLS
para neqo y no expone nada de PKCS#12; `nss-sys` está congelado desde 2016. El
camino (1) son unas quince declaraciones `extern "C"` escritas a mano, que es
poco y es aburrido, que es lo que se quiere.

---

## Recomendación

**Camino (1), con dos correcciones sobre cómo lo describía el #230.**

1. **Enlazar contra `libsmime3.so` además de `libnss3.so`.** El descodificador
   de PKCS#12 no está en `libnss3.so`. `pkg-config --libs nss` ya trae las dos.
2. **`NSS_NoDB_Init(NULL)` + `SECMOD_OpenUserDB`, nunca `NSS_Init` sobre el
   `configdir`.** No convierte el almacén del `.p12` en la base de datos por
   omisión del proceso y hace el `NSS_Shutdown` posterior trivial.

Y una condición de la que depende que funcione:

3. **La importación entera va dentro de `with_token_turn`, y termina con
   `SECMOD_CloseUserDB` + `NSS_Shutdown` antes de soltar el turno.** Es la
   única forma de garantizar que `libsoftokn3.so` no está inicializado mientras
   NSS vive. El turno ya existe y ya serializa todo lo que abre un almacén; la
   importación es una operación más de esa familia. **Conviene fijarlo con una
   prueba**, porque el fallo que evita —el experimento 4— es mudo: no da error,
   da el almacén equivocado.

Coste: cero en empaquetado en los tres canales, `Depends: libnss3` automático
donde toca, y unas quince declaraciones FFI a mano.

**Riesgos que quedan vivos:**

- **La codificación de la contraseña.** El descodificador quiere un
  `BMPString` (UCS-2 *big endian*, terminado en cero). Con ASCII es trivial y es
  lo que se midió. `pk12util` hace además dos cosas para lo demás: registra
  `PORT_SetUCS2_ASCIIConversionFunction` y, si la primera pasada falla,
  **reintenta con los bytes intercambiados** (`p12u_SwapUnicodeBytes`,
  `pk12util.c:175`) porque hay generadores que emiten UCS-2 *little endian*.
  Un `.p12` con contraseña acentuada —perfectamente posible en España— va a
  necesitar ese reintento. No se ha medido: es trabajo de la implementación y
  hay que presupuestarlo.
- **`RFIRMA_PKCS11_MODULE` apuntando a `libsoftokn3.so`** deja softoken
  inicializado para todo el proceso y la importación fallará con un error
  ilegible. Hace falta un mensaje que lo diga.
- **La cadena de CA.** Por omisión no entra en el almacén (§4). Decidir a
  propósito, no por omisión.
- **Deriva de versión de NSS.** El anfitrión tiene 3.120 y el runtime 3.101.4.
  Todos los símbolos usados existen en las dos y son API pública y estable de
  NSS desde hace más de una década, pero la medición dentro del bundle es la que
  manda y hay que rehacerla cuando el runtime salte de versión.

**Lo que este sondeo descarta y por qué**: el camino (2) porque en el canal
principal no existe el binario, porque la dependencia sería dura y no débil
—contra la línea que fijó el #228—, y porque pasar la contraseña por línea de
mandato o por fichero temporal es peor que no pasarla; el camino (3) porque
trae de cuatro a siete crates de criptografía para descifrar un fichero que
elige la persona usuaria y acaba con la clave privada en claro en memoria de
Rust, que es exactamente lo que las dos frases doctrinales del proyecto
prohíben.

---

## Dos hallazgos laterales

**El `.p12` heredado de la FNMT ya no hay que reexportarlo.**
`docs/research/token-flags-login.md:66-68` dice que «`pk12util` rechaza el
cifrado antiguo del `.p12` de la FNMT» y lo reexporta con
`openssl pkcs12 -legacy`. Con NSS 3.120 **ya no**: el mismo fichero, sin tocar,
lo importan tanto `pk12util` (`PKCS12 IMPORT SUCCESSFUL`, `rc=0`) como la API
directa. No se ha investigado en qué versión cambió; lo que sí conviene es no
construir la ficha 17a sobre la creencia de que hace falta reexportar, porque
eso sí traería `openssl` al árbol.

**El campo `LibraryParameters`** (§3) merece una línea en `pkcs11/mod.rs` junto
al comentario que ya explica `pReserved`: quien lea la cabecera de NSS para
entender ese código verá una estructura distinta de la que el código supone, y
las dos son correctas.

---

## Fuentes

Primarias, todas consultadas directamente:

- **Código de NSS**, `cmd/pk12util/pk12util.c` (rama `master` del espejo
  `nss-dev/nss`): `P12U_InitSlot` (l. 321-346) es `PK11_NeedUserInit` →
  `SECU_ChangePW` → `PK11_Authenticate`, y `P12U_ImportPKCS12Object`
  (l. 500-580) es la secuencia `DecoderStart` → `Update` → `Verify` →
  `ValidateBags` → `ImportBags` → `Finish`. El programa de este sondeo repite
  esa secuencia llamada por llamada.
- **Cabeceras de NSS** del sistema: `p12.h:179-200` (firma de
  `SEC_PKCS12DecoderStart`, **ocho** argumentos), `pkcs11t.h:1736-1750`
  (`LibraryParameters`), `secerr.h:203` (`SEC_ERROR_UNKNOWN_PKCS11_ERROR`),
  `secerr.h:24` (`SEC_ERROR_INVALID_ALGORITHM`).
- **`nss.spec` de Fedora rawhide** (`src.fedoraproject.org/rpms/nss`), l. 875:
  `pk12util` está en el subpaquete `%files tools`, o sea `nss-tools`.
  `mdapi.fedoraproject.org` confirma `nss-tools` 3.127.0-1.fc43 y que el
  paquete `nss` provee `libsmime3.so`.
- **`apt-file`** en Ubuntu 26.04: `/usr/bin/pk12util` → `libnss3-tools`;
  `dpkg -S` → `libsmime3.so` está en `libnss3`.
- **API de crates.io** para las versiones y fechas del §7.
- **Código de rfirma**: `pkcs11/mod.rs` (`initialized`, `context`,
  `with_token_turn`), `pkcs11/stores.rs` (`Store::nss`).

## Cómo rehacer la medición

Hace falta `libnss3-dev`, `libnss3-tools`, `gcc`, el kit de pruebas de la FNMT
en `~/.local/share/rfirma-test-certs` y el bundle `me.sgomez.rfirma` instalado.

```bash
# lo que hay dentro del bundle (sin GUI)
cat script.py | flatpak run --command=python3 me.sgomez.rfirma -
flatpak run --command=sh --devel me.sgomez.rfirma -c 'pkg-config --modversion nss'

# el programa de sondeo
gcc -o probe probe.c $(pkg-config --cflags --libs nss nspr) -ldl

./probe two-softokn      dirA dirB    # CKR_CRYPTOKI_ALREADY_INITIALIZED
./probe two-softokn-fin  dirA dirB    # con C_Finalize en medio: sí cambia
./probe softokn-then-nss dirA dirB    # SECFailure -8018   <- el experimento clave
./probe nss-then-softokn dirA dirB    # 0x191 y el almacén equivocado
./probe userdb           dirNuevo fichero.p12 1234   # el camino recomendado

# verificación desde fuera
certutil -L -d sql:dirNuevo
certutil -K -d sql:dirNuevo            # SEC_ERROR_BAD_PASSWORD
certutil -K -d sql:dirNuevo -f pw.txt  # la clave privada
```

Lo imprescindible del `probe.c`, si hay que reescribirlo: llenar el campo
**`LibraryParameters`** —no `pReserved`— de `CK_C_INITIALIZE_ARGS` con
`configdir='sql:…' certPrefix='' keyPrefix='' secmod='secmod.db' flags=readOnly`,
que es la cadena que construye `Store::nss`, y contrastar los flags del token
contra `pkcs11-tool --module libsoftokn3.so -T` con `NSS_LIB_PARAMS` puesto a la
misma cadena. Si los dos no coinciden, la cadena no está llegando.
