# El servidor local usa HTTPS, y la CA la instala la aplicación en los almacenes NSS de cada persona

Las sedes electrónicas invocan al cliente de firma desde una página servida por HTTPS, así
que `rfirma` levanta un servidor **HTTPS** en `127.0.0.1`. Para que el navegador confíe en
él, **la propia aplicación** registra su **CA local** en los **almacenes NSS de la persona
que la usa**, en su sesión y en su primer arranque. Ni el instalador, ni `root`, ni un
`postinst`, ni un `pkexec`, ni el almacén de CA del sistema.

Son **dos piezas** y conviene no confundirlas:

- La **CA local** es lo que se registra en los almacenes NSS. No identifica a nadie ni firma
  documentos: su único trabajo es firmar la otra pieza. **Se conserva su clave privada**, y
  **vive 2–3 años** (§ *La retirada*).
- El **certificado del servidor local** es el que rfirma presenta en cada saludo TLS:
  `CN=localhost`, SAN `DNS:localhost` **e** `IP:127.0.0.1` —esa forma exacta, la única que
  pasa en los cuatro verificadores medidos en el
  [#310](https://github.com/sgomez/rfirma/issues/310) y el
  [#326](https://github.com/sgomez/rfirma/issues/326)—, firmado por la CA local. **No se
  guarda en disco**: se genera en memoria en cada arranque y vive lo que vive el proceso.

El *host* y el protocolo **no son variables de diseño**: `autoscript.js`, que sirve la sede y
no nosotros, tiene cableados `SERVER_HOST = "127.0.0.1"` y
`URL_REQUEST_PREFIX = "wss://" + SERVER_HOST + ":"`. Migrar a `ws://` en texto plano no
está sobre la mesa, y **ninguna CA pública puede emitir para `127.0.0.1`**: el CA/Browser
Forum lo prohíbe desde el 11 de noviembre de 2015 por ser IP reservada, y los existentes
debían revocarse antes del 1 de octubre de 2016. La CA local es obligatoria.

**El puerto sí es variable, y no lo elegimos nosotros.** Una redacción anterior de este ADR
decía `127.0.0.1:63117`, y es falso para el transporte vigente: la sede **sortea tres puertos**
del rango efímero y los manda en `afirma://websocket?ports=…`; rfirma se queda con el primero
que abra. El `63117` es el puerto fijo del protocolo v3, que no es el camino de este hito.
Medido en el [#309](https://github.com/sgomez/rfirma/issues/309). Que el puerto cambie de un
trámite a otro **no afecta al permiso de red local**: la concesión del navegador persiste por
*(origen de la sede, espacio de direcciones)*, no por IP y puerto.

**La v0.4 decide esto y no implementa nada; el código va con el servidor del protocolo, que
es la v0.5.** Instalar una raíz de confianza en el navegador de alguien para un servidor que
todavía no existe es pasivo de seguridad puro.

## Dónde se escribe, y qué no

Los almacenes son **los que declare `pkcs11::stores`**, y este ADR **no los enumera a
propósito**. Enumerarlos aquí es fabricar una segunda lista que se desincroniza del código,
que es exactamente lo que pasó cuando Firefox 147 se mudó a XDG y no se enteró ni el
manifiesto, ni los informes, ni `stores.rs`
([#246](https://github.com/sgomez/rfirma/issues/246)).

**El almacén de CA del sistema se retira entero.** En el flujo de rfirma no tiene ningún
consumidor: el navegador no lo lee en Debian ni en Ubuntu —medido en el
[#225](https://github.com/sgomez/rfirma/issues/225)— y rfirma no valida su propio
certificado. Sin él no hace falta `root` en ningún punto, y **los tres canales ejecutan
literalmente el mismo código**. Si algún día aparece un consumidor real, se añade entonces
con un motivo concreto.

**La política empresarial de Firefox** (`policies.json`, `Certificates.Install`) tampoco se
usa: se lee sólo al arranque, exige `root` en `/etc`, y escribe la confianza de forma que la
persona no puede deshacerla desde la interfaz. Y `security.enterprise_roots.enabled` **no
existe en Linux** (bug 1600509, WONTFIX).

## Lo que le cuesta al flatpak, dicho entero

Los canales nativos escriben en esas rutas sin pedir permiso a nadie. El flatpak necesita
declararlas en el manifiesto, y la concesión **pasa de `:ro` a lectura y escritura**: un
atacante que ya ejecute código en rfirma pasa de **leer** `logins.json`, `key4.db` y
`cookies.sqlite` —que ya puede desde el [#101](https://github.com/sgomez/rfirma/issues/101),
porque listar certificados de Firefox lo exige— a poder **sustituirlos** y a poder plantar
una raíz de confianza propia. Es la capacidad que hay que ejercer: es el enunciado, no un
extra. Sigue sin ser `--filesystem=home` —`~/.ssh`, `~/.gnupg`, los llaveros y `~/.aws` no
aparecen— y sigue siendo un subdirectorio de `~/.config`, no `~/.config` entero.

La regla que gobierna esa lista: **lo que tiene ruta conocida se declara y se usa; de lo que
no la tiene, el flatpak no se entera y no puede avisar.** No se declara nada `:ro` «para
detectar»: no se puede detectar lo que no se ha declarado, y declararlo `:ro` cuesta lo
mismo que declararlo `rw` y que funcione. Lo que sí puede decir la interfaz, en los tres
canales, es **cuántos almacenes ha encontrado**, que es un hecho y no una detección.

**El permiso está puesto desde la v0.5**, que es cuando entró el código que lo consume
(`app::trust`), y ni un ciclo antes: un permiso sin consumidor es el mismo pasivo que se
rechaza arriba para la CA. Va **sin `:create`**: un perfil que no existe lo crea el
navegador, no nosotros, y materializar un directorio en el `$HOME` de la persona sin que
nada lo use es ese mismo pasivo con otra cara. Un perfil ausente se salta y no deja sin CA
a los demás.

Y con el permiso se invierte **la comprobación** de `packaging/flatpak/verifica.sh`: donde
exigía que el sandbox **no** pudiera escribir en esas rutas, ahora exige que **sí** pueda.
La inversión es el enunciado; leerla como un descuido es lo único que hay que evitar.

## Cuándo, y qué se le dice a la persona

**En el primer arranque de rfirma**, no en la primera invocación desde una sede. La opción
atractiva era la segunda, porque ahí hay un motivo concreto que enseñar, y se cae por
medición: **Chrome nunca relee su `nssdb` en caliente** (no hay `FilePathWatcher` sobre esos
ficheros; sólo notifica cuando el propio Chrome modifica la base), y **Firefox envenena su
caché de confianza** si ya falló contra ese certificado (`CertVerifier::mTrustCache`; bug
1156713 **WONTFIX**). Instalar en mitad del trámite significaría pararlo para pedir que
reinicie el navegador y vuelva a empezar. En el primer arranque, reiniciar el navegador no
le cuesta nada a nadie.

Se narra mientras se hace, se avisa después de que reinicie el navegador, y **queda visible
y retirable en Preferencias con su fecha de caducidad a la vista**. **No hay diálogo de
permiso previo**: el navegador ya interrumpe ese flujo con un permiso real e ineludible —el
de Local Network Access, abajo—, así que el nuestro dejaría de ser el guardián y pasaría a
ser ruido delante del guardián.

**rfirma no mata el navegador de nadie.** AutoFirma lo hace desde su `preinst` y su `prerm`
(`pkill firefox`, como `root`, sin aviso), y el
[#238](https://github.com/sgomez/rfirma/issues/238) documentó que esa conducta ni siquiera
nació por los certificados: el commit original mataba tres navegadores «para definir
correctamente las preferencias» y el comentario se reescribió un año después.

## La retirada, decidida antes que la instalación

1. **La CA local caduca sola, a los 2–3 años.** Su caducidad es **lo único** que hace que un
   residuo abandonado deje de valer sin que nadie se acuerde de borrarlo: su clave privada vive
   en el `$HOME`, que un `apt remove` no toca, pero queda inerte en cuanto el navegador deja
   de confiar. Es lo único que funciona cuando la desinstalación es un `apt remove` a las tres
   de la mañana en una máquina con cuatro usuarios.

   > Una redacción anterior decía **90 días**, y era un agujero disfrazado de prudencia. Los 90
   > días se le ponían a una pieza que **ya no existe** —el certificado del servidor local ya no
   > se guarda en ningún sitio— y, sobre todo, **a partir de la v0.5 la aplicación la abre la
   > sede**: una caducidad más corta que el hueco entre dos usos garantiza el camino malo todas
   > las veces, porque quien firma dos o tres veces al año se la encontraría caducada casi
   > siempre. Diez años, como AutoFirma, es no tener red: el
   > [#225](https://github.com/sgomez/rfirma/issues/225) midió el resultado, CA huérfanas
   > válidas hasta 2033. Dentro de la banda 2–3 el número es un juicio; los dos valen igual.

2. **Reemitir el certificado del servidor local no toca el `nssdb`.** El punto de confianza es
   la CA local; el certificado del servidor viaja en el saludo TLS. Por eso generarlo en memoria
   en cada arranque es gratis y no interrumpe a nadie.

3. **Solape: la CA local siguiente se instala meses antes**, mientras la vigente sigue
   sirviendo. El [#326](https://github.com/sgomez/rfirma/issues/326) midió que dos certificados
   de confianza con el mismo sujeto conviven en Firefox y en Chrome, en cualquier orden. Consiste
   en *no tirar lo viejo todavía*, así que es barato, y convierte la reparación en el camino
   excepcional.

   «Sigue sirviendo» es literal y es lo que le da sentido: la vigente **sigue siendo la que firma
   el certificado del servidor local** hasta que caduca. Por eso el almacén guarda **dos ranuras**
   —la que sirve y la siguiente— y no una: si la siguiente ocupara la ranura de la vigente al
   fabricarse, el navegador que ya estaba abierto recibiría desde ese mismo arranque un
   certificado firmado por una CA que no ha cargado, y el trámite inmediatamente posterior a la
   renovación fallaría igual que sin solape. El relevo llega cuando la vigente caduca: la
   siguiente pasa a servir **sin instalar nada**, porque lleva meses en los almacenes, y ahí es
   donde el solape se cobra —nadie tiene que reiniciar el navegador—.

4. **No se repara en caliente.** «Reparar y continuar» no existe —Chrome no relee el `nssdb`,
   Firefox envenena su caché tras haber fallado—: sólo existe **reparar y volver a empezar**, y
   se dice así, sin fingir lo otro. Además rfirma **no puede distinguir** «no hay confianza» de
   «el navegador ha denegado el acceso a la red local»
   ([#309](https://github.com/sgomez/rfirma/issues/309)): el saludo TLS ni siquiera empieza.
   El aviso de **«reinicia el navegador» va al final del trámite**, no al empezarlo: es el único
   momento en el que interrumpir no cuesta nada.

5. **Retirada explícita desde Preferencias**, siempre disponible. Con el solape puede haber **dos
   CA locales vivas a la vez**, y la retirada **tiene que llevarse las dos**.

6. **Borrado por huella del certificado, nunca por *nickname*.** Es literalmente el fallo
   medido en el #225: hay CA huérfanas de AutoFirma marcadas `CT,C,C` y válidas hasta 2033
   que ningún desinstalador retira, porque borra por un *nickname* que ya no es el que usa la
   versión actual.

7. **Verificar la confianza es leer los bits con `certutil -L`, no verificar una cadena.** El
   #326 midió que el veredicto de `vfychain` puede salir **invertido** respecto a lo que hace el
   navegador de verdad.

No hay retirada desde el `prerm`: `root` no tiene sesión, y recorrer los `$HOME` ajenos es
justo lo que la Debian Policy §9.1.2 existe para impedir. La caducidad es la red.

## La CA no resuelve la conectividad

**Local Network Access se aplica por dirección de destino, no por esquema**, así que
`wss://` con nuestra CA instalada recibe exactamente el mismo aviso de permiso del navegador
que `ws://` en claro. Está en vigor: **Chrome 147** (7 de abril de 2026) y **Firefox 154**
(18 de agosto de 2026), activado por defecto y cubriendo WebSocket.

La CA sigue siendo obligatoria, pero por un motivo más estrecho: sin ella el `wss://` que la
sede exige **ni siquiera completa el saludo TLS**. Queda escrito para que nadie lea este ADR
y concluya que con la CA ya conecta. El diseño del aviso —y en particular distinguir en el
mensaje de error «no hay aplicación escuchando» de «el navegador ha denegado el acceso a
loopback», que son remedios opuestos— es de la v0.5.

## Considered Options

- **El almacén de CA del sistema, con `root` una vez al instalar.** Era la decisión
  anterior de este ADR. La mata el #225: en Debian y Ubuntu ese almacén **no llega a
  ningún navegador**. Y la mata también la forma: los almacenes que importan son por
  usuario, así que el usuario creado después de instalar no recibe nada, ni el perfil de
  Firefox nuevo, ni el Chrome que estrenó `~/.local/share/pki/nssdb`.
- **Hacer las dos cosas, como AutoFirma** —`postinst` como `root` sobre todos los `/home`,
  más un camino de sesión desde la aplicación—. Que su menú «Restaurar la instalación»
  exista es la confesión de que la vía de `root` no aguanta sola. Si el camino de sesión hay
  que escribirlo igualmente, tener además el de `root` es un segundo mecanismo para el mismo
  enunciado, a cambio de violar la Debian Policy §9.1.2, dejar ficheros NSS propiedad de
  `root` en el `$HOME` de la persona, y un flatpak que no puede replicarlo. Dos raíces de
  confianza para lo mismo es peor seguridad, no más.
- **Pedir permiso con un diálogo antes de instalar la CA.** El argumento a favor es serio
  —lo que se instala es la única cosa que, mal hecha, sirve para suplantar cualquier sitio
  web—, y se abandona por el aviso de Local Network Access: el navegador ya pregunta, y
  nuestra pregunta delante de la suya es ruido. Lo que de verdad resolvía el problema del
  residuo —visible, con caducidad y retirable— se conserva entero.
- **Que el flatpak no instale la confianza** y quede como el canal degradado. Rompe el
  «los tres canales ejecutan el mismo código» que hace barata toda esta decisión, y con él
  la única manera de razonar sobre un fallo sin preguntar primero cómo se instaló.
- **Declarar en el flatpak sólo los almacenes de un navegador**, o sólo los no confinados.
  Es la división menos predecible de todas: la misma instalación funcionaría o no según qué
  navegador tenga abierto la persona. El #246 lo descarta por eso, no por coste.
- **Un certificado del servidor local autofirmado, sin CA local**, registrado él mismo como
  certificado de confianza. El #326 midió que **existe**: carga sin aviso en Firefox 155 y
  Chrome 152. Se descarta por **robustez**, no por seguridad —en residuo están empatadas: una
  clave, 2–3 años, y `nameConstraints` deja a la CA local acotada exactamente a los mismos
  nombres que podría afirmar un certificado de servidor—. Se sostiene sobre una intersección
  estrecha y no documentada de dos verificadores: la única cadena de bits que sirve en los dos
  es `PC,,`, y **cada bit por separado falla en uno de ellos**, siendo uno de esos fallos el
  fatal (`ERR_CERT_INVALID` en Chrome, sin salida manual). Un cambio en el verificador de
  Chrome rompería rfirma sin aviso y sin remedio para la persona. Su única ventaja real —cada
  renovación se ve, o sea reinicio del navegador— es justo lo que el solape hace innecesario.
- **El modelo AutoFirma**: CA local con la clave privada **tirada** más certificado del
  servidor guardado en disco y de vida larga. Está **dominado** por la opción anterior, que es
  lo mismo con una pieza menos y la misma renovación visible. Y su promesa de «residuo inerte»
  **no es cierta en rfirma**: AutoFirma puede decirlo porque su `.pfx` vive en el directorio de
  instalación y se lo lleva el desinstalador; aquí las claves viven en el `$HOME`, que un
  `apt remove` no toca. No existe ningún diseño con cero claves privadas en disco: sólo cambia
  cuál es la que está ahí.
- **Un gancho de sesión** —autoarranque XDG o unidad de usuario de `systemd`— que diera un
  momento tranquilo anterior al navegador. Funciona, y es la respuesta más directa al hecho de
  que desde la v0.5 la aplicación la abre la sede: al iniciar sesión el navegador aún no ha
  arrancado, así que los dos obstáculos medidos desaparecen. Pero el solape le quita casi todo
  el trabajo, y lo que queda no paga sus dos costes: **partir el comportamiento de los tres
  canales** —un flatpak no instala unidades de usuario ni escribe en `~/.config/autostart`:
  tendría que pedirlo por el portal `Background`, con su diálogo de permiso— y meter justo el
  diálogo que este ADR se esfuerza en no tener. Tampoco garantiza el orden si la persona
  restaura sesión con el navegador dentro. Vuelve **como enmienda explícita a este ADR, y con
  el flatpak degradado a propósito**, el día que el solape se demuestre insuficiente.

## Consequences

- La CA local **no se distribuye precompilada**: su clave privada sólo puede vivir en la
  máquina de quien la usa, con permisos restrictivos. Y **no se protege con una constante en el
  código** —`RestoreConfigLinux.java` lleva `KS_PASSWORD = "654321"` en el fuente público de
  AutoFirma—.
- **La clave privada de la CA local va en un fichero propio, `0600`, sin cifrar**, creado con
  ese modo desde el principio y no con un `chmod` posterior. Dentro del flatpak, en el
  directorio de datos de la aplicación. Es el mismo trato que `~/.ssh/id_*` y el llavero de GPG.
- **El atacante que ya ejecuta código como la persona queda declarado fuera del modelo de
  amenaza**, y se declara aquí para que nadie lo «mejore» dentro de un año. Se descartó el
  llavero del escritorio (Secret Service) porque **no aísla por aplicación**: cualquier proceso
  que corra como la persona habla por D-Bus con un llavero ya desbloqueado y lee el secreto, y
  el portal `org.freedesktop.portal.Secret` aísla a un flatpak de otro flatpak, no de un binario
  corriente. Y porque el suelo no lo pone nuestro fichero: ese mismo atacante **escribe él en el
  `nssdb`** y planta su propia raíz de confianza, sin `nameConstraints` y sin caducidad, que es
  estrictamente más poderoso que robarnos nada.
- **La CA local va restringida**: `keyUsage` reducido a `keyCertSign`+`cRLSign`, y
  `nameConstraints` limitando la emisión a `localhost` y `127.0.0.1` — que el
  [#310](https://github.com/sgomez/rfirma/issues/310) midió que se imponen de verdad en los tres
  motores, incluida la restricción sobre `iPAddress`, y con la violación **visible y no
  salteable**.
- **Se registra por la API de NSS, no por `certutil`.** El binario de `libnss3-tools` no
  está en el runtime del flatpak y `libnss3.so` sí, y el ADR-0004 manda que los tres canales
  ejecuten literalmente el mismo código. Es la misma vía por la que ya entra un `.p12`
  (`docs/research/p12-en-almacen-nss.md`), con sus dos cuidados: `NSS_NoDB_Init` más
  `SECMOD_OpenUserDB` —nunca `NSS_Init` sobre un `configdir`, que no convive con el
  `C_Initialize` de `cryptoki`— y **dentro del turno del token**.
- **`sql:<ruta>` explícito, siempre.** Sobre el backend DBM la operación **falla en
  silencio**: se devuelve éxito y la confianza se pierde. No se hereda el heurístico del
  `pkcs11.txt` de AutoFirma, anterior a NSS 3.35.
- **Los bits son `C,,` y solo `C,,`**: CA de confianza para TLS y para nada más. El
  `TCP,TCP,TCP` que pone AutoFirma en `~/.pki/nssdb` le regala a la CA local una confianza
  —correo, certificados de cliente— que nadie le ha pedido.
- **Lo que se escribe y lo que se lee no son el mismo número.** La confianza no vive como
  máscara de bits sino como un `CKA_TRUST_SERVER_AUTH` del softoken, y al leerla de vuelta un
  `CKT_NSS_TRUSTED_DELEGATOR` viene siempre con `CERTDB_NS_TRUSTED_CA` puesto encima. La
  comprobación mira los dos bits que importan y **no** compara el número entero.
- **Contraseña maestra de Firefox.** Los bits de confianza son atributos autenticados: sin
  sesión iniciada, el certificado puede quedar añadido con confianza `,,` —éxito parcial
  silencioso—. rfirma **no inicia sesión y no pide nada**: registra su función de contraseña
  devolviendo `NULL`, para que ninguna ruta de NSS se quede esperando a un diálogo que aquí
  no existe, y **relee los bits** después de escribir. Un perfil que no los tenga puestos se
  cuenta como no instalado y se dice al final.
- **Nunca fiarse del código de salida como única señal**: hay que verificar que la confianza
  quedó puesta, leyendo los bits.
- Un administrador puede neutralizar el almacén NSS de Chrome por política
  (`CAPlatformIntegrationEnabled: false`, Chrome 131+). No es evitable; conviene detectarlo
  antes que fallar sin explicación.
- **El flatpak sí puede registrar `x-scheme-handler/afirma`** exportando su `.desktop`; el
  nuestro simplemente no lo declara todavía. Lo que sostiene el hito v0.4 no es que el
  flatpak no pueda ser la puerta, sino las otras fichas, que se sostienen solas.
