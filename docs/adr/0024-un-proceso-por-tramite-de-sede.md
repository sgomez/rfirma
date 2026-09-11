# Un proceso por trámite de sede, y el escritorio aparte

rFirma es un solo binario con dos **roles de proceso**, decididos por la línea
de órdenes antes de montar nada y sin cambiar después: el **proceso de
escritorio**, único en el equipo, con la ventana principal y la colocación de
la firma; y el **proceso de sede**, uno por invocación `afirma://`, con su
ventana de sede, sin ventana principal, que termina con su trámite. Ninguno se une al otro ni lo cierra. Es el modelo del original:
`SimpleAfirma.main` atiende la URL con `launch()` y sale, y solo la herramienta
de escritorio comprueba si ya está abierta.

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
- **El proceso de sede termina cuando el trámite ha terminado y no hay
  ventana visible**; si la hay, al cerrarla. La aplicación avisa de que el
  trámite terminó por el puerto de ventana; el adaptador cierra la ventana solo
  si sigue oculta; Tauri sale al quedarse sin ventanas. Nadie llama a `exit`.
  La persona conserva la pantalla de resultado hasta que la cierra.
- **Cerrar la ventana por el gestor de ventanas es cancelar.** En
  consentimiento, confirmación o firma, la X manda `CANCEL` a la sede, retiene
  el cierre hasta que el envío se confirma con un tope de un segundo, y sale.
  Con el desenlace en pantalla o en un callejón sin salida, sale sin más.
- **La biblioteca nativa se abre en el primer trabajo**, no al arrancar: un
  rechazo de protocolo solo paga el hilo del aislado, que nace ocioso.
- **Carpeta de paso propia por proceso, en los dos roles**, nombrada por rol
  y PID bajo la temporal del sistema y borrada entera al salir. Cada arranque
  barre las de procesos que ya no existen; se acepta que un PID reutilizado
  retrase una barrida.
- **La memoria entre sesiones (ADR-0010) se escribe releyendo.** Toda
  mutación, en los dos roles, relee el fichero y toca un solo campo; el
  proceso de sede solo muta el último certificado usado. El fichero de estado
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
