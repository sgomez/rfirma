# La versión mínima que exige la operación se comprueba siempre, venga por donde venga

El parámetro `ver` de una invocación `afirma://` es la **versión mínima de
protocolo que la sede necesita** para que su trámite se ejecute como lo
diseñó. No describe el transporte —eso es `v`—, sino lo que la sede da por
supuesto del cliente que va a firmar.

AutoFirma solo lo mira cuando no hay nada mejor: `ProtocolInvocationLauncher`
compara `ver` contra la versión implementada **únicamente** si la versión
pedida al arrancar vale `-1`, es decir, en el camino del servidor intermedio,
donde no llegó ningún `v`. Por el canal ya abierto (`websocket` y `service`)
manda `v` y `ver` queda sin leer, aunque venga dentro de la misma operación
cifrada.

rFirma comprueba `ver` **en toda operación y por los tres transportes**, en
`read_operation`, antes de tocar `dat` o el verbo. Donde el original atendería
un trámite que declara necesitar una versión que aquí no se habla, rFirma lo
rechaza con `SAF_21` antes de firmar nada.

Es, por tanto, una divergencia deliberada y **siempre en el sentido estricto**:
rFirma nunca atiende un `ver` que el original rechazaría, y sí rechaza alguno
que el original atendería.

## Por qué

**Aceptar de más es incompatibilidad, y de la peor clase.** La sede prueba su
trámite contra AutoFirma; si rFirma es más laxa, la sede no se entera de nada
hasta que le llega una firma que no esperaba. `ver` existe precisamente para
que el cliente diga «esto no lo sé hacer» **antes** de hacerlo, y silenciarlo
según el transporte convierte una comprobación de compatibilidad en una
lotería: la misma operación, con el mismo `ver`, se atiende o no según por
dónde entró.

El rechazo, en cambio, es **visible**: la sede recibe `SAF_21`, que es el
código que el propio original usa para esta situación, y puede decidir. Un
trámite roto que lo dice es reparable; uno que se firma con supuestos que no se
cumplen, no.

## Considered Options

- **Replicar el original literalmente** (mirar `ver` solo cuando no hay `v`).
  Descartada: reproduce un descuido, no una decisión. La condición `== -1` del
  original es un efecto de cómo se propaga la versión del arranque, no una
  regla que nadie escribiera; y el precio de imitarla es atender trámites que
  declaran no poder ejecutarse aquí.
- **No leer `ver` en absoluto**, que es como estaba antes del
  [#618](https://github.com/sgomez/rfirma/issues/618). Descartada: es la
  divergencia más laxa de todas: rFirma atendía cualquier `ver`, incluso uno
  que el original rechaza en el camino del servidor intermedio.

## Consecuencias

Una sede que mande un `ver` por encima de la versión de protocolo que rFirma
habla recibe `SAF_21` por cualquier transporte. En el mapa del protocolo
(`docs/mapa-protocolo.md`), la fila de `ver` es **desviación declarada** por
este ADR, y no un hueco.

El valor se lee con la misma aritmética que `Integer.parseInt`: 32 bits, con
lo que un `ver` que se desborda no es un entero y vale `1`, igual que en el
original. Ausente vale `0`.
