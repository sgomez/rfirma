# Una sola lista de certificados, con la tarjeta en caliente

AutoFirma trata la tarjeta como un almacén aparte: al detectar un lector
pregunta en una ventana modal si se quiere usar, solo admite un almacén activo,
vuelve al de NSS si no hay DNIe y no ve llegar ni marcharse la tarjeta con la
aplicación abierta. Quien firma tiene que entender qué es un almacén para
elegir con qué identidad firma.

rFirma no tiene almacén activo:

1. **Hay una sola lista**, con los certificados de todos los almacenes juntos:
   Firefox, Chrome, el NSS del sistema, el Almacén de rFirma y las tarjetas.
   Nunca se pregunta qué almacén usar.
2. **La tarjeta entra y sale de la lista sola.** Un vigilante PC/SC, en su
   propio hilo y detrás de un puerto de `identity`, dice cómo quedan los
   lectores tras cada cambio. Cuando cambian las tarjetas presentes, el caso de
   uso vuelve a listar, fuera del hilo de la interfaz, solo los almacenes de
   clase tarjeta; lo demás sale del último listado completo. La ventana
   principal recibe por evento la lista de siempre con los certificados de la
   tarjeta añadidos o quitados.
3. **OpenSC no se reinicializa.** El contexto de cada módulo PKCS#11 se carga
   una vez por proceso, y `C_GetSlotList` ya ve la tarjeta nueva y deja de ver
   la retirada. No se usa `C_WaitForSlotEvent`: para pararlo hace falta
   `C_Finalize`, que tiraría el contexto que comparten el listado y la firma.
4. **El estado del lector es una vista, no una fila.** Viaja en el mismo
   evento: sin lector, lector sin tarjeta, leyendo, tarjeta lista (DNIe u otra)
   o ilegible, que es una tarjeta presente que, terminado el listado, ningún
   almacén de tarjeta enseña. Con varios lectores manda el más avanzado:
   leyendo, lista, ilegible, sin tarjeta. La lista solo contiene certificados.
5. **Un certificado que sigue en la lista conserva su asa** (ADR-0011): meter
   o sacar una tarjeta no cambia una elección hecha con otro certificado.
6. **El PIN solo se pide al firmar** (ADR-0025, ADR-0047). Listar una tarjeta
   no abre sesión de usuario en ella.

## Por qué se diverge de AutoFirma

La pregunta modal y el almacén activo existen en AutoFirma porque su listado de
tarjetas es caro y bloquea la ventana. En rFirma el listado corre en otro hilo
y sin PIN, así que no hay nada que preguntar: el estado del lector dice lo que
pasa sin interrumpir, y los demás certificados siguen elegibles mientras se lee
la tarjeta.

## Consequences

- Sin `pcscd`, o en una plataforma donde el vigilante aún no existe, no hay
  estado de lector ni lista en caliente, y la lista se busca como antes, con
  «Volver a buscar».
- Un `.p12` instalado o quitado entra en la lista en caliente cuando la
  ventana vuelve a pedir el listado completo: la lista en caliente no abre los
  almacenes que no son de tarjeta.

## Considered Options

- **Volver a listar todos los almacenes en cada cambio.** Descartada: abre
  perfiles NSS que no han cambiado por una tarjeta que sí.
- **Una fila provisional de «leyendo» o de «ilegible» en la lista.**
  Descartada: la lista solo contiene lo que se puede elegir, y el estado lo
  cuenta la línea del lector.

## Enmienda: sin vigilante hay un estado, «no soportado»

La consecuencia «sin vigilante no hay estado de lector» deja de valer. El
estado de lectores gana `Unavailable` y es el valor inicial: solo pasa a otro
si arranca el vigilante. Sin él, la cabecera no distinguía «he mirado y no hay
lector» de «esta versión no mira».

**Esta decisión caduca** cuando el vigilante cubra todas las plataformas y
canales: `Unavailable` desaparece con la regla de arranque que lo sostiene.
