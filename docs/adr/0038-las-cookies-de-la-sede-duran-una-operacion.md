# Las cookies de la sede duran una operación

AutoFirma 1.9.2 instala en un bloque estático un `CookieManager` con
`CookiePolicy.ACCEPT_ALL` como `CookieHandler` de toda la JVM
(`UrlHttpManagerImpl.java:60-64`). Toda conexión del proceso guarda y
reenvía las cookies que recibe: entre la prefirma y la postfirma del lote,
entre las llamadas al servidor intermedio y entre la prefirma y la postfirma
trifásicas. rFirma no guardaba ninguna.

Un servicio que dependa de una cookie entre dos llamadas, como un balanceador
con afinidad de sesión que no reescribe la URL, manda la segunda llamada a
otro servidor que no conoce la primera. Con AutoFirma funciona; con rFirma
fallaba.

## La regla

1. **Cada operación de sede tiene su almacén de cookies**, compartido por los
   cuatro clientes HTTP (servlets del lote, servidor intermedio, servidor
   trifásico y descarga del `dat`). Nace con la operación y muere con su
   respuesta.
2. **Dentro de la operación se guarda y se reenvía todo, como `ACCEPT_ALL`**:
   las cookies de sesión y las persistentes, con las reglas de dominio y de
   ruta de siempre.
3. **Nada pasa de una operación a otra**, ni dentro del mismo trámite con
   `service` o WebSocket, ni entre trámites. Tampoco se escribe nada a disco.

Lo mide la comprobación de conformidad
`the_batch_keeps_the_cookie_between_presign_and_postsign`, con un servlet
falso que rechaza la postfirma si no trae la cookie que puso en la
prefirma.

## Consequences

- Una sede con afinidad por cookie funciona igual que con AutoFirma dentro
  de una operación, que es donde la afinidad se usa.
- Una sede que dependa de una cookie puesta en una operación anterior falla
  con rFirma y funciona con AutoFirma mientras la JVM siga abierta. No se
  conoce ninguna. Con el servidor intermedio el proceso de AutoFirma atiende
  una sola operación, así que la diferencia solo existe con `service` y con
  WebSocket.

## Considered Options

**Un almacén por proceso, como el original.** Es la copia literal. Con
`service` y con WebSocket el proceso atiende operaciones sucesivas, y una
cookie de una sede viajaría a otra sede que comparta dominio, o a la misma
sede en una operación que ya no es la que la creó. El original no lo acota porque su almacén es
global a la JVM, no porque lo decida. Descartada: el alcance de una operación
cubre el caso que justifica las cookies y no arrastra estado entre sedes.

**Solo en los servlets del lote.** Es donde el caso es más claro, pero la
afinidad de sesión puede estar delante de cualquiera de los cuatro servicios,
y el original no distingue. Descartada.

**Sin cookies.** Deja fallar una sede real por una diferencia que no protege
de nada dentro de una misma operación. Descartada.
