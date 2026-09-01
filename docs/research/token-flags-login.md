# `CK_TOKEN_INFO.flags` y `C_Login` en los almacenes que rfirma lista

Sondeo del [#117](https://github.com/sgomez/rfirma/issues/117), hijo del mapa
[#113](https://github.com/sgomez/rfirma/issues/113). Decide la prioridad de la
**ficha 23** del informe *rFirma después de v0.1* (el PIN condicional, v0.4).

La pregunta, tal cual la trae el ticket: ¿qué devuelven exactamente
`CK_TOKEN_INFO.flags` y `C_Login` en los almacenes que rfirma ya lista, para
saber si la ficha 23 es «quitar un diálogo que sobra» o «los usuarios de Firefox
no pueden firmar»?

**Respuesta corta: es «los usuarios de Firefox no pueden firmar»**, con una
salida de emergencia que nadie va a encontrar. Un perfil de Firefox sin
contraseña maestra —el que trae Firefox de fábrica— firma perfectamente **sin
`C_Login`**, pero rfirma le pide un PIN que no existe, y cualquier cosa que se
teclee ahí devuelve `CKR_PIN_INCORRECT` y **aborta la firma**. El único camino
que funciona es enviar el diálogo con el campo **vacío**, que sí devuelve
`CKR_OK`. Nada en la interfaz lo sugiere y el mensaje que se ve es «PIN
incorrecto».

---

## Qué se midió y cómo

### El método

Las medidas salen de un programa desechable de un solo fichero que usa **la
misma versión de `cryptoki` (0.12) que `rfirma-app`** y, sobre todo, **las
mismas llamadas**: `CInitializeArgs::new_with_reserved` con los init args que
construye `Store::nss`, `open_ro_session`, y `session.login(UserType::User,
Some(&AuthPin::new(...)))`, que es literalmente la línea de
`sign_holding_the_turn`. No es una reimplementación del recorrido: es el mismo
camino con trazas.

Para cada ranura con token inicializado imprime `CKF_LOGIN_REQUIRED`,
`CKF_USER_PIN_INITIALIZED`, `CKF_PROTECTED_AUTHENTICATION_PATH`,
`CKF_TOKEN_INITIALIZED` y el rango de PIN; después prueba a firmar **sin
`C_Login`**, y luego, en una sesión nueva por cada uno, `C_Login(User, "1234")`,
`C_Login(User, "")` y `C_Login(User, NULL)`, cada uno seguido de un `C_Sign` con
`CKM_SHA256_RSA_PKCS` —el mecanismo del ID-16—.

Un proceso por almacén: `C_Initialize` es por proceso y módulo, y los perfiles
NSS solo se distinguen por sus init args.

### Los cuatro almacenes

Tres son **perfiles NSS desechables** creados en `/tmp` con `certutil -N`, con
el certificado del kit de pruebas de la FNMT
(`docs/research/token-pkcs11-pruebas.md`) dentro:

- **`nopass`** — `certutil -N --empty-password`, que es exactamente lo que deja
  Firefox cuando nadie pone contraseña maestra.
- **`master`** — `certutil -N -f` con la contraseña `secreto`.
- **`emptydb`** — `--empty-password` y **sin** certificados: el `~/.pki/nssdb`
  recién creado por cualquier aplicación de GNOME o por Chrome.

El cuarto almacén es el token SoftHSM **`rfirma-test`** (PIN `1234`, módulo
`/usr/lib/softhsm/libsofthsm2.so`).

Los tres perfiles se montan así, y con esto basta para rehacer la medición:

```bash
S=/tmp/rfirma-117/nss; rm -rf "$S"; mkdir -p "$S"; cd "$S"
P12=~/'.local/share/rfirma-test-certs/Claves RSA/AC FNMT Usuarios/Nuevos/Nuevo Perfil no SMIME/ACTIVO_EIDAS_CERTIFICADO_PRUEBAS___99999999R.p12'

# pk12util rechaza el cifrado antiguo del .p12 de la FNMT: se reexporta.
openssl pkcs12 -in "$P12" -passin pass:1234 -nodes -legacy \
  | openssl pkcs12 -export -passout pass:1234 -name FNMT -out modern.p12

printf ''        > empty.txt
printf 'secreto' > master.txt

certutil -N -d sql:"$S/nopass"  --empty-password
pk12util -i modern.p12 -d sql:"$S/nopass"  -W 1234 -K ""
certutil -N -d sql:"$S/master"  -f master.txt
pk12util -i modern.p12 -d sql:"$S/master"  -W 1234 -k master.txt
certutil -N -d sql:"$S/emptydb" --empty-password
```

Los init args con los que se abre cada perfil son los que construye
`Store::nss`, literalmente:

```
configdir='sql:<perfil>' certPrefix='' keyPrefix='' secmod='secmod.db' flags=readOnly
```

**Ni el perfil real de Firefox del titular ni su `~/.pki/nssdb` ni su
certificado personal se tocan en ningún punto.**

Versiones: `libnss3` 2:3.120-1ubuntu2.1
(`/usr/lib/x86_64-linux-gnu/libsoftokn3.so`), SoftHSM 2.6.1, `cryptoki` 0.12.0.

### Lo que queda fuera del alcance

**`CKF_PROTECTED_AUTHENTICATION_PATH` no se ha podido medir contra ningún token
real.** No hay DNIe ni lector con teclado (pinpad) en este equipo, y ni NSS ni
SoftHSM lo anuncian nunca —los cuatro almacenes lo dan a `false`, como se ve en
la tabla—. Lo que se dice más abajo sobre ese flag sale de la especificación y
del código de SunPKCS11, no de una medición, y está marcado como tal.

---

## Tabla de flags por almacén

Solo se listan las ranuras que tienen algo que decir; ver la nota sobre la
ranura `NSS Generic Crypto Services` justo debajo.

| Almacén (ranura) | `CKF_LOGIN_REQUIRED` | `CKF_USER_PIN_INITIALIZED` | `CKF_PROTECTED_AUTHENTICATION_PATH` | PIN mín/máx |
|---|---|---|---|---|
| Firefox **sin** contraseña maestra (`NSS Certificate DB`) | `false` | `true` | `false` | 0/500 |
| Firefox **con** contraseña maestra (`NSS Certificate DB`) | **`true`** | `true` | `false` | 0/500 |
| `~/.pki/nssdb` vacío, contraseña vacía (`NSS Certificate DB`) | `false` | `true` | `false` | 0/500 |
| SoftHSM `rfirma-test` | **`true`** | `true` | `false` | 4/255 |
| *(cualquier perfil NSS)* `NSS Generic Crypto Services` | `false` | `false` | `false` | 0/0 |

**La ranura `NSS Generic Crypto Services` hay que filtrarla por flags, no por
índice.** Todo perfil NSS anuncia dos ranuras con token inicializado; la primera
es la interna de cifrado, no guarda claves de usuario y contesta
`CKR_USER_TYPE_INVALID` a **cualquier** `C_Login`, con PIN, con cadena vacía o
con `NULL`. Se reconoce porque es la única con `CKF_USER_PIN_INITIALIZED` a
`false` y rango de PIN 0/0. Hoy rfirma abre sesión sobre ella al listar (no le
hace daño: no tiene certificados) y nunca la elige para firmar, porque
`slot_of` busca por etiqueta de token y ningún certificado sale de ahí.

---

## `C_Login` y `C_Sign`, resultados reales

Salida literal de la sonda, condensada a lo que importa. «cadena vacía» es `""`
—puntero válido, longitud 0— y `NULL` es `NULL_PTR`.

| Almacén | sin `C_Login` | `C_Login(User, "1234")` | `C_Login(User, "")` | `C_Login(User, NULL)` |
|---|---|---|---|---|
| Firefox **sin** contraseña | **1 clave visible, `C_Sign` → `CKR_OK` (256 B)** | `CKR_PIN_INCORRECT` — y **la clave sigue visible y firma igual** | `CKR_OK`, firma | `CKR_OK`, firma |
| Firefox **con** contraseña `secreto` | 0 claves | `CKR_PIN_INCORRECT` | `CKR_PIN_INCORRECT` | `CKR_PIN_INCORRECT` |
| ídem, con la contraseña buena | — | `C_Login(User, "secreto")` → `CKR_OK`, 1 clave, firma | — | — |
| `~/.pki/nssdb` vacío | 0 claves (no hay ninguna) | `CKR_PIN_INCORRECT` | `CKR_OK` | `CKR_OK` |
| SoftHSM `rfirma-test` | 0 claves | **`CKR_OK`, 6 claves, firma** | `CKR_PIN_INCORRECT` | `CKR_ARGUMENTS_BAD` |

Tres cosas que la tabla dice y conviene leer despacio:

1. **En un Firefox de fábrica se firma sin iniciar sesión.** La clave privada es
   visible y `C_Sign` devuelve una firma de 256 bytes sin haber llamado a
   `C_Login` ni una vez. El flag lo anunciaba: `CKF_LOGIN_REQUIRED` a `false`.
2. **Un `CKR_PIN_INCORRECT` de softoken no rompe nada.** Tras el `C_Login` con
   `"1234"`, la misma sesión sigue viendo la clave y sigue firmando. El fallo es
   informativo, no destructivo — softoken no cambia el estado de la sesión.
3. **La cadena vacía y `NULL` no son intercambiables.** En NSS las dos valen; en
   SoftHSM `""` es `CKR_PIN_INCORRECT` y `NULL` es `CKR_ARGUMENTS_BAD`. Quien
   escriba la ficha 23 no puede tratarlas como el mismo caso.

### Por qué sale eso: las fuentes

- **PKCS#11 v2.40 (OASIS), tabla 6 y §5.6.** `CKF_LOGIN_REQUIRED` significa «hay
  funciones que exigen iniciar sesión»; `CKF_PROTECTED_AUTHENTICATION_PATH`
  significa que el PIN se teclea fuera de la biblioteca y que `C_Login` se llama
  con `pPin = NULL_PTR`. La especificación **no** dice en ningún sitio que
  `C_Sign` exija sesión iniciada: eso lo decide cada token.
- **NSS softoken**, `lib/softoken/pkcs11.c`. `NSC_GetTokenInfo` (~l. 4431)
  distingue tres estados: sin contraseña en la base → solo `CKF_LOGIN_REQUIRED`;
  contraseña **vacía** —lo que crea Firefox por defecto— → solo
  `CKF_USER_PIN_INITIALIZED`; contraseña real → los dos. `NSC_Login` (~l. 5058):
  si no hace falta iniciar sesión, `ulPinLen == 0` devuelve `CKR_OK` y cualquier
  PIN devuelve `CKR_PIN_INCORRECT` **sin tocar el estado**. Con contraseña vacía
  la ranura arranca ya como `isLoggedIn` (`pkcs11u.c` ~l. 2098), y por eso las
  claves privadas se ven y firman. NSS **nunca** pone
  `CKF_PROTECTED_AUTHENTICATION_PATH`.
- **SoftHSM v2**, `OSToken.cpp` l. 101 y `SoftHSM.cpp` l. 1633.
  `CKF_LOGIN_REQUIRED` siempre; `pPin == NULL` → `CKR_ARGUMENTS_BAD`; PIN vacío
  → `CKR_PIN_INCORRECT`. Una clave privada sin sesión iniciada da
  `CKR_USER_NOT_LOGGED_IN` (`access.cpp` l. 528) — aquí ni siquiera se ve, así
  que la sonda se queda en «0 claves».
- **SunPKCS11**, `SunPKCS11.login` (~l. 1600). **Se salta el inicio de sesión si
  `CKF_LOGIN_REQUIRED` está a cero**, y entonces no dispara ningún diálogo; con
  `CKF_PROTECTED_AUTHENTICATION_PATH` llama a `C_Login` con PIN nulo.
- **AutoFirma**, `NssKeyStoreManager.java` l. 74-98: carga primero con
  contraseña vacía y solo si eso falla pide la del almacén. **AutoFirma no mira
  los flags**: delega el problema entero en SunPKCS11, que sí los mira. El
  comportamiento que ve el usuario de AutoFirma —Firefox de fábrica firma sin
  preguntar nada— es el de SunPKCS11, no el de AutoFirma.

---

## Qué hace hoy el recorrido de rfirma

El PIN entra por `sign_with_pin` (`rfirma-app/src-tauri/src/commands/mod.rs`
l. 853) y llega a `sign_holding_the_turn`
(`rfirma-app/src-tauri/src/pkcs11/mod.rs` l. 377), cuyo único trato con la
sesión es:

```rust
match session.login(UserType::User, Some(&AuthPin::new(pin.into()))) {
    Ok(()) => {}
    Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => {}
    Err(other) => return Err(other.into()),   // l. 402
}
```

Es decir: **rfirma llama a `C_Login` siempre, con lo que se haya tecleado, y
cualquier error que no sea `CKR_USER_ALREADY_LOGGED_IN` aborta la firma antes de
llegar al `C_Sign`.** No mira `CKF_LOGIN_REQUIRED` en ningún punto —el flag no
se lee en todo el módulo—.

Arriba, el diálogo tampoco pregunta nada: `useSigning.ts` l. 96 pasa a
`{ kind: "pin" }` en cuanto la prefirma sale bien, sin condición, y
`PinDialog.tsx` no marca el campo como obligatorio ni deshabilita el botón de
enviar con el campo vacío. `RvError::PinIncorrect` se traduce a
`Situation::IncorrectPin` → `"incorrectPin"`, que `belongsToPinDialog` clasifica
como reintentable: el diálogo **no se desmonta**, se vacía el campo y se enseña
el error. `attempts_left` va fijo a `None` (`commands/mod.rs` l. 101), así que
tampoco hay un contador que delate que ese token no tiene intentos que gastar.

Caso por caso, esto es lo que le pasa hoy a una persona:

| Almacén | Lo que ve hoy en rfirma |
|---|---|
| **Firefox sin contraseña maestra** (el de fábrica) | Se le pide un PIN que **no existe**. Si teclea cualquier cosa: `CKR_PIN_INCORRECT`, «PIN incorrecto», y el diálogo se queda ahí para que vuelva a intentarlo — para siempre, porque no hay ningún PIN correcto. Si por casualidad **pulsa Firmar con el campo vacío**, `C_Login(User, "")` devuelve `CKR_OK` y **firma bien**. |
| **Firefox con contraseña maestra** | Funciona, por accidente feliz: el «PIN» que se le pide es su contraseña maestra, `C_Login` la acepta y firma. El único pero es que la etiqueta dice PIN y no «contraseña maestra». |
| **`~/.pki/nssdb` vacío** | No llega nunca al diálogo: no hay certificados que listar, así que no hay nada que elegir ni que firmar. |
| **SoftHSM / tarjeta con PIN** | Funciona como se espera: se pide el PIN, `C_Login("1234")` da `CKR_OK` y firma. Es el caso para el que se escribió el recorrido. |
| **Lector con teclado (`CKF_PROTECTED_AUTHENTICATION_PATH`)** | **Sin medir**, no hay hardware. Por el código: rfirma llamaría a `C_Login` con el PIN tecleado en pantalla en vez de con `NULL`, que no es lo que la especificación manda para esos lectores. Es un tercer caso roto, probable pero no comprobado. |

La fila que decide el ticket es la primera. Y hay un detalle que la agrava: al
listar, la ranura de un Firefox sin contraseña **sí** enseña su clave privada,
así que el filtro del ID-07 funciona y el certificado **aparece en la lista**. Se
le ofrece firmar con él, se le pide un PIN inexistente y se le dice que lo ha
escrito mal. Es el peor orden posible de esos tres pasos.

---

## Recomendación sobre la ficha 23

**La ficha 23 no es «quitar un diálogo que sobra». Es «los usuarios de Firefox no
pueden firmar», y por eso debería salir de v0.4.**

El argumento, en tres pasos medidos:

1. El perfil que Firefox crea de fábrica —sin contraseña maestra— es el caso
   **mayoritario** entre quienes tienen su certificado en Firefox, que es como
   la mayoría de la gente guarda el de la FNMT.
2. En ese perfil rfirma **no puede firmar** salvo enviando el diálogo vacío, algo
   que la interfaz no insinúa y que el mensaje de error empuja activamente a no
   hacer: quien lea «PIN incorrecto» vuelve a teclear, no borra el campo.
3. El fallo es **silencioso e irrecuperable desde la interfaz**: el diálogo se
   queda reintentando sin límite y sin contador, así que ni siquiera parece un
   fallo del programa. Parece que la persona no se acuerda de su PIN.

Que exista la salida de emergencia del campo vacío es lo único que impide
llamarlo «bloqueante absoluto», pero no cambia la prioridad: un camino correcto
que solo se acierta por descarte no es un camino.

### La regla que sale del sondeo, para cuando se escriba la ficha

Antes de pedir nada, leer `CK_TOKEN_INFO.flags` de la ranura elegida —
`cryptoki` 0.12 ya lo da hecho con `TokenInfo::login_required()`,
`user_pin_initialized()` y `protected_authentication_path()`:

- `login_required() == false` → **no llamar a `C_Login` y firmar directamente.**
  Ni diálogo, ni PIN, ni cadena vacía. Es lo que hace SunPKCS11, y por tanto lo
  que hace AutoFirma, que es el oráculo.
- `login_required() == true` y `protected_authentication_path() == false` → pedir
  el PIN como hoy. Si el almacén es un perfil de Firefox, la etiqueta debería
  decir «contraseña maestra», no «PIN».
- `protected_authentication_path() == true` → `login(User, None)` y esperar al
  teclado del lector, sin pedir nada por pantalla. **Sin medir**: hay que
  comprobarlo cuando haya un DNIe o un lector con pinpad delante, y hasta
  entonces esta rama es una lectura de la especificación, no un hecho.

Dos avisos para quien la implemente, los dos medidos arriba: `""` y `NULL` **no**
son el mismo caso (NSS acepta los dos, SoftHSM ninguno), y hay que seguir
tratando `CKR_USER_ALREADY_LOGGED_IN` como éxito, como hoy.

---

## Reproducir

Monta los tres perfiles con el bloque de «Los cuatro almacenes» y recórrelos
**un proceso por almacén** —`C_Initialize` es por proceso y módulo—, con
`/usr/lib/x86_64-linux-gnu/libsoftokn3.so` para los perfiles NSS y
`/usr/lib/softhsm/libsofthsm2.so` para el token de pruebas.

Para los flags de la primera tabla basta `pkcs11-tool` (`opensc`), sin escribir
nada:

```bash
NSS=/tmp/rfirma-117/nss/nopass
pkcs11-tool --module /usr/lib/x86_64-linux-gnu/libsoftokn3.so -T   # ver nota
pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so         -T
```

`pkcs11-tool` no acepta los init args de NSS, así que para los perfiles hay que
apuntarlo al perfil por `NSS_LIB_PARAMS` o repetir la llamada desde código con
los init args de arriba; esto último es lo que se hizo aquí.

La segunda tabla —`C_Login` con `"1234"`, con `""` y con `NULL`, y el `C_Sign`
posterior— sí necesita código, porque `pkcs11-tool` no distingue la cadena vacía
de `NULL`, que es justo la diferencia que el sondeo mide. Son unas cien líneas
con `cryptoki` 0.12: `CInitializeArgs::new_with_reserved` con los init args,
`open_ro_session`, `login(UserType::User, …)` en una sesión nueva por variante y
un `C_Sign` con `CKM_SHA256_RSA_PKCS`, el mecanismo del ID-16.

Requiere `certutil`/`pk12util` (`libnss3-tools`), `openssl`, el kit de pruebas de
la FNMT en `~/.local/share/rfirma-test-certs` y el token SoftHSM `rfirma-test`
ya inicializado. Los perfiles viven en `/tmp` y se borran con `rm -rf`.
