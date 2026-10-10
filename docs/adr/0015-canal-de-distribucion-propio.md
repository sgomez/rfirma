# El canal de distribución es propio: `rfirma.sgomez.me` y Releases en GitHub

rfirma no va a ninguna tienda. El artefacto llega a la persona por dos sitios nuestros:

- **GitHub Releases** guarda los paquetes de cada versión —el `.flatpak`, el `.deb`, el
  `.rpm` y el instalador de Windows— con su `SHA256SUMS`, la firma de ese fichero y la
  atestación de procedencia. Es el fichero suelto, para quien quiera instalar a mano o sin remoto.
- **`rfirma.sgomez.me` sirve tres repositorios** —**ostree**, **apt** y **dnf**— más un
  `.flatpakref` de un clic, y **el canal de Windows** en `/windows/`: el instalador de cada
  versión con su firma minisign y el `latest.json` que consulta el *updater*. Es el camino
  recomendado, y el único que da actualizaciones.

## Por qué el repositorio y no sólo el paquete suelto

**Un paquete instalado a mano no se actualiza nunca.** `flatpak update` no sabe de dónde
vino un `.flatpak` suelto, y `apt` y `dnf` no saben de dónde vino un `.deb` o un `.rpm`
descargado. En una aplicación cualquiera eso es una molestia; en esta no, porque el mapa
lleva medidas **tres** maneras de invalidar una firma en silencio —`extraParams`, `TIME` y
zona horaria, las tres con `Digest Mismatch` y sin excepción, unificadas en el sello de
sesión del [ADR-0016](0016-sello-de-sesion-una-sola-invariante.md)—. Si una versión se lleva
alguna por delante, sin canal de actualización la persona se queda ahí y no hay forma de
avisarle.

Ese argumento es el corazón de este ADR y **vale para los cuatro canales por igual**: es lo
que justifica pagar tres repositorios en vez de uno, y un canal de Windows servido desde el
mismo sitio. En Windows no hay gestor de paquetes que haga de repositorio: lo hace el
*updater* de Tauri contra `/windows/latest.json`.

Aplazarlo tiene precio: flatpak **no migra el origen** de una aplicación ya instalada, así
que quien instale desde el bundle suelto tendrá que desinstalar y reinstalar desde el remoto
el día que exista, y **no se va a enterar solo**. Se le dice en **dos sitios y ninguno más**:
la landing y las notas de la primera Release que traiga repositorios. Nada dentro de la
aplicación.

## El árbol servido es derivado

**La fuente de verdad son las Releases.** `rfirma.sgomez.me` sirve una reconstrucción de
ellas que se puede tirar y rehacer entera: el workflow de publicación descarga **todas** las
Releases de la serie vigente, reconstruye desde cero los tres repositorios y el directorio de
Windows, los firma, los sube a un directorio nuevo del anfitrión, y **el último paso es
intercambiar un enlace simbólico**. Un despliegue a medias no llega a ser visible, y la vuelta atrás es reapuntar el enlace.

Se descarta mutar el repositorio en el servidor (`build-export` sobre el existente,
`createrepo_c --update`), que es lo convencional. El motivo no es la elegancia: **esto va a
correr pocas veces al año y nadie va a recordar cómo estaba el volumen**. Una publicación no
idempotente convierte cualquier fallo en arqueología sobre un servidor, y obliga además a un
cerrojo contra publicaciones concurrentes y a un backup de algo que se puede regenerar.

Para apt y dnf es trivial. Para ostree lo hace posible `flatpak build-import-bundle`, y su
viabilidad **está medida**: importar el mismo `.flatpak` en dos repositorios vacíos da el
mismo commit, con el mismo `ContentChecksum` y el mismo `xa.metadata`, así que **nadie se
redescarga la aplicación entera** por reconstruir. Tres cabos que hay que atar: hace falta
`ostree init --mode=archive` delante porque `build-import-bundle` no crea el repositorio; la
historia **se trunca a lo que importes**, así que hay que importar todos los bundles que se
quieran servir, en orden; y la firma **no viaja dentro del bundle**, pero re-firmar tras
importar (`flatpak build-sign`) **no altera el checksum del commit**, así que la receta
segura es importar y firmar siempre.

