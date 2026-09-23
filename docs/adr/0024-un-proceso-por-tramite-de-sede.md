# Un proceso por trámite de sede, y el escritorio aparte

rFirma es un solo binario con dos **roles de proceso**, decididos por la línea
de órdenes antes de montar nada y sin cambiar después: el **proceso de
escritorio**, único en el equipo, con la ventana principal y la colocación de
la firma; y el **proceso de sede**, uno por invocación `afirma://`, con su
ventana de sede, sin ventana principal, que termina con su trámite. Ninguno se
une al otro ni lo cierra. Es el modelo del original: `SimpleAfirma.main`
atiende la URL con `launch()` y sale, y solo la herramienta de escritorio
comprueba si ya está abierta.

**Lo que dura un trámite lo decide su transporte.** Con `service` y con el
servidor intermedio, el trámite es una operación, y acaba con su respuesta.
Con WebSocket (`afirma://websocket`, versiones 3 y 4), el trámite atiende las
operaciones sucesivas que lleguen por el canal **mientras siga conectado su
primer cliente**, como el original: `SimpleAfirma.main` no fuerza el cierre
cuando la URL empieza por `afirma://websocket`, y el `onClose` del servidor
solo termina el proceso cuando se va `wsClient`, la primera conexión que
completó el saludo. `autoscript.js` cuenta con ello: `isAppOpened()` reutiliza
el socket abierto para la siguiente operación, con su eco delante, y trata su
`onclose` como la caída de la aplicación.

Hasta ahora las dos cosas vivían en un proceso, unidas por el plugin de
instancia única: un `afirma://` llegado con la ventana principal abierta abría
la ventana de sede al lado, y el transporte del servidor intermedio, que no
tiene canal que sostener, cerraba el proceso entero al contestar, escritorio
incluido. Además una firma local en curso no impedía abrir un trámite sobre el
mismo aislado, y un segundo trámite se rechazaba por «ya hay uno vivo», que era
un límite del proceso, no de la sede.

## Reglas que se derivan

- **La instancia única es del proceso de escritorio.** El proceso de sede no
  la registra ni la consulta. `rfirma documento.pdf` sigue llegando al
  escritorio abierto y reemplazando lo que hubiera.
- **Dos trámites conviven en dos procesos**, cada uno en la terna de `ports=`
  que sorteó su navegador; no hay cerrojo de exclusión. Dentro de un proceso
  sigue habiendo un solo trámite.
- **Con `service` y con el servidor intermedio, el proceso de sede termina
  cuando el trámite ha terminado y no hay ventana visible**; si la hay, al
  cerrarse, a mano o por el cierre automático del desenlace. La aplicación
  avisa de que el trámite terminó por el puerto de ventana; el adaptador
  espera el acuse de entrega de la respuesta y cierra la ventana solo si sigue
  oculta; Tauri sale al quedarse sin ventanas. Un rechazo que viaja por el
  canal también abre la ventana oculta, que se cierra al servirse el rechazo o
  al vencer la espera. Nadie llama a `exit`.
- **Con WebSocket, contestar acaba la operación, no el trámite.** Se olvidan
  la petición, el consentimiento, el asa de respuesta y el documento de paso
  de esa operación; el canal, el códec negociado, la ventana y `sticky`
  (ADR-0010) siguen. Cada operación que llega vuelve a enseñar la ventana en
  la espera, como la primera; una operación contestada sin nada que enseñar
  la oculta en el acto, y la que dejó un desenlace en pantalla la oculta al
  cerrarse, a mano o por su cierre automático. El rechazo de la petición
  misma (`SAF_03`, `SAF_04`, `SAF_13`) es un desenlace que la sede aún no ha
  recibido, como el diálogo de error del original: se contesta al cerrarse, y
  la ventana se oculta y olvida la operación **antes** de escribir la
  respuesta, porque la sede puede mandar la siguiente en cuanto la lee.
- **Con WebSocket, el trámite y el proceso terminan cuando se va el primer
  cliente**: el que completó primero el saludo cierra su conexión, sea como
  sea. La aplicación cierra entonces la ventana, con la operación que hubiera
  en vuelo, y Tauri sale al quedarse sin ventanas. Que se vaya otro cliente no
  termina nada, y sus operaciones se atienden igual. Sin temporizador de
  inactividad ni latido. Si el navegador no llega nunca, la espera de treinta
  segundos enseña la ventana como siempre, y cerrarla termina el proceso.
- **Un `jvc` menor que 1 retiene el arranque tras un aviso**, como el diálogo
  modal del original: la ventana lo enseña antes de abrir el canal o de ir al
  servidor intermedio, y descartarlo, con `Entendido` o con la X, arranca el
  trámite como si no hubiera habido aviso. Las esperas de arranque cuentan
  desde ahí.
