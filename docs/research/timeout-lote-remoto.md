# El canal ante un lote remoto largo: medición del timeout (issue #499)

## La pregunta

El original (`AfirmaWebSocketServer`/`ProtocolInvocationLauncher`, 1.9.2) sube
`setConnectionLostTimeout` del WebSocket de 60 s a 240 s cuando la operación
empieza por `afirma://batch`, porque un lote grande tarda más que una firma
suelta. ¿Hace falta ese mismo ajuste en rFirma?

## Lo que dice el código

Ni `adapters/channel/server.rs` (transporte `wss`) ni `adapters/service/mod.rs`
(transporte `service`) tienen ningún código de tiempo de conexión perdida:

- `wss`: `attend()` hace `socket.next().await` en un bucle sin ningún
  `tokio::time::timeout` ni configuración de *ping* alrededor. La conexión
  vive mientras el socket TCP viva y el par no mande `Close`.
- `service`: `the_framed_request()` hace `stream.read(&mut chunk).await` en
  un bucle sin plazo, hasta encontrar `@EOF` o que el par cierre. Tampoco hay
  temporizador.

Es decir: **rFirma no impone ningún tiempo de conexión perdida propio**, así
que no hay un límite de 60 s que un lote remoto pueda agotar.

## La medición

Ledger #89 pide no quedarse en la lectura del código: dos pruebas de
integración de grada B abren el canal de verdad, mantienen la conexión TLS
callada con `tokio::time::sleep` (tiempo real, sin `pause()`) durante **245 s**
—más que el máximo de 240 s del original— y comprueban que la operación de
siempre (`echo=`) se sigue contestando después:

- `rfirma-app/src-tauri/tests/channel_client.rs`,
  `a_silent_wss_connection_survives_longer_than_the_original_batch_allowance`.
- `rfirma-app/src-tauri/tests/service_idle_timeout.rs`,
  `a_silent_service_connection_survives_longer_than_the_original_batch_allowance`.

Las dos están marcadas `#[ignore]` —duermen de verdad más de cuatro minutos,
así que no corren en `just check`— y se ejecutan a mano con
`cargo test --test <fichero> -- --ignored --nocapture`. Ejecutadas juntas en
este equipo:

```
test a_silent_wss_connection_survives_longer_than_the_original_batch_allowance ... ok
test result: ok. 1 passed; 0 failed; ...; finished in 245.02s

test a_silent_service_connection_survives_longer_than_the_original_batch_allowance ... ok
test result: ok. 1 passed; 0 failed; ...; finished in 245.02s
```

`cargo test` marca ambas «has been running for over 60 seconds» antes de
terminar, lo que confirma que de verdad esperaron el silencio entero y no que
el reloj se adelantó por alguna vía.

## Conclusión

Ni `wss` ni `service` cortan una conexión callada de 60 s ni de 240 s: no hay
timeout propio que agotar. **No hace falta declarar una duración por
operación ni tocar el transporte** — el ADR-0009 (situaciones del canal) no
necesita una nueva, y el ID de "keepalive parametrizable por operación" del
#467 queda sin ejercitar en la práctica porque la premisa que lo motivaba (un
timeout que un lote largo agota) no se da aquí.

Las dos pruebas quedan en el árbol como red de seguridad: si algún día se
añade un timeout de conexión perdida al canal, alguna de las dos lo notará.