**Retención**, en dos ejes que conviene no confundir: en el anfitrión, **el árbol vigente y
el anterior** —el anterior existe para que la vuelta atrás sea reapuntar el enlace—; dentro
de cada árbol, **todas las versiones de la serie menor vigente**; en `/windows/` también, pero
`latest.json` anuncia solo la última que no es candidata. Las Releases no se borran
nunca.

## La forma de los repositorios

| ruta | qué |
|---|---|
| `/` | la landing |
| `/rfirma.asc` | la clave pública: el `Signed-By` de apt y el `gpgkey` de dnf |
| `/flatpak/` | el repositorio ostree |
| `/rfirma.flatpakref` | instalación de un clic, ya no anunciada |
| `/rfirma.flatpakrepo` | el alta del remoto de rFirma |
| `/apt/` | con `dists/stable/main/binary-amd64/` |
| `/rpm/` | con `repodata/` |
| `/windows/` | el `-setup.exe` y su `.sig` de cada versión, y `latest.json` |

Estas rutas van dentro del `.flatpakref` y del `.flatpakrepo`, de las órdenes de alta publicadas y del *endpoint*
del *updater* embebido en cada instalación de Windows, así que se fijan aquí.

**apt con una sola suite**, no repositorio plano: el plano es más barato y **no admite
`Suites:`/`Components:` en un fichero `.sources` deb822**, que es el formato obligado para
que la clave vaya en `Signed-By` sin `apt-key` (retirado). **dnf con URL literal**, sin
`$basearch` ni `$releasever`: meterlas sería prometer arquitecturas que no se construyen.
Un solo repositorio de cada, no uno por distribución: el paquete vale igual en Debian y en
Ubuntu porque sus dependencias son **débiles**
([ADR-0013](0013-estructura-del-repositorio-y-cadena-de-compilacion.md)). El **`.Debug` no
se publica**.

El transporte es **`rsync` sobre SSH** contra un usuario dedicado del VPS cuya clave está
atada a una **orden forzada** en `authorized_keys` (`command="rrsync /srv/rfirma-repo"`,
sin pty ni reenvío de puertos): no da consola, sólo sabe escribir en el directorio que ya
sirve ficheros públicos. El destino es un **montaje de directorio del anfitrión**
(`/srv/rfirma-repo`), **no** un volumen con nombre de Docker, que sólo escribe `root`.

La imagen es **`caddy:alpine` con la configuración y la landing, y nada más**, construida
por Coolify desde `main` con el `Dockerfile` de `packaging/repo/`. **Los datos no van dentro
de la imagen**: cada publicación produciría una capa nueva con la historia entera repetida.
Con los datos fuera, el CI de publicación no toca Docker en ningún momento.

**La landing es un entregable propio y sale antes que la tubería**, porque hoy el dominio
resuelve al VPS y no sirve nada. Dice qué es rFirma, que está en alfa, las órdenes de alta de
los tres canales, la huella GPG y el párrafo de migración; mientras no exista la v0.4, la
sección de instalación dice que todavía no hay versión publicada, en vez de esconderse.

**Se construye con Astro, en `packaging/repo/site/`**, y lo que se sirve sigue siendo HTML
estático. El generador entra por dos cosas que un fichero suelto no da: **los cinco idiomas
de la aplicación** —`/` en castellano y `/ca/`, `/eu/`, `/gl/` y `/en/`, con un diccionario
por idioma y una prueba que exige los cinco al 100 %— y **una sección, un componente**, que es
lo que permite tocar la comparativa sin releer la página entera. El precio es un paso de
construcción: la imagen se arma en dos etapas —`node` ejecuta `astro build`, `caddy:alpine`
se queda con el `dist/`— y su contexto pasa a ser la raíz del repositorio, porque la página
consume el sistema de diseño de `rfirma-app/src/design-system/bundle/` en vez de repetir sus
tokens.

## La tubería: tres ficheros, cada uno con un motivo para cambiar

| fichero | disparador | permisos | qué hace |
| `build.yml` | `workflow_call` | `contents: read`, **sin secretos** | compilación única de `librfirma_crypto.so` distribuida a jobs paralelos de empaquetado y pruebas de grada C para el objetivo único `x86_64` (ID-147) —garantizando los mismos bytes en los tres canales (ADR-0004)—, guardia de versión, `just check-glibc`, artefactos y digests como salidas |
| `release.yml` | `push: tags v*` | `environment: release`, solo etiquetas `v*` | descarga los artefactos, firma, atesta la procedencia y crea la Release **en borrador** con el `pdf-puerta-manual` adjunto |
| `publish.yml` | `release published`, si no es prerelease | `environment: release`, solo etiquetas `v*` | baja y verifica la serie, reconstruye los tres repositorios y el directorio de Windows, y los despliega |

