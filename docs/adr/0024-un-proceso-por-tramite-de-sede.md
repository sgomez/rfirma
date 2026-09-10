# Un proceso por trámite de sede, y el escritorio aparte

rFirma es un solo binario con dos **roles de proceso**, decididos por la línea
de órdenes antes de montar nada y sin cambiar después: el **proceso de
escritorio**, único en el equipo, con la ventana principal y la colocación de
la firma; y el **proceso de sede**, uno por invocación `afirma://`, con su
ventana de sede, sin ventana principal, que termina cuando se cierra esa
ventana. Ninguno se une al otro ni lo cierra. Es el modelo del original:
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
- **El proceso de sede termina cuando se cierra su ventana**, que es la única.
  Cerrarla a medias sigue cancelando ante la sede, y la persona conserva la
  pantalla de resultado hasta que la cierra. Nadie llama a `exit` desde un
  adaptador.
- **El aislado se crea al consentir**, no al arrancar: un rechazo de protocolo
  no lo paga.
- **Carpeta de paso propia por proceso**, borrada al salir.
- **La memoria entre sesiones (ADR-0010) la escribe el escritorio.** El proceso
  de sede la lee entera y escribe solo el último certificado usado, releyendo
  el fichero antes de escribir; el escritorio relee esa rebanada al listar
  certificados.
- **La CA local (ADR-0005) la refresca el escritorio.** El proceso de sede no
  toca las ranuras ni los almacenes: si la CA falta o caduca pronto, enseña el
  callejón sin salida con el botón de instalar, que ya existía para «falta».

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
