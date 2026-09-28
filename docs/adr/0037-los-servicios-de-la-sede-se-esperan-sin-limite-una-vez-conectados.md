# Los servicios de la sede se esperan sin límite una vez conectados

rFirma llama por HTTP a cuatro servicios de la sede: los servlets del lote,
el servidor intermedio, el servidor trifásico y la descarga del `dat`. Los
cuatro clientes cortaban la petición entera a los 30 s. AutoFirma 1.9.2 no
fija ningún tiempo de espera: los cuatro envíos equivalentes llaman a
`readUrl` sin `timeout`, que deja `UrlHttpManagerImpl.DEFAULT_TIMEOUT` en
`-1`, sin `connectTimeout` (`UrlHttpManagerImpl.java:279-282`) y sin
`readTimeout`. Espera lo que tarde el servicio.

Con 30 s, una postfirma de un lote grande o una firma trifásica sobre un
documento grande fallaba en rFirma y funcionaba con AutoFirma: el mismo tipo
de fallo que el del lote con los parámetros en la query, que solo se veía
en una sede real.

## La regla

1. **Conectar tiene límite: 30 s.** Un servicio que ni acepta la conexión no
   va a contestar, y la persona se entera pronto. El original espera lo que
   decida el sistema operativo; esa diferencia no cambia ningún trámite que
   el original complete.
2. **Esperar la respuesta no tiene límite**, como en el original. Los cuatro
   clientes lo aplican igual.
3. **La salida de una espera larga es cerrar la ventana de sede.** El cierre
   contesta a la sede con la cancelación y termina el trámite sin esperar a
   la petición en curso. Con el servidor intermedio el proceso de sede
   termina con él (ADR-0024). Con `service` y con WebSocket el proceso sigue
   sirviendo, y la petición se queda esperando en su hilo hasta que el
   servicio conteste o corte la conexión, sin que su respuesta llegue ya a
   ninguna parte.

Lo mide la comprobación de conformidad
`the_batch_waits_for_a_postsigner_slower_than_thirty_seconds`, con un servlet
falso que tarda más de 30 s en contestar a la postfirma.

## Consequences

- Un servicio que acepta la conexión y no contesta nunca deja la ventana de
  sede esperando hasta que la persona la cierre. Es lo que hace AutoFirma.
- Con `service` y con WebSocket, un hilo colgado ocupa memoria hasta que el
  servicio corta o el canal termina, y no puede impedir que el canal atienda
  la siguiente operación.

## Considered Options

**Mantener los 30 s para toda la petición.** Deja una red de seguridad, pero
el número no sale del original ni de ninguna medición, y corta trámites que
AutoFirma completa. Descartada.

**Un límite de lectura alto (10 minutos, por ejemplo).** Cualquier número es
una apuesta sobre el tamaño del lote más grande de una sede real, y cuando
falla es el mismo fallo silencioso de antes, solo que más raro. Como cerrar
la ventana ya libera a la persona, el límite no protege a nadie. Descartada.

**Derivar el límite del tamaño del lote.** Pide conocer lo que tarda el
servidor por documento, que depende de la sede. Descartada por la misma
razón.
