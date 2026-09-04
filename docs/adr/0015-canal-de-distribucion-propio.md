# El canal de distribución es propio: tres repositorios en `rfirma.sgomez.me` y Releases en GitHub

rfirma no va a ninguna tienda. El artefacto llega a la persona por dos sitios nuestros:

- **GitHub Releases** guarda los paquetes de cada versión —el `.flatpak`, el `.deb` y el
  `.rpm`— con su `SHA256SUMS`, la firma de ese fichero y la atestación de procedencia. Es el
  fichero suelto, para quien quiera instalar a mano o sin remoto.
- **`rfirma.sgomez.me` sirve tres repositorios** —**ostree**, **apt** y **dnf**— más un
  `.flatpakref` de un clic. Es el camino recomendado, y el único que da actualizaciones.

## Por qué el repositorio y no sólo el paquete suelto

**Un paquete instalado a mano no se actualiza nunca.** `flatpak update` no sabe de dónde
vino un `.flatpak` suelto, y `apt` y `dnf` no saben de dónde vino un `.deb` o un `.rpm`
descargado. En una aplicación cualquiera eso es una molestia; en esta no, porque el mapa
lleva medidas **tres** maneras de invalidar una firma en silencio —`extraParams`, `TIME` y
zona horaria, las tres con `Digest Mismatch` y sin excepción, unificadas en el sello de
sesión del [ADR-0016](0016-sello-de-sesion-una-sola-invariante.md)—. Si una versión se lleva
alguna por delante, sin canal de actualización la persona se queda ahí y no hay forma de
avisarle.

Ese argumento es el corazón de este ADR y **vale para los tres formatos por igual**: es lo
que justifica pagar tres repositorios en vez de uno.

Aplazarlo tiene precio: flatpak **no migra el origen** de una aplicación ya instalada, así
que quien instale desde el bundle suelto tendrá que desinstalar y reinstalar desde el remoto
el día que exista, y **no se va a enterar solo**. Se le dice en **dos sitios y ninguno más**:
la landing y las notas de la primera Release que traiga repositorios. Nada dentro de la
aplicación.

## El árbol servido es derivado

**La fuente de verdad son las Releases.** `rfirma.sgomez.me` sirve una reconstrucción de
ellas que se puede tirar y rehacer entera: el workflow de publicación descarga **todas** las
Releases de la serie vigente, reconstruye los tres árboles desde cero, los firma, los sube a
un directorio nuevo del anfitrión, y **el último paso es intercambiar un enlace simbólico**.
Un despliegue a medias no llega a ser visible, y la vuelta atrás es reapuntar el enlace.

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
de cada árbol, **todas las versiones de la serie menor vigente**. Las Releases no se borran
nunca.

## La forma de los repositorios

| ruta | qué |
|---|---|
| `/` | la landing |
| `/rfirma.asc` | la clave pública: el `Signed-By` de apt y el `gpgkey` de dnf |
| `/flatpak/` | el repositorio ostree |
| `/rfirma.flatpakref` | instalación de un clic |
| `/apt/` | con `dists/stable/main/binary-amd64/` |
| `/rpm/` | con `repodata/` |

Estas rutas van dentro del `.flatpakref` y de las órdenes de alta publicadas, así que se
fijan aquí.

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

La imagen es **`nginx:alpine` con la configuración y la landing, y nada más**, construida
por Coolify desde `main` con el `Dockerfile` de `packaging/repo/`. **Los datos no van dentro
de la imagen**: cada publicación produciría una capa nueva con la historia entera repetida.
Con los datos fuera, el CI de publicación no toca Docker en ningún momento.

**La landing es un entregable propio y sale antes que la tubería**, porque hoy el dominio
resuelve al VPS y no sirve nada. Es un `index.html` escrito a mano, sin generador y sin paso
de construcción: qué es rfirma, que está en alfa, las órdenes de alta de los tres canales, la
huella GPG y el párrafo de migración. Mientras no exista la v0.4, la sección de instalación
dice que todavía no hay versión publicada, en vez de esconderse.

## La tubería: tres ficheros, cada uno con un motivo para cambiar