Y **cuatro invariantes**, que son justo lo que un agente futuro colapsaría por comodidad:

1. **`build.yml` no ve un secreto jamás**, ni él ni las acciones locales que alcanza. Es lo
   que permite reutilizarlo desde un PR etiquetado sin que la reutilización sea un camino
   hacia la clave de firma.
2. **La Release nace en borrador; publicarla es un acto humano.** Una etiqueta no publica
   nada por sí sola: el despliegue cuelga de `release published`. Es el mismo gesto que ya
   cierra la puerta manual del PDF, y por eso el `pdf-puerta-manual` se adjunta al borrador:
   la puerta deja de ser una convención en un comentario y pasa a ser un artefacto delante
   de quien publica. De aquí sale que **empujar una etiqueta `v*` esté restringido por una
   regla del repositorio**, y que esa regla sea la cerradura de los secretos: el
   `environment: release` **solo admite etiquetas `v*` y no tiene revisor humano**.
   `release.yml` y `publish.yml` corren sobre la etiqueta y entran sin pedir aprobación; un
   workflow en una rama o en un PR no entra. Una release son dos gestos humanos: empujar la
   etiqueta y publicar el borrador después de mirar el PDF.
3. **El suelo de glibc lo hace verdad una puerta, no el entorno de construcción.** Se promete
   `GLIBC_2.34` y lo comprueba `just check-glibc` sobre lo que se va a publicar. La receta es
   `just` y no un paso `run:`, porque una puerta que no puedes reproducir en tu equipo es una
   puerta que un día se salta con `continue-on-error`.
4. **Una GPG para lo verificable por humanos, una minisign para la máquina.** Una GPG
   «rfirma signing» con una sola huella publicada firma `SHA256SUMS.asc`, ostree, apt y dnf:
   dos claves GPG para el mismo enunciado —«esto lo hizo rfirma»— son dos raíces de confianza
   para una cosa, y eso es peor seguridad, no mejor. La minisign del *updater* sí es otro
   animal: firma el `-setup.exe` de cada versión y la consume una instalación de Windows sin
   persona delante, así que su compromiso significa instalación silenciosa de código. La
   pública va embebida en la configuración de Windows y la privada, con su contraseña, solo
   en los secretos del `environment: release`. `release.yml` comprueba cada firma contra la
   pública embebida en la **última etiqueta estable** —la que aceptan las instalaciones que ya
   existen—, o contra la actual si todavía no hay ninguna estable con *updater*; no contra la
   que embebe la versión que se firma, porque en la versión puente de una rotación esa ya es
   la nueva. Cómo se crea y se rota lo dice `packaging/repo/README.md`, en «La clave de
   actualizaciones de Windows».

**Al CI se le da sólo la subclave de firma** (`gpg --export-secret-subkeys`), no la maestra.
El CI puede firmar; no puede certificar, ni crear subclaves, ni tocar la identidad. Si se
filtra, se revoca la subclave, se emite otra bajo la misma maestra y **la huella que la gente
añadió a su `Signed-By` sigue valiendo**. Con la maestra en el CI, una filtración significa
«todo el mundo tiene que volver a añadir el repositorio a mano», que en un cliente de firma
electrónica es el peor final posible de un incidente. La maestra vive fuera de línea con su
certificado de revocación, y va al `SECURITY.md`.

**Las firmas no son la misma firma en cada repositorio.** En apt la convención es firmar
**sólo el índice** (`InRelease`) y la integridad del `.deb` cuelga de su hash dentro de él.
En dnf hay dos interruptores independientes: `repo_gpgcheck=1` verifica `repomd.xml.asc` y
`gpgcheck=1` verifica **la firma dentro de cada `.rpm`**. Se firma **cada `.rpm`**, con los
dos a 1; los `.deb` no se firman individualmente, porque no hay consumidor convencional de
esa firma y el `.deb` suelto se verifica por el `SHA256SUMS.asc`. Un `.rpm` suelto, en
cambio, **sólo es verificable si lleva la firma dentro**.

