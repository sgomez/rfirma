# La versión negociada al abrir el canal rige la sesión; `ver` solo cuenta sin canal

El protocolo tiene **una sola versión por trámite**, y rFirma la toma del
primer sitio donde se declara, como AutoFirma:

- Por un canal (`afirma://service` o `afirma://websocket`), de `v` en la
  apertura. Se comprueba ahí y rige todas las operaciones de la sesión; un
  `ver` dentro de una operación que llega por ese canal **se ignora**.
- Por el servidor intermedio, donde no hay apertura, de `ver` en la propia
  operación. Si pide más de lo que rFirma habla, sale `SAF_21` antes de tocar
  nada.

`v` y `ver` no son dos exigencias distintas: son el mismo dato en dos sitios.
El cliente publicado solo manda `ver` por el servidor intermedio, y por los
canales manda `v` (`autoscript.js:1747, 2621, 3715, 3785`). En AutoFirma, las
guardas `if (requestedProtocolVersion == -1)` de `ProtocolInvocationLauncher`
leen `ver` solo cuando no llegó ninguna `v`, y la rama `master` conserva la
misma regla con `null` como centinela.

## Por qué

**La suite de conformidad mide contra AutoFirma, y AutoFirma aquí no se
contradice.** Aplica una sola regla en todos sus transportes: la versión se
fija una vez y no se renegocia operación a operación. Responder distinto es
ser no conforme, sin más.

**`ver` no es «lo mínimo que exige el trámite».** El nombre del getter Java
(`getMinimumProtocolVersion`) lo sugiere, pero el uso no: el valor pasa a ser
la versión de la operación, igual que `v` en un canal. Para que un trámite
exija un cliente mínimo, el protocolo tiene otro parámetro, `mcv`, que se
comprueba en toda operación y por todos los transportes y sale con `SAF_41`.

**La estrictez no protegía a nadie.** Como el cliente publicado nunca manda
`ver` por un canal abierto, comprobarlo ahí solo se disparaba con peticiones
hechas a mano, y lo único que conseguía era apartar a rFirma del original.

## Considered Options

- **Comprobar `ver` en toda operación, por los tres transportes.** Es lo que
  decía antes este ADR. Descartada: se apoyaba en leer `ver` como una exigencia
  del trámite, cosa que AutoFirma no hace, y la garantía que buscaba ya la da
  `mcv`. Dejaba a rFirma no conforme en la suite sin que ninguna sede real
  ganara nada.
- **No leer `ver` en absoluto.** Descartada: por el servidor intermedio es la
  única versión que llega, y AutoFirma rechaza con `SAF_21` un `ver` por encima
  de 4.

## Consecuencias

La comprobación vive en el arranque del servidor intermedio y no en la lectura
de la operación. En el mapa del protocolo (`docs/mapa-protocolo.md`), la fila
de `ver` es **Igual**.

El valor se lee con la misma aritmética que `Integer.parseInt`: 32 bits, con
lo que un `ver` que se desborda no es un entero y vale `1`, igual que en el
original. Ausente vale `0`.