| fichero | disparador | permisos | qué hace |
|---|---|---|---|
| `build.yml` | `workflow_call` | `contents: read`, **sin secretos** | la matriz, la guardia de versión, el carril lento, `just check-glibc`, artefactos y digests como salidas |
| `release.yml` | `push: tags v*` | `environment: release` | descarga los artefactos, firma, atesta la procedencia y crea la Release **en borrador** con el `pdf-puerta-manual` adjunto |
| `publish.yml` | `release published`, si no es prerelease | `environment: release` | reconstruye los tres repositorios y los despliega |

Y **cuatro invariantes**, que son justo lo que un agente futuro colapsaría por comodidad:

1. **`build.yml` no ve un secreto jamás.** Es lo que permite reutilizarlo desde un PR
   etiquetado sin que la reutilización sea un camino hacia la clave de firma.
2. **La Release nace en borrador; publicarla es un acto humano.** Una etiqueta no publica
   nada por sí sola: el despliegue cuelga de `release published`. Es el mismo gesto que ya
   cierra la puerta manual del PDF, y por eso el `pdf-puerta-manual` se adjunta al borrador:
   la puerta deja de ser una convención en un comentario y pasa a ser un artefacto delante
   de quien publica. De aquí sale que **empujar una etiqueta `v*` esté restringido por una
   regla del repositorio**: si no, la puerta la abre cualquiera con permiso de escritura y
   el revisor del `environment: release` se rodea empujando una etiqueta.
3. **El suelo de glibc lo hace verdad una puerta, no el entorno de construcción.** Se promete
   `GLIBC_2.34` y lo comprueba `just check-glibc` sobre lo que se va a publicar. La receta es
   `just` y no un paso `run:`, porque una puerta que no puedes reproducir en tu equipo es una
   puerta que un día se salta con `continue-on-error`.
4. **Una GPG para lo verificable por humanos, una minisign para la máquina.** Una GPG
   «rfirma signing» con una sola huella publicada firma `SHA256SUMS.asc`, ostree, apt y dnf:
   dos claves GPG para el mismo enunciado —«esto lo hizo rfirma»— son dos raíces de confianza
   para una cosa, y eso es peor seguridad, no mejor. La minisign del *updater* sí es otro
   animal: la consume una máquina sin persona delante, y su compromiso significa instalación
   silenciosa de código.

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

**Etiquetas `v*-rc.N`** producen una Release marcada como prerelease y **no llegan a ningún
repositorio**. No es un *nightly* por la puerta de atrás —es a mano y con etiqueta
explícita—: es cómo se ensaya la tubería sin publicar una versión de verdad.

**Las acciones se fijan por SHA en todo el repositorio**, `ci.yml` incluido, con el
comentario de etiqueta al lado, más `dependabot.yml` para `github-actions` **mensual y
agrupado**. Lo que se compra no es inmunidad, sino una revisión humana en medio en vez de la
ejecución silenciosa; por eso mensual, que un flujo de PRs que nadie mira es peor que no
tenerlas.

## El runtime sigue viniendo de Flathub

El bundle no lleva `org.gnome.Platform//50` dentro, así que sin el remoto de Flathub añadido
la instalación falla con «runtime not found». **Consumir un runtime no es publicar en la
tienda** y no lo condiciona nada de lo anterior. Se documenta como requisito de instalación
—una línea de `flatpak remote-add --if-not-exists flathub …`, que la mayoría de escritorios
ya traen puesta— y no se resuelve por otro lado: servir el runtime desde nuestro repositorio
son cientos de megas para ahorrar un comando.

## Considered Options

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
- **Copiar `nightly.yml` de tabularis.** El [#222](https://github.com/sgomez/rfirma/issues/222)
  dejó a tabularis medido como **contraejemplo, no modelo**. Lo que sobrevive de él es el
  hecho desnudo: un remoto propio es un canal normal, no una rareza.

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
- Se crea **`SECURITY.md`**, con las claves de larga vida (cuáles hay, qué firma cada una,
  dónde vive la pública, caducidad y revocación) y la vía de reporte, que es el **private
  vulnerability reporting de GitHub** y no un correo: un correo personal en un fichero
  público es un dato personal publicado para siempre y sin acuse de recibo.
- Generar la GPG —maestra fuera de línea, subclave exportada para el CI, huella publicada— y
  aprovisionar los **tres secretos** (subclave GPG, clave SSH con orden forzada, token de
  Coolify que sólo redespliega esa aplicación) es **trabajo humano y bloqueante**.