De ahí una **costura de tubería que es orden obligatoria**, porque firmar *modifica el
fichero*: en `release.yml`, **firmar el `.rpm` → calcular `SHA256SUMS` → atestar la
procedencia → adjuntar el asset**. Si se firmara después, el `.rpm` del repositorio y el de
la Release dejarían de ser los mismos bytes, y se rompería la invariante de que los tres
canales llevan lo mismo
([ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md)).

**Qué paquetes hay y cuáles se firman después lo dice un manifiesto**, `paquetes.json`, que
una receta de `just` escribe junto al `SHA256SUMS` en el artefacto `paquetes`: la recogida, la
atestación, la construcción del árbol y la verificación de paquetes lo leen en vez de repetir
una lista de extensiones, y una plataforma o un formato nuevos son una fila más.

**Un directorio de paquetes se verifica con una sola receta, `just verify-packages`**, y cada
llamador usa uno de sus tres modos:

- **Sin opciones, la puerta del contenido**, en `build.yml`: cada paquete del manifiesto lleva
  exactamente una biblioteca nativa y ninguna de AWT
  ([ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md)).
- **`--against`, contra los resúmenes de la construcción**, en `release.yml` antes de firmar:
  lo que se firma es lo que se construyó. Falla con un fichero de más, con uno que falte y con
  un resumen que no case; solo puede cambiar lo que el manifiesto marca como firmable, porque
  firmar un `.rpm` lo modifica.
- **`--signed`, la Release firmada**, en la descarga de la serie, versión a versión: la firma
  de `SHA256SUMS.asc`, `sha256sum --check --strict`, y que los nombres de la Release sean
  exactamente los del `SHA256SUMS` más el propio `SHA256SUMS`, su firma y el PDF de la puerta
  manual. Un asset subido a mano a una Release publicada no está en el fichero firmado y hace
  fallar la publicación.

`publish.yml` no tiene un job de verificación aparte: la descarga de la serie ya verifica cada
versión antes de que se construya el árbol, y ese orden dentro del job es el que vigila
`check-workflows.sh`.

**Etiquetas `v*-rc.N`** producen una Release marcada como prerelease y **no llegan a ningún
repositorio**. No es un *nightly* por la puerta de atrás —es a mano y con etiqueta
explícita—: es cómo se ensaya la tubería sin publicar una versión de verdad. Tampoco llegan a
`/windows/latest.json`: el *updater* nunca ofrece una candidata.

**La etiqueta `preview` de una PR** construye los paquetes de su head con el mismo `build.yml`
de la entrega, sin secretos y con todas sus puertas, y los ofrece como artefactos
`rfirma-preview-<plataforma>` (uno por plataforma del manifiesto de paquetes, con su
`SHA256SUMS`) que caducan a los 14 días. Se diferencia de la entrega en que no hay etiqueta
`v*`, ni Release, ni repositorio: solo artefactos. El workflow Preview declara únicamente
`contents: read`, y `check-workflows.sh` vigila que ningún job de `preview.yml` mencione un secreto; solo
se dispara al poner la etiqueta: un push no reconstruye, y para rehacer el preview se quita la etiqueta y se
vuelve a poner, lo que cancela la construcción anterior.

**Las acciones se fijan por SHA en todo el repositorio**, `ci.yml` incluido, con el
comentario de etiqueta al lado, más `dependabot.yml` para `github-actions` **mensual y
agrupado**, con una espera (`cooldown`) de siete días para no proponer una versión recién
publicada. Lo que se compra no es inmunidad, sino una revisión humana en medio en vez de la
ejecución silenciosa; por eso mensual, que un flujo de PRs que nadie mira es peor que no
tenerlas.

**Los jobs sin secretos se preparan con una sola acción local, `setup-runner`**, que declara
lo que el job necesita —la cadena de Rust por perfil, Node, GraalVM, un perfil cerrado de
paquetes de apt, las herramientas fijadas y el banco de conformidad— y lee `versions.env` y
las diferencias de Windows dentro de ella. `check-workflows.sh` extiende la invariante 1 a esa
acción y a las que alcanza. Que la acción guarde cachés es la entrada `save-cache`, siempre
explícita y, en un workflow invocable, nunca derivada de la ref: bajo `workflow_call` la ref es la del llamador, y una PR
o una etiqueta que guardan cachés desbordan la cuota de 10 GB y desalojan las de `main`, así
que `build.yml` solo las lee. El checkout, el `autocrlf` de Windows y los artefactos quedan en
el job; los jobs con secretos no la usan.

