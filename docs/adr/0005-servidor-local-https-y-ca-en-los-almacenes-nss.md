# El servidor local usa HTTPS, y la CA la instala la aplicación en los almacenes NSS de cada persona

Las sedes electrónicas invocan al cliente de firma desde una página servida por HTTPS, así
que `rfirma` levanta un servidor **HTTPS** en `127.0.0.1:63117` con un certificado
autofirmado generado en la primera ejecución. Para que el navegador confíe en él, **la
propia aplicación** registra su CA local en los **almacenes NSS de la persona que la usa**,
en su sesión y en su primer arranque. Ni el instalador, ni `root`, ni un `postinst`, ni un
`pkexec`, ni el almacén de CA del sistema.

El puerto y el protocolo **no son variables de diseño**: `autoscript.js`, que sirve la sede
y no nosotros, tiene cableados `SERVER_HOST = "127.0.0.1"` y
`URL_REQUEST_PREFIX = "wss://" + SERVER_HOST + ":"`. Migrar a `ws://` en texto plano no
está sobre la mesa, y **ninguna CA pública puede emitir para `127.0.0.1`**: el CA/Browser
Forum lo prohíbe desde el 11 de noviembre de 2015 por ser IP reservada, y los existentes
debían revocarse antes del 1 de octubre de 2016. La CA propia es obligatoria.

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

**El permiso entra con el código de la v0.5, no antes.** Un permiso concedido sin ningún
consumidor durante un ciclo entero es el mismo pasivo que se rechaza arriba para la CA, y
agrava el caso el `:create`, que materializa un directorio en el `$HOME` de la persona sin
que nada lo use. En la v0.4 del manifiesto cambia **una sola cosa: el comentario**, que hoy
afirma que rfirma «no tiene ningún motivo para escribir en el perfil» y eso ya es falso.

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

1. **La CA caduca sola.** Vida corta —90 días— renovada por la aplicación mientras siga
   instalada y en uso. Es lo único que funciona cuando la desinstalación es un `apt remove` a
   las tres de la mañana en una máquina con cuatro usuarios: el residuo tiene fecha de
   caducidad garantizada sin depender de que nadie haga nada.
2. **Retirada explícita desde Preferencias**, siempre disponible. Es además el camino de
   reparación cuando un perfil nuevo se queda sin la confianza.
3. **Borrado por huella del certificado, nunca por *nickname*.** Es literalmente el fallo
   medido en el #225: hay CA huérfanas de AutoFirma marcadas `CT,C,C` y válidas hasta 2033
   que ningún desinstalador retira, porque borra por un *nickname* que ya no es el que usa la
   versión actual.

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

## Consequences

- La CA **no se distribuye precompilada**: su clave privada sólo puede vivir en la máquina
  de quien la usa, con permisos restrictivos. Y **no se protege con una constante en el
  código** —`RestoreConfigLinux.java` lleva `KS_PASSWORD = "654321"` en el fuente público de
  AutoFirma—.
- **La CA va restringida**: `keyUsage` reducido a `keyCertSign`+`cRLSign`, y
  `nameConstraints` limitando la emisión a `localhost` y `127.0.0.1`.
- **`-d sql:<ruta>` explícito, siempre.** Sobre el backend DBM la operación **falla en
  silencio**: `certutil -A` devuelve 0 y la confianza se pierde. No se hereda el heurístico
  del `pkcs11.txt` de AutoFirma, anterior a NSS 3.35.
- **Contraseña maestra de Firefox.** Los bits de confianza son atributos autenticados: sin
  login, `-t` deja **el certificado añadido con confianza `,,`** —éxito parcial silencioso—,
  y `certutil` reintenta con `PK11_Authenticate`, que **bloquea leyendo del tty**. Nunca
  lanzarlo con `stdin` conectado a un terminal.
- **Nunca fiarse del código de salida como única señal**: hay que verificar que la confianza
  quedó puesta.
- Un administrador puede neutralizar el almacén NSS de Chrome por política
  (`CAPlatformIntegrationEnabled: false`, Chrome 131+). No es evitable; conviene detectarlo
  antes que fallar sin explicación.
- **El flatpak sí puede registrar `x-scheme-handler/afirma`** exportando su `.desktop`; el
  nuestro simplemente no lo declara todavía. Lo que sostiene el hito v0.4 no es que el
  flatpak no pueda ser la puerta, sino las otras fichas, que se sostienen solas.
