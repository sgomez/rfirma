# La apertura de la ventana de sede no pisa un momento posterior del trámite

Cuando una sede electrónica arranca rFirma a través de un servidor intermedio
(`afirma://sign?fileid=…&rtservlet=…`, tanto si trae `stservlet` en la URL como
si este viene declarado dentro del XML de parámetros recuperado), no existe
ningún canal local ni navegador al que esperar en loopback. La operación se
descarga, descifra y entrega en el acto durante la propia atención de la
invocación (`site::attend_launch`), y el trámite vivo (`LiveErrand`) calcula y
anota su momento real de ejecución (`AskingToSign`, `AskingForConsent`, etc.).

Al completarse la atención de la invocación, el arranque abre la ventana de sede
asociada al trámite en servicio (`SiteWindowContent::TheErrand`). Hasta ahora, la
función de apertura anotaba incondicionalmente el momento del contenido con el
que se abría, que para un trámite en servicio es «esperando» (`Moment::Waiting`),
pisando el momento bueno ya alcanzado. En este transporte no llega ningún
mensaje posterior que vuelva a mover el trámite, con lo que la persona quedaba
atrapada en «Conectando con la sede» de forma indefinida, sin poder ver los
certificados ni consentir la firma.

Este ADR establece que **la apertura de la ventana no pisa un momento posterior
al suyo**, formalizando la precedencia del momento inicial de espera respecto a
los momentos derivados de atender una operación.

## Por qué no abrir la ventana antes de atender la invocación

Se evaluó la alternativa de abrir la ventana de sede antes de atender la
invocación, asemejándose a cómo AutoFirma inicializa su entorno gráfico antes de
ejecutar la orden del protocolo. Sin embargo, en la arquitectura de rFirma
(`ADR-0005`, `ADR-0017`), los tres transportes (`wss`, `service` y servidor
intermedio) comparten una costura unificada en `site::attend_launch` que decide
el desenlace (`Attendance`) antes de interactuar con el puerto de ventana:

1. **Rechazos por canal (`RefusingOverTheChannel`):** Si una invocación viene
   con error de protocolo (como una versión no soportada o credencial duplicada)
   y tiene canal donde responder, el rechazo viaja por el cable y **no debe
   abrir ventana** alguna. Abrir la ventana antes de evaluar la invocación
   obligaría a abrir y destruir ventanas fantasma ante rechazos comunes.
2. **Segundo trámite en el mismo proceso:** cada invocación `afirma://`
   arranca su propio proceso (ADR-0024), así que dentro de uno solo hay un
   trámite; si aun así llegara otro, se rechaza sin abrir una segunda ventana.
3. **Separación de capas:** `site::attend_launch` pertenece a la capa de
   aplicación pura de trámites y no conoce el puerto de ventana (`SiteWindow`),
   que se gestiona en `startup`. Mover la apertura dentro o antes requeriría
   desacoplar la entrega diferida o posponerla, introduciendo ventanas en blanco
   mientras se resuelven las peticiones de red del servidor intermedio.

## Regla de precedencia de momentos

`Moment::Waiting` modela el estado preliminar de canal abierto a la espera de la
llegada de la petición del navegador. Todo momento correspondiente a una
operación atendida (`AskingToSign`, `AskingForConsent`, `NoCertificate`, etc.),
así como cualquier callejón sin salida irrecuperable, es posterior a `Waiting`.

La regla de apertura es:

- Si el trámite vivo ya contiene un momento posterior al que aporta el
  contenido de apertura (en particular, si ya está en una fase de operación y el
  contenido propone `Waiting`), se conserva el momento existente.
- Si el trámite vivo no tiene momento anotado (como ocurre en `wss` y `service`
  recién arrancados), se anota el momento de apertura (`Waiting`) y se arma el
  temporizador de respaldo habitual.
- Los callejones sin salida (`SiteWindowContent::ADeadEnd`: CA local ausente o
  rechazo sin canal) no se consideran inferiores y siempre prevalecen para
  garantizar que los bloqueos se muestren a la persona usuaria.

## Consecuencias

- En el arranque por servidor intermedio (con `stservlet` en la URL o dentro del
  XML de parámetros), la ventana muestra directamente el paso de consentimiento
  con los certificados correspondientes al montarse.
- Los transportes `wss` y `service` continúan enseñando «Conectando con la
  sede» (`Waiting`) hasta que el navegador envía su primer mensaje, y su
  temporizador de «La petición no ha llegado» se arma de forma idéntica.
- Un rechazo sin canal sigue abriendo la ventana en su callejón sin salida sin
  verse afectado.
- Se implementa `Moment::is_posterior_to` para reflejar la precedencia entre el
  momento de espera inicial y los momentos de operación.