**La grada C es una sola acción composite, `native-tier`, sobre `setup-runner`**, que usan el
CI y la entrega. Cada llamador elige con `library` cómo obtiene la biblioteca nativa
([ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md)): el CI la compila o la saca de la
caché, la entrega la descarga antes. La caché de la biblioteca, la del validador de
referencia y la subida del PDF del validador viven dentro de la acción, porque `just native`
necesita el GraalVM que instala su `setup-runner`; `save-cache` sigue siendo explícito y la
entrega solo lee.

## El runtime sigue viniendo de Flathub

El bundle no lleva `org.gnome.Platform//50` dentro, así que sin el remoto de Flathub añadido
la instalación falla con «runtime not found». **Consumir un runtime no es publicar en la
tienda** y no lo condiciona nada de lo anterior. Se documenta como requisito de instalación
—una línea de `flatpak remote-add --if-not-exists flathub …`, que la mayoría de escritorios
ya traen puesta— y no se resuelve por otro lado: servir el runtime desde nuestro repositorio
son cientos de megas para ahorrar un comando.

## La comprobación de versión no tiene caché por tiempo

El Diagnóstico pregunta siempre por la última publicación cuando mide la señal de versión
—a la API de Releases de GitHub en Linux, a `/windows/latest.json` en Windows—, sin ventana
de validez: **no hay un «hace menos de 24 horas, no preguntes»**. El escritorio es una
aplicación que se abre cada varios días, no un proceso en segundo plano que sondee sin que
se le pida: una caché por tiempo ahorraría peticiones solo en la reapertura
seguida del mismo día, y a lo que se entra a propósito —abrir el Diagnóstico, pulsar «Volver a
comprobar»— se le responde al momento, con la lectura de la red que toca. La última respuesta
sí se recuerda entre sesiones, pero solo como último recurso si esa petición falla: nunca como
sustituto de preguntar.

Por eso la lectura inicial del Diagnóstico **no sale a la red**: trae la señal de versión en
«Comprobando», y es la medición aparte —la misma que usa «Volver a comprobar»— la que pregunta.
Repetir la pregunta dentro de la propia lectura de estado sería preguntar dos veces por la
misma cosa en la misma apertura del panel.

## Considered Options

- **Un preámbulo de pasos repetido en cada job**, que es como nació la tubería: cada job
  escribía su toolchain, su caché, su lista de apt y sus arreglos de Windows. Se descarta
  porque las copias divergían —la misma lista de apt con paquetes distintos en cada job, el
  arreglo del `link.exe` en tres sitios— y porque la invariante 1 solo se podía vigilar
  fichero a fichero.

