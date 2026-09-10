# Changelog

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).
No se reconstruye el histórico anterior a la v0.4.0: este fichero empieza
aquí (ID-153).

Este fichero no se edita a mano: cada issue entrega su nota en
`changelog.d/<issue>.md` y `just changelog-release <version>` reúne los
fragmentos presentes bajo la sección de la versión, en el momento de
publicarla. Ver `changelog.d/README.md`.

## [0.9.0] - 2026-09-10

### Added
- Mecanismo de CHANGELOG por fragmentos: `changelog.d/`, `CHANGELOG.md` desde
  la v0.4.0 y la receta `just changelog-release` (#252).
- rFirma tiene icono propio: un SVG maestro versionado y los ocho tamaños que
  instala el paquete, de 16 a 512, rasterizados desde él y no reescalados al
  construir (#254).
- Un almacén cuyo secreto se teclea en el teclado del lector se rechaza con su
  propio aviso antes de intentar firmar, en vez de pedir el secreto por
  pantalla y fallar contra el token (#257).
- Un certificado en fichero `.p12` se puede instalar en rFirma y firmar con él:
  cada fichero pasa a ser un almacén propio, y sus certificados aparecen en la
  lista junto a los del navegador sin volver a teclear su contraseña (#261).
- Preferencias tiene una sección de certificados en fichero: la lista de los
  `.p12` instalados —titular, DNI, emisor y fecha de caducidad— y sus dos
  gestos, «Añadir…» y «Quitar». Sin casillas por almacén ni diálogos anidados
  (#262).
- Instalar un `.p12` con clave elíptica lo dice en la propia sección y en un
  solo renglón, sin la lista cambiar (#262).
- Preferencias avisa de que el documento firmado cae «Junto al documento
  original» cuando el entorno sabe devolver su ruta real; donde no la sabe,
  el destino se queda en la carpeta con su «Cambiar carpeta…», como antes
  (#264).
- `packaging/verifica-contenido.sh`, la puerta independiente del formato que comprueba la
  invariante del ADR-0012 (un solo `librfirma_crypto.so`, `libawt.so` en ninguna parte) sobre
  el paquete construido, hoy el `.flatpak` (#265).
- `just check-glibc`, la puerta que comprueba el suelo `GLIBC_2.34` de la librería nativa sin
  adoptar un contenedor de construcción (#265).
- Los paquetes `.deb` y `.rpm`, producidos por el *bundler* de Tauri a partir de la misma
  construcción que el flatpak, con la librería nativa en `/usr/lib/rfirma/librfirma_crypto.so`
  y sin ninguna dependencia de PKCS#11 (#266).
  La receta `just bundle` los construye y les pasa `packaging/verifica-contenido.sh`.
- rFirma se abre desde fuera con un documento, `rfirma /ruta/documento.pdf`: la
  ventana completa con ese PDF cargado, el mismo estado en que la deja
  arrastrarlo (#267).
- «Firmar con rFirma» en el menú contextual de KDE sobre un PDF, al primer nivel y sólo con un
  fichero seleccionado. Lo instalan el `.deb` y el `.rpm` en
  `/usr/share/kio/servicemenus/`; el flatpak no lo lleva, porque ahí el gestor de ficheros
  entregaría una ruta del portal (#268).
- El ADR-0018, que explica por qué la firma empieza por un verbo y no por un tipo de fichero:
  rFirma no declara `application/pdf` en ningún lanzador y, a cambio, arrastrar un PDF sobre el
  icono del *dock* no funciona (#268).
- «Firmar con rFirma» en el menú contextual de Nautilus sobre un PDF, al primer nivel y sólo con
  un fichero seleccionado. Es una extensión de `nautilus-python` que el `.deb` y el `.rpm`
  instalan en `/usr/share/nautilus-python/extensions/`; el flatpak no la lleva, por lo mismo que
  no lleva el menú de KDE (#269).
- El paquete de `nautilus-python` viaja como **recomendación**, con el nombre que le da cada
  familia (`python3-nautilus` en el `.deb`, `nautilus-python` en el `.rpm`). Sin él la extensión
  queda inerte y no pasa nada más (#269).
- rFirma comprueba al arrancar si hay una versión nueva publicada, preguntándole
  a GitHub una vez cada 24 horas. Es sólo un aviso: no descarga ni instala nada
  (#270).
- Cuando hay una versión nueva publicada, rFirma lo dice en una franja bajo la
  cabecera, con una acción que lleva a *Acerca de* y una `×` que la descarta.
  Nada de esto interrumpe el trabajo: no hay ningún diálogo, y sin versión
  nueva la franja no ocupa ni un píxel (#271).
- *Acerca de* enseña cómo actualizar: las órdenes de alta del repositorio,
  copiables, para Flatpak, Debian y Ubuntu, y Fedora y openSUSE. No hay botón
  de descarga (#272).
- En Preferencias, sección *Privacidad*, un interruptor para dejar de avisar
  de las versiones nuevas. Siempre visible, sin condición (#272).
- La landing de `rfirma.sgomez.me`: `index.html` escrito a mano y la imagen `nginx` que la
  sirve, construida por Coolify desde `main` (#273).
- Sección «Elige un canal» en la instalación del README (#273).
- El workflow de construcción de la entrega, `.github/workflows/build.yml`: invocable desde otro
  workflow, con permisos de sólo lectura y sin ver ningún secreto, produce el flatpak, el `.deb` y
  el `.rpm` de una sola construcción y les pasa `packaging/verifica-contenido.sh` a los tres antes
  de subirlos con su `SHA256SUMS` (#274).
- Los otros dos workflows de la tubería de entrega: `release.yml`, que ante una etiqueta `v*`
  construye, firma y deja la Release **en borrador** con los tres paquetes, su `SHA256SUMS.asc`,
  la atestación de procedencia y el PDF de la puerta manual adjunto; y `publish.yml`, que sólo
  reacciona a una Release **publicada** que no sea candidata y comprueba la firma, los resúmenes
  y la puerta del contenido antes de que se sirva nada (#275).
- `SECURITY.md`: el aviso privado de GitHub como única vía de reporte, las claves de larga vida
  con lo que firma cada una, cómo verificar un paquete descargado, y la comprobación de versión
  declarada como la primera —y hoy única— conexión saliente de rFirma, con su ajuste para
  apagarla (#275).
- El mecanismo de publicación de `rfirma.sgomez.me`: `publish.yml` reconstruye el árbol servido
  **entero** desde las Releases de la serie vigente, lo sube al anfitrión por `rsync` sobre SSH con
  orden forzada (`rrsync`) y lo pone en servicio **intercambiando un enlace simbólico**. Un
  despliegue a medias no llega a verse, la vuelta atrás es reapuntar el enlace y el volumen se puede
  tirar y rehacer: la fuente de verdad son las Releases. En el anfitrión se quedan el árbol vigente y
  el anterior, y ni uno más (#276).
- Caddy sirve los tres repositorios y la clave pública desde el montaje del anfitrión a través del
  enlace `actual`, con las rutas que fija el ADR-0015 —`/flatpak/`, `/apt/`, `/rpm/`, `/rfirma.asc`,
  `/rfirma.flatpakref`— y sólo esas; la landing sigue viniendo dentro de la imagen (#276).
- `just check-publish`: las pruebas del mecanismo, que levantan el mismo `rrsync` del `authorized_keys`
  del VPS detrás de un `ssh` de mentira. Es la única parte de la tubería que no se puede ensayar con
  una etiqueta `v*-rc.N`, porque el ensayo se detiene justo antes de tocar el anfitrión (#276).
- Los tres repositorios de `rfirma.sgomez.me`, montados encima del mecanismo de publicación:
  el **ostree** de `/flatpak/` con todos los bundles de la serie vigente importados en orden y
  su `rfirma.flatpakref` de un clic; el **apt** de `/apt/` con suite `stable` —no un
  repositorio plano— y su fichero deb822 con `Signed-By`; y el **dnf** de `/rpm/` con la URL
  literal, `gpgcheck` y `repo_gpgcheck` a 1 y su `repodata` firmado (#277).
- Reconstruir el árbol **no obliga a nadie a redescargar**: los bundles reimportados en un
  ostree vacío dan el mismo commit, y `just check-publish` lo comprueba construyendo los tres
  repositorios dos veces y comparando. El repositorio se re-firma en cada reconstrucción,
  porque la firma es metadato desacoplado que no viaja dentro del bundle (#277).
- Los filtros de certificado que manda una sede se aplican con el motor del
  original: `afirma-keystores-filters` entra en el puente por una llamada sin
  estado y sin sello, y la expresión cruza literal, sin reinterpretar (#350).
- Lista blanca de criterios antes de llamar al motor: un criterio que rFirma no
  reconoce se rechaza con `SAF_03` en vez de ignorarse en silencio, que es lo
  que hace el original y deja el listado más ancho de lo que la sede pidió
  (#350).
- El trámite de sede de punta a punta para `selectcert`: la invocación abre el
  canal, la operación se lee de la URL, el listado se acota con el filtro que
  mandó la sede y lo que ella recibe es el certificado en Base64 URL-safe, o
  `CANCEL`, o su código `SAF_` (#352).
- El consentimiento no se salta nunca: `headless` y `mandatoryCertSelection` se
  ignoran los dos, también cuando queda un solo certificado, y en `selectcert`
  ese momento consiente entregar identidad (#352).
- Lo que la sede recibe sale de inmediato, sin esperar a que se cierre ninguna
  ventana: la precisión de lo que pasó se queda en la ventana y el código, en el
  cable (#352).
- El trámite de sede atiende `sign` y `cosign` en PAdES: la operación se lee de
  la URL con su formato, su algoritmo y el documento, la persona consiente con
  el documento delante y lo que la sede recibe es el certificado y la firma en
  Base64 URL-safe, separados por `|` (#353).
- `expPolicy=FirmaAGE` lo expande `ExtraParamsProcessor` de `afirma-core`, por
  una entrada nueva del puente, y no una reimplementación: expandirlo mal sería
  firmar con una política distinta de la declarada (#353).
- Una política que no se puede aplicar al formato pedido llega con nombre propio
  desde el puente y la sede recibe `SAF_23`, en vez de colapsarse en «la firma
  no ha salido» (#353).
- La firma visible que pide una sede se atiende sin visor y sin hacerla esperar:
  con posición y página se firma con recuadro, sin ellas `visibleSignature=optional`
  firma invisible y `want` cancela con `SAF_43`, y un `visibleAppearance=custom`
  del que no vienen datos estampa el aspecto por omisión (#354).
- `signaturePages` admite la gramática entera del puente, índices contados desde
  el final incluidos —`-1` es la última—, y **rechaza `append`**: añadir una
  página en blanco es modificar el documento antes de firmarlo (#354).
- rFirma se registra como manejador del esquema `afirma://`: pulsar un enlace del
  protocolo en el navegador la arranca con la URL entera, en los tres canales (#356).
- rFirma detecta el canal de distribución (`/.flatpak-info`) y, fuera del flatpak,
  puede leer qué aplicaciones dice el escritorio que atienden `afirma://`, sin
  nombrar ninguna en el código; dentro del sandbox no lo intenta, porque la
  respuesta no es de fiar (#357).
- Preferencias trae una sección *Sedes* con un solo control: quién atiende los
  enlaces `afirma://`, elegido entre lo que el escritorio diga que hay
  registrado, con el aviso de que Firefox impone la elección que guarda aparte.
  En el flatpak, donde no hay portal que lo permita, hay una frase fija que
  remite a los ajustes del escritorio en vez de un control que no cumpliría
  (#364).
- Al arrancar, un banner bajo la cabecera pregunta si rFirma debe atender esos
  enlaces —*Sí*, *Ahora no*, *No volver a preguntar*—, y «No volver a
  preguntar» se deshace en Preferencias (#364).
- La ventana de sede enseña los momentos de guardar y de cargar, con el nombre
  que propone la sede y nunca su ruta, y abre sola el diálogo del portal (#500).
- Las cuatro variantes XAdES —Enveloping, Detached, Enveloped y ASiC-S— cruzan
  la frontera nativa: `Format::Xades(_)` va a las entradas XAdES del puente y
  deja de ser un formato sin resolver (#537).
- La sede puede pedir SHA-256, SHA-384 o SHA-512 y rFirma compone el algoritmo
  con la clave del certificado, como hace el original (#544).
- rFirma firma con certificados de curva elíptica: el token puede pedir el
  mecanismo compuesto o el crudo sobre el resumen, y la firma sale en el DER
  que esperan CAdES y XAdES (#545).
- `just token` provisiona también el token de pruebas `rfirma-test-ecc`, con un
  certificado P-256 del kit FNMT (#545).
- `format=CAdES-ASiC-S` cruza el puente por las entradas de CAdES y devuelve el
  contenedor ASiC-S con la firma CAdES dentro, como AutoFirma (#591).
- Una sede que pide `checkSignatures=true` obtiene la validación de las firmas que el documento ya trae antes de pedir consentimiento: si alguna no vale, recibe `SAF_39`; si hace falta confirmar y mandó `headless=true`, recibe `SAF_50`; y si no, el trámite se para en un momento nuevo de confirmación con dos salidas, seguir o cancelar (#595).
- Cuando la validación previa de `checkSignatures` necesita una decisión de la persona, la ventana de sede la pide con las palabras del original —traducidas a los cinco idiomas— y dos salidas: continuar, que repite la validación y sigue al consentimiento, o cancelar, que contesta `CANCEL` a la sede (#596).
- El `dat` de la sede puede ser una URL `http(s)`: rFirma se descarga el
  documento y sigue el trámite con él, como AutoFirma (#612).
- `sign`, `cosign` y `countersign` sin `dat` dejan de ser un `SAF_03`: se pide
  el documento a la persona con las claves de carga de `properties`, y
  cancelar contesta `CANCEL` (#612).
- Catálogos completos en catalán, euskera y gallego: el selector de idioma ofrece ya los cinco (#641).
- La landing de `rfirma.sgomez.me` se reescribe con Astro en `packaging/repo/site/`:
  las diez secciones de la página validada en el lienzo, en los cinco idiomas de la
  aplicación (`es`, `ca`, `eu`, `gl`, `en`) con `/` en castellano y `/ca/`, `/eu/`,
  `/gl/` y `/en/` para el resto, y una prueba que exige los cinco diccionarios al
  100 % (#642).

### Changed
- La versión se cambia en un solo sitio, `tauri.conf.json`, y `just check-version` pone en rojo cualquier divergencia con `package.json`, `Cargo.toml` y el metainfo (#253).
- El README enlaza a `releases/latest/download/…` en vez de llevar el número de versión dentro (#253).
- El metainfo declara `version`, `date` y un enlace al CHANGELOG en vez de copiar sus notas (#253).
- El secreto que desbloquea un almacén deja de ser una cadena y pasa a ser de
  tres clases —sin sesión, tecleado en pantalla, tecleado en el teclado del
  lector—, que la prefirma lee de la ranura y devuelve a la ventana (#257).
- Sin necesidad de sesión no se abre ningún diálogo: la firma sigue directa,
  con el secreto vacío (#258).
- El diálogo del secreto ya no nombra la clase de almacén PKCS#11 ni el
  nombre del token, ni lleva ninguna frase tranquilizadora; la palabra la
  elige el almacén —«PIN» para un módulo, «contraseña» para un fichero— y
  «Tarjeta bloqueada» deja de resolverse ahí dentro (#258).
- Fuera del sandbox, el documento abierto cruza a la ventana con su **ruta
  real**, como la enseña cualquier aplicación de escritorio (#263).
- «Guardar junto al original» lo contesta **el documento**: uno de ruta directa
  ofrece la carpeta en la que está, y uno que entró por el portal responde que
  no hay carpeta original (#263).
- La guarda del ADR-0011 vigila **valor y no texto**: comprueba sobre los
  valores que produce la aplicación que la ruta del portal (`/run/user/*/doc/`)
  no sale a la ventana por ningún campo, en ningún canal (#263).
- `packaging/flatpak/verifica.sh` deja de comprobar la invariante del ADR-0012: ahora vive en
  `packaging/verifica-contenido.sh` (#265).
- Solo hay una rFirma abierta: invocarla otra vez con un documento sustituye el
  que hubiera delante, sin preguntar; con una firma a medias no sustituye nada
  (#267).
- Una invocación `afirma://` **nunca sustituye** el documento que la ventana
  tuviera delante: abre lo suyo, y una firma local a medias tampoco la detiene
  (#352).
- Un segundo `afirma://` con un trámite de sede vivo se rechaza con `SAF_45` por
  su propio socket mientras el primero siga vivo (#352).
- El documento que manda una sede no deja rastro: entra por la puerta que no
  recuerda, su fichero de paso se borra en cuanto el trámite contesta, y la
  postfirma del trámite devuelve los bytes sin escribir fila en Recientes, ni
  colocación del recuadro, ni «último documento» (#353).
- La firma de un trámite de sede vuelve a pasar el filtro de la sede justo antes
  del PIN, y no resuelve el certificado por el camino local (#353).
- Los `extraParams` que declara la sede van **debajo** de los seis ajustes de
  rFirma: la sede decide la política y rFirma el recuadro que la persona vio
  (#353).
- Los dos caminos del recuadro dejan de compartir conversión: cuando el sitio lo
  elige la persona con el ratón se aplica la corrección de rotación, y cuando lo
  elige la sede los `signaturePositionOnPage*` cruzan al puente **crudos**, que
  es lo que hace AutoFirma y contra lo que las sedes ajustaron esos números
  (#354).
- Un algoritmo que el token no ofrece se rechaza antes de pedir el PIN, con el
  `SAF_09` del original y una vista que lo dice en castellano (#544).
- El trámite de sede deja de contestar `CAdES-ASiC-S` como formato sin puente:
  `Format::bridged()` lo admite (#591).
- El almacén de certificados que la sede nombra en `keystore` o `ksb64` ya no se ignora: se obedece el de la familia NSS, que es el que rFirma abre, y cualquier otro se rechaza con `SAF_07` diciéndolo en la ventana (#617).
- Las cadenas hablan de tú en los cinco idiomas, con un registro cercano y directo fijado en el ADR-0009; las confirmaciones de sede dejan el «¿Desea continuar?» heredado de AutoFirma, y el inglés pasa a la voz de la aplicación con contracciones (#641).
- La imagen de `rfirma.sgomez.me` se construye en dos etapas y sirve la salida de
  `astro build` en lugar de un `index.html` suelto; su contexto de construcción pasa a
  ser la raíz del repositorio (#642).

### Removed
- Se retira la fontanería de tarjeta, que nunca se había publicado: el
  cliente PC/SC y el módulo PKCS#11 de OpenSC salen del flatpak, y las siete
  rutas de OpenSC salen de la colección de almacenes. Tarjetas y DNIe no
  están soportados en la v0.4 (#256).
- El contador de intentos restantes del secreto incorrecto: PKCS#11 no lo
  cuenta nunca, así que deja de fingirse (#258).
- `countersign` se contesta con `SAF_04`: en PAdES contrafirmar no existe
  (#353).
- `save` y `signandsave` se rechazan con nombre propio, por seguridad y no por
  coste: que una sede escriba ficheros en el equipo es una decisión (#353).

### Fixed
- El lanzador enseña `rFirma`, no `rfirma` (#253).
- El lanzador enseña el icono al tamaño que pide, en vez de estirar el único
  PNG de 64×64 que se instalaba antes (#254).
- La colección de almacenes de certificados vuelve a encontrar los de Firefox
  147 y Chrome M146, que se habían mudado a rutas XDG
  (`~/.config/mozilla/firefox` + `~/.local/share/mozilla/firefox`, y
  `~/.local/share/pki/nssdb`); el
  manifiesto del flatpak las declara en sólo lectura (#255).
- Un perfil de Firefox con contraseña maestra ya no enseña sus autoridades
  sueltas al listar certificados: la sesión se inicia antes de listar, y se
  retira la vuelta atrás que devolvía el almacén entero sin filtrar cuando
  ninguna clave privada era visible (#259).
- La ventana ya no crece en cada arranque hasta salirse de la pantalla. Se
  recordaba el tamaño con la medida equivocada —la superficie con las sombras
  del CSD dentro, que `set_size` no cuenta—, y eso sumaba 52x99 px cada vez. Se
  retira la memoria de tamaño de ventana: rFirma abre siempre a 1280x720 y quien
  quiera otra cosa maximiza (ADR-0010, enmienda) (#306).
- La imagen nativa firma XAdES Enveloped: le faltaban los canonicalizadores
  `REC-xml-c14n-20010315` y la tabla de funciones XPath, que solo se alcanzan
  por reflexión (#537).
- Las sedes que fuerzan el modo servidor intermedio ya pueden firmar: cuando la
  operación no cabe en la URL, la sede solo manda `fileid`, `rtservlet` y `key`,
  y rFirma recupera con ellos el XML de parámetros de la operación —de donde
  salen el verbo, el documento, `stservlet` e `id`— en vez de rechazar la
  invocación (#601).
- El algoritmo se normaliza con el criterio del original en vez de contra una
  lista cerrada: se atienden los nombres compuestos (`SHA256withRSA`,
  `SHA384withRSA`, `SHA512withRSA` y sus variantes `withECDSA`) en la cabecera
  del lote —tanto en XML heredado como en JSON remoto y local— y las variantes
  con guion (`SHA-256`, `SHA-384`, `SHA-512`), OID y URI de XMLDSig en todas
  las operaciones (#602).
- Los algoritmos no atendidos como SHA1 se rechazan al leer la operación con
  `SAF_03` nombrando `algorithm` (#602).
- El arranque por servidor intermedio conserva el momento del trámite al abrir
  la ventana en vez de quedarse en «Conectando con la sede», permitiendo ver los
  certificados y consentir la firma (#606).
- El cliente de servidor intermedio desacopla las llamadas bloqueantes HTTP del
  runtime de Tokio, evitando el pánico al subir la firma al servlet de guardado (#610).
- La versión mínima de protocolo que la sede declara en `ver` se lee en toda
  operación: si pide más de la que rFirma habla, el trámite sale con `SAF_21`
  antes de firmar nada, y en el camino del servidor intermedio es ella la que
  fija la versión de la operación (#618).

### Security
- Un `.p12` con clave elíptica se rechaza al instalarlo, y no al firmar, porque
  rFirma solo firma con claves RSA (#261).
- Sin red, la comprobación no dice nada: ni aviso, ni error, ni reintento hasta
  el siguiente arranque. Es la única conexión saliente de rFirma, y el flatpak
  gana por ella —y sólo por ella— el permiso de red (#270).
- Todas las acciones de GitHub Actions quedan fijadas por SHA, con `dependabot.yml` revisándolas
  una vez al mes en una sola PR agrupada, y una puerta —`just check-actions`— que impide que
  vuelva a entrar ninguna por etiqueta (#274).
- Una sola clave GPG para todo lo verificable por humanos —Releases, ostree, apt y dnf—, con la
  **subclave de firma en el CI y la maestra fuera de línea**: el CI puede firmar, no puede
  certificar, y una filtración se resuelve revocando la subclave sin que nadie tenga que volver a
  dar de alta el repositorio. La genera `packaging/setup-signing-key.sh`, la importa una acción
  local que comprueba la huella contra la que el repositorio declara, y tres puertas nuevas de
  `just check-actions` impiden que se pierdan el borrador, la guarda de las candidatas o la
  frontera del secreto (#275).
- **La huella de la clave, publicada por dos caminos**: `SECURITY.md` y la portada de
  `rfirma.sgomez.me`. Una huella sólo sirve si quien descarga la clave pública puede
  contrastarla con algo que ya sabía, así que `SECURITY.md` prometía una portada que no la
  enseñaba (#275).
- `packaging/setup-publish-access.sh`, hermano del de la clave de firma: da de alta el acceso
  de publicación al anfitrión —clave ed25519 encerrada con `command="rrsync …",restrict`, el
  secreto del entorno `release` y las tres variables—, y comprueba antes de guardar nada que
  esa clave habla por rsync y **no** da consola (#275).
- La Release **se para si la huella publicada no es la de la clave que firma**. La acción de
  importación ya ataba el secreto a `vars.GPG_FINGERPRINT`, pero `SECURITY.md` y la portada
  eran texto suelto: una huella equivocada ahí no rompía nada y dejaba verificando contra nada
  a quien se fiara de ella (#275).
- Dos puertas nuevas de `just check-actions`: el `rsync` de la publicación tiene que ir por el guion
  probado y no suelto dentro del YAML, y **ningún workflow puede tocar Docker ni un registro de
  imágenes** —los repositorios no van dentro de la imagen, así que la tubería de entrega no toca
  Docker en ningún momento— (#276).
- El repositorio dnf **rechaza un `.rpm` que no lleve la firma dentro**, que es lo que
  verifica `gpgcheck=1` en la máquina de quien instala. Y una puerta nueva de
  `just check-actions` fija el orden de `release.yml` —firmar cada `.rpm`, luego el
  `SHA256SUMS`, luego la atestación y sólo entonces adjuntar—: firmar modifica el fichero, así
  que reordenar esos pasos dejaría el `.rpm` de la Release y el del repositorio con bytes
  distintos sin poner en rojo ninguna ejecución (#277).
- Otras dos puertas de `just check-actions`: la publicación tiene que pasarle la huella de
  firma a `build-tree.sh`, y el modo sin firma del constructor —que existe porque las claves
  las crea una persona y ninguna prueba puede fabricarse una— no puede aparecer en ningún
  workflow (#277).