- **Una operación a la vez por canal.** Por una misma conexión llegan en
  serie: lo que el cliente mande mientras una está en vuelo se atiende al
  acabar ella. Una operación que llega por otra conexión con una en vuelo se
  rechaza con `SAF_45`, el mismo código que ya recibe una invocación con un
  trámite vivo.
- **La respuesta a la sede tiene acuse de entrega.** El asa de respuesta dice
  cuándo la respuesta ha salido por el canal, en los tres transportes; sin él,
  cerrar el proceso podría cortar una respuesta a medio escribir.
- **Cerrar la ventana por el gestor de ventanas es cancelar.** En
  consentimiento, confirmación o firma, la X manda `CANCEL` a la sede, retiene
  el cierre hasta el acuse de entrega con un tope de un segundo, y sale.
  Con el rechazo de la petición en pantalla, manda ese rechazo en vez de
  `CANCEL`. Con otro desenlace o en un callejón sin salida, sale sin más. Con
  WebSocket y el primer cliente conectado, en vez de salir la oculta. Sobre
  el aviso de `jvc`, solo lo descarta.
- **La biblioteca nativa se abre en el primer trabajo**, no al arrancar: un
  rechazo de protocolo solo paga el hilo del aislado, que nace ocioso.
- **Carpeta de paso propia por proceso, en los dos roles**, nombrada por rol
  y un sufijo aleatorio bajo la temporal del sistema y borrada entera al
  salir. El proceso mantiene un cerrojo sobre un fichero de su carpeta mientras
  vive, y cada arranque barre solo las carpetas cuyo cerrojo consigue tomar.
  No se barre por PID: flatpak da a cada instancia su propio espacio de PID.
  El documento de paso de cada operación se borra al acabar esa operación.
- **La memoria entre sesiones (ADR-0010) se escribe releyendo.** Toda
  mutación relee el fichero y toca un solo campo; el proceso de sede no muta
  ninguno, ni siquiera el último certificado usado. El fichero de estado
  se lee del disco en cada acceso; la configuración, que la sede nunca
  escribe, conserva su copia viva.
- **La CA local (ADR-0005) la refresca el escritorio.** El proceso de sede no
  toca las ranuras ni los almacenes: mira solo el fichero de la CA, y si falta
  o le quedan menos de siete días enseña el callejón sin salida con el botón
  de instalar, que ya existía para «falta». Que la hoja esté en el almacén del
  navegador lo detecta la espera de treinta segundos, que ya ofrece reparar.

## Considered Options

- **Dos aplicaciones, dos binarios.** Descartado: el proceso de sede ya monta
  su propia ventana sin nada del árbol de la principal, y todo lo demás es el
  mismo crate; dos binarios doblan el empaquetado para separar lo que separa
  una rama en la raíz de composición.
- **Seguir en un proceso y arreglar el `exit` del relay.** Descartado: quedaba
  la sesión local y el trámite compartiendo aislado, y el segundo trámite
  rechazado por un límite que la sede no impone.
- **Un cerrojo para que solo haya un trámite en el equipo.** Descartado por
  innecesario: el protocolo ya separa los trámites por puerto y credencial. Si
  hiciera falta por política, es un cerrojo en `$XDG_RUNTIME_DIR`, no la
  instancia única.
- **Que el proceso de sede también refresque la CA bajo un cerrojo, o que
  arranque el escritorio en segundo plano.** Descartado: la ventana de sede ya
  sabe reparar la CA a petición, y la persona ve por qué se le pide.
- **Un trámite por operación también con WebSocket: cerrar el socket tras la
  respuesta y terminar el proceso.** Era el modelo anterior. Descartado:
  rompe `isAppOpened()` y el `onclose` de `autoscript.js`. La segunda
  operación de una sede ya no encuentra el canal y paga una invocación nueva,
  con su espera de arranque; y las órdenes que una sede manda seguidas por el
  mismo socket se quedan sin respuesta desde la primera.
- **El latido del original (`setConnectionLostTimeout`, 60 s y 240 s en
  lote).** Descartado: el canal no tiene temporizador propio, y una conexión
  callada tiene que sobrevivir a esos plazos; cerrar la pestaña o el navegador
  ya llega como cierre del socket.
- **Un aviso de `jvc` que no retiene el canal.** Descartado: con el canal ya
  abierto, la sede opera en el acto y su primera operación sustituye el aviso
  antes de que nadie lo lea.
- **Poner en cola la operación que llega por otra conexión.** Descartado: el
  original no lo describe, y un rechazo inmediato con el código de trámite
  vivo no deja a nadie esperando detrás de un diálogo que no ve.