- **Un workflow reutilizable para la grada C**, que sería el sitio natural de una definición
  única. Se descarta para no anidar tres niveles (`build.yml` invocable desde la entrega,
  y este dentro), decisión del [#1247](https://github.com/sgomez/rfirma/issues/1247); una acción
  composite se comparte sin ese anidamiento.

- **Una guarda de workflows que también comprueba la forma del YAML** (`save-if`, disparadores,
  permisos, argumentos literales de rsync y del árbol), que es como nació. Se descarta porque un
  refactor sin efecto daba un falso rojo: lo que esas líneas protegen ya lo prueban el
  `justfile` y los tests de `packaging/repo/`, y la guarda solo vigila las invariantes. La de
  `save-if` se sustituye por su invariante: la cache de Rust solo se abre desde la acción de
  preparación, que es donde `save-cache` decide si se escribe.

- **Flathub**, que el [#22](https://github.com/sgomez/rfirma/issues/22) dio por hecho sin
  decidirlo. Queda fuera, y **no cerrado para siempre**: volver es un esfuerzo nuevo
  —vendorizar el árbol Maven, y lo que sus reglas digan cuando toque—, no la continuación de
  este.
- **Colgarse de un repositorio ostree ajeno** (Flatpark y similares) ahorraría la clave GPG y
  el despliegue. Se descarta por lo mismo por lo que se descarta la tienda: **mete a un
  tercero entre el usuario y una aplicación de firma electrónica**. El dominio ya existe, el
  `app-id` ya es `me.sgomez.rfirma` por DNS inverso de `rfirma.sgomez.me`, y servir un
  directorio estático es lo más barato de toda esta decisión.
- **Un remoto sin firmar** (`--no-gpg-verify`). No es defendible en una aplicación de firma
  electrónica: sin ficha en un centro de software, esa firma es la única cadena de confianza
  que el usuario tiene.
- **Que el contenedor tire de las Releases por su cuenta**, con el CI llamando sólo al
  extremo de despliegue de Coolify y sin ninguna credencial del VPS. Falla por dónde vive la
  firma: o la clave GPG baja al servidor —peor secreto en peor sitio—, o el CI publica además
  los índices ya firmados como assets y el contenedor queda de espejo tonto. Lo segundo
  funciona y es mucha maquinaria para lo que resuelve.
- **Seguir con la landing a mano**, un `index.html` sin generador ni paso de construcción,
  como fue hasta que hubo que publicarla en cinco idiomas. Cinco copias del mismo fichero se
  desincronizan en el primer cambio de una frase, y no hay forma de que una prueba diga que a
  una le falta un párrafo. Con el diccionario por idioma, eso es exactamente lo que se
  comprueba.
- **Un revisor humano en el `environment: release`** como cerradura de los secretos, con el
  entorno abierto a cualquier rama. Cada job que entra pide su aprobación —y `publish.yml`
  tenía entonces dos encadenados, que no se agrupan—, así que una release eran tres clics más el de
  publicar, todos de la misma persona y ninguno con una decisión que no se hubiera tomado
  ya al empujar la etiqueta o al publicar. Con un solo administrador, quién puede crear la
  etiqueta ya es quién puede llegar a los secretos.
- **Un *nightly* de `main`**, como el `nightly.yml` de tabularis: un cron que construye el
  último commit con el CI en verde y lo publica como prerelease. Se descarta por cuatro
  razones, y ninguna es de gusto:
  - **Rompe la cerradura de los secretos.** Firmado, obliga a abrir el
    `environment: release` a un cron sobre `main` o a una segunda clave; sin firmar, no puede
    entrar en los repositorios, por lo mismo que se descarta el remoto sin firmar.
  - **Rompe la versión única.** tabularis reescribe la versión en cada construcción y lleva
    un contador semver propio, porque WiX y NSIS rechazan prereleases no numéricas; aquí
    `Cargo.toml` es la fuente y `just check-version` la vigila.
  - **No tiene a quién servir.** tabularis alimenta con él un canal de su actualizador y un
    paquete AUR aparte; el *updater* de rFirma en Windows solo anuncia versiones estables, y a
    quien firma un trámite no se le ofrece una construcción diaria.
  - **Lo que resolvería ya tiene sitio.** Probar una rama lo cubre `build.yml` invocado desde
    un PR etiquetado, que la invariante 1 permite sin secretos; ensayar la tubería, las
    etiquetas `-rc.N`.
- **Un job de verificación en `publish.yml`**, delante del que publica, que bajaba la Release
  recién publicada, comprobaba su firma y sus resúmenes y le pasaba otra vez la puerta del
  contenido. Repetía lo que la descarga de la serie hace sobre esa misma versión un job después,
  y no veía lo que sí importa: un asset subido a mano que no está en el `SHA256SUMS` pasaba las
  dos comprobaciones, porque `sha256sum --check` solo mira los ficheros que el resumen nombra.
  La puerta del contenido ya la pasaron esos mismos bytes en la construcción, y lo que llega a
  la Release está atado a ellos por la firma.
- **Comparar los resúmenes con un script propio (`check-digests.sh`)** y la firma con pasos
  sueltos en cada workflow: tres puertas sobre el mismo directorio de paquetes, escritas cada
  una a su manera, y ninguna comprobaba el conjunto de nombres de una Release firmada.
- **Una copia versionada aparte de la pública minisign de firma**, junto al script que firma,
  para comprobar la firma contra ella en vez de contra la embebida. Decía lo mismo que la
  pública embebida de la última estable, que el historial de etiquetas ya guarda, y rotar la
  clave obligaba a un paso manual más —copiarla por PR entre publicar la versión puente y subir
  la privada nueva— que, olvidado, paraba la release siguiente.
- **Una caché de 24 horas para la comprobación de versión**, la lectura obvia cuando se
  guarda la última respuesta. Se descarta porque no resuelve el problema que resolvería una
  caché: el escritorio no se reabre varias veces en un mismo día con la frecuencia que haría
  falta para que ahorrar esas peticiones importe, y en cambio sí complica el caso que sí
  importa —«Volver a comprobar» debe responder con una lectura fresca, no con la de hace
  unas horas—, obligando a que ese botón sepa saltarse su propia caché.

## Consequences

- El `type: dir` del manifiesto **no es un problema**: lo prohibía el linter de Flathub, y
  nada más. `flatpak-builder` lo construye igual.
- Construir sin red deja de ser una obligación externa y pasa a ser preferencia nuestra. El
  [ADR-0013](0013-estructura-del-repositorio-y-cadena-de-compilacion.md) ya la había adoptado
  por su cuenta —fuentes generadas y versionadas, el CI comprueba que están al día— y se
  mantiene por esa razón, no por la de Flathub.
- El [#37](https://github.com/sgomez/rfirma/issues/37) preguntaba cómo entran los `.so` en
  una construcción apta para Flathub. Con la tienda fuera, **la pregunta no llega a
  importar**: la medición se conserva por si algún día se retoma.
- **El metainfo lo necesita este ADR, no los paquetes nativos.** Hasta que existe el
  repositorio no hay tienda que lo lea; cuando existe, hay que corregir su `<launchable>`
  para la identidad de escritorio nativa, que diverge de la del flatpak (ADR-0013).
- **La política de portales no vive aquí.** Este ADR decide *dónde se sirve* el paquete; qué
  entra y sale del sandbox lo fija el
  [ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md), y qué se declara sobre los
  almacenes NSS, el [ADR-0005](0005-servidor-local-https-y-ca-en-los-almacenes-nss.md).
- **`option_env!("PACKAGE_MANAGER_SRC")`, el truco de tabularis, es imposible aquí**: los
  tres canales llevan los mismos bytes de una sola construcción, así que el `.deb` suelto de
  Releases y el del repositorio apt son *el mismo fichero*. La pregunta que sí tiene respuesta
  en tiempo de ejecución es otra, y es la que importa: **¿está añadido el repositorio de
  rfirma?** —existe `/etc/apt/sources.list.d/rfirma.sources` o `/etc/yum.repos.d/rfirma.repo`,
  más `FLATPAK_ID` para el flatpak—. Si está, la actualización llega sola; si no, no llega.
- Se crea **`SECURITY.md`**, con las claves de larga vida —la GPG y la minisign: qué firma
  cada una, dónde vive la pública, caducidad, revocación o rotación, y qué implica su
  compromiso— y la vía de reporte, que es el **private vulnerability reporting de GitHub** y
  no un correo: un correo personal en un fichero público es un dato personal publicado para siempre y sin acuse de recibo.
- Generar la GPG —maestra fuera de línea, subclave exportada para el CI, huella publicada— y
  la minisign del *updater*, y aprovisionar los **cinco secretos** —subclave GPG, clave SSH
  con orden forzada, token de Coolify que sólo redespliega esa aplicación, y la privada
  minisign con su contraseña— es **trabajo humano y bloqueante**.

## Enmienda: el flatpak se instala dando de alta el remoto

La vía de instalación del flatpak es dar de alta el remoto de rFirma con `rfirma.flatpakrepo`,
después del de Flathub, de donde sale el runtime: `flatpak remote-add --if-not-exists rfirma
https://rfirma.sgomez.me/rfirma.flatpakrepo` y `flatpak install rfirma me.sgomez.rfirma`. Un
remoto dado de alta así es un remoto normal: GNOME Software lo incluye en la búsqueda y enseña
la ficha que publica su rama `appstream`. El `.flatpakref` y un bundle crean un remoto de
origen con `xa.noenumerate=true`, que la búsqueda no ve, y GNOME Software solo enseña de un
bundle sin instalar el nombre, el resumen y la versión.

El `.flatpak` de la Release sigue ahí, sin firmar y sin origen, pero es la materia prima con la
que `publish.yml` monta el repositorio ostree, no un canal: ni la landing, ni el manual ni el
README lo ofrecen como descarga. El `.flatpakref` se sigue sirviendo para no romper los enlaces
que ya circulan, pero deja de anunciarse.
