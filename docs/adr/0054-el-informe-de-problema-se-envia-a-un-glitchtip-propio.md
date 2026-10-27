# El informe de problema se envía a un GlitchTip propio y solo cuando la persona lo pide

Cuando un trámite de sede no acaba con éxito, o rFirma sufre un fallo interno en
cualquiera de sus dos ventanas, la persona puede enviar el informe de problema.
El informe va a un único destino: un GlitchTip autoalojado en
`monitor.sgomez.me`, compatible con el protocolo de Sentry. No hay correo,
issue de GitHub, fichero ni portapapeles.

1. **Nada sale sin que la persona pulse «Enviar».** El SDK no envía nada por su
   cuenta. La ventana del informe enseña el texto entero que va a salir, y lo
   que no se enseña no sale: ni el nombre del equipo (`server_name`), ni las
   rutas de las bibliotecas cargadas, ni las migas automáticas del SDK.
2. **Si GlitchTip no contesta, el informe se pierde.** No hay reintento, cola
   ni otro canal de reserva. La ventana dice «No se ha podido enviar» y no
   ofrece nada más.
3. **El DSN va en el binario.** Es público por diseño, porque solo sirve para
   enviar eventos y no para leerlos. El abuso se frena en el servidor.
4. **El informe de un trámite es su diario** y sigue las reglas del ADR-0053.
5. **Un pánico de Rust se informa con su ubicación y su traza, sin su
   mensaje.** La ubicación es `fichero:línea:columna`. La traza son nombres de
   función y se corta donde empieza la maquinaria de pánico de `std`. El
   mensaje no sale porque lleva valores de ejecución, como rutas o datos del
   documento.
6. **Un fallo de React al pintar se informa con el tipo del error, su pila de
   componentes y su traza**, también sin el mensaje.
7. **El binario de release se publica con `strip = "debuginfo"`.** Así conserva
   los nombres de función que necesita la traza. Con `strip = true`, la traza
   queda en un único símbolo inservible.

## Consequences

- Con `strip = "debuginfo"`, el binario de release pesa unos 8 MB más en macOS.
- La traza de React sale minificada en release. Para leerla, hay que subir a
  GlitchTip los source maps de cada versión.
- Un fallo cuya causa solo está en el mensaje del pánico o del error no se
  diagnostica con el informe.
- Si `monitor.sgomez.me` cae, nadie puede informar de nada mientras esté caído.
- La CSP de la ventana tiene que permitir `connect-src` hacia GlitchTip.
