# La ventana de sede existe antes que la operación, y solo se enseña cuando hay algo que decir

Un trámite de sede termina avisando a su ventana (`LiveErrand::end` llama a
`SiteWindow::errand_ended`), y la ventana se cierra —y con ella el proceso,
ADR-0024— si sigue oculta. Ese aviso solo funciona si la ventana ya está
registrada en el trámite cuando este termina. Y la ventana solo debe verse si
la persona tiene algo que leer o decidir. Dos reglas lo sostienen, sin que
importe en qué orden lleguen las cosas:

1. **La ventana existe, oculta y registrada en el trámite, antes de que se
   entregue ninguna operación.** `site::attend_launch` negocia, abre el canal y
   registra el trámite, pero **no dispara** la entrega inmediata del servidor
   intermedio: la deja en el canal (`OpenChannel::take_delivery`). El arranque
   (`startup::attend_site_launch_with_threshold`) guarda la ventana en el
   trámite (`keep_the_window`), la abre oculta con «esperando» y solo entonces
   dispara la entrega. Así `site::attend_launch` sigue sin conocer el puerto
   `SiteWindow`, y el fin del trámite siempre encuentra la ventana.
2. **La ventana solo se enseña cuando el trámite tiene algo que decir a la
   persona.** Por servidor intermedio (llegada inmediata) no se enseña nunca
   «Conectando con la sede»: la operación la enseña si su paso deja algo
   delante —un consentimiento, un rechazo enseñado, la falta de certificados…
   (`errand::attend` → `reveal_an_immediate_arrival`)—, y un fallo al subir la
   respuesta la enseña con el rechazo (`the_site_did_not_get_the_answer`). Si la
   operación se contesta sin nada que enseñar (un `SAF_06` subido), el trámite
   termina con la ventana oculta y el proceso acaba sin que llegue a verse. Con
   WebSocket y `service` la espera no cambia: la ventana nace oculta, la enseña
   la llegada del navegador, y el temporizador de respaldo (`WAITING_THRESHOLD`)
   la enseña con «La petición no ha llegado» si el navegador no llega.

Los callejones sin salida del arranque —puertos ocupados, rechazo sin canal,
CA local ausente— son algo que decir, y se enseñan en el acto. Con la CA local
ausente el callejón se abre tras la entrega, para que la operación entregada no
tape lo que bloquea el trámite.

## Considered Options

- **Una regla de precedencia de momentos** (la versión anterior de este ADR):
  `attend_launch` disparaba la entrega antes de que el arranque abriera la
  ventana, y la apertura no pisaba un momento «posterior» a «esperando»
  (`Moment::is_posterior_to`). Se descarta porque era un remiendo del orden, no
  su arreglo: salvaba el momento, pero el trámite podía terminar durante la
  entrega sin ventana registrada, nadie la cerraba, y el arranque la enseñaba
  después en «Conectando con la sede» para siempre.
- **Cerrar en el arranque si el trámite ya terminó**, como se hace a mano con
  el rechazo por el canal. Se descarta porque repara un caso y deja el
  desorden: cada desenlace que ocurra durante la entrega pediría su propio
  parche, mientras que con la ventana registrada antes el fin del trámite la
  cierra por el camino de siempre.
- **Abrir la ventana antes de atender la invocación.** Se descarta porque el
  contenido de la ventana depende del desenlace de la negociación (trámite,
  rechazo por el canal, rechazo en la ventana, callejón), y porque obligaría a
  la capa de aplicación de `site` a conocer la ventana. Basta con que exista
  antes de la entrega, que es lo único que el fin del trámite necesita.

## Consequences

- Por servidor intermedio, una operación que se contesta sola termina el
  proceso sin enseñar nada; una que pide consentimiento abre la ventana ya en
  ese paso.
- El rechazo por el canal del arranque (versión no soportada, credencial
  duplicada) sigue su camino: la ventana oculta lo sostiene, y se cierra al
  servirse o se enseña si la subida falla.
- Con WebSocket queda una carrera que esta regla no cubre: el canal escucha
  desde que se abre en `attend_launch`, y un navegador muy rápido podría
  entregar su operación antes de que el arranque abra la ventana.
