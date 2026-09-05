//! **Atarse a uno de los puertos que sorteó la sede**, y a ninguno más
//! (ID-215).
//!
//! Los tres puertos vienen en `ports=` de la URL de arranque y se prueban en el
//! orden en que la sede los mandó (`AfirmaWebSocketServerManager.startService`,
//! §«Qué pasa si ningún puerto está libre» del informe del protocolo). Aquí no
//! hay puerto por omisión: **el 63117 no se ata jamás**, ni siquiera para
//! contestar un error, porque es el puerto fijo del protocolo 3 y el protocolo
//! 3 no existe en rfirma (ID-247).
//!
//! Se ata **sólo en `127.0.0.1`**. Un `0.0.0.0` abriría el canal a la red
//! local, y la guardia de origen del original —`SAF_47`— es la segunda
//! cerradura, no la primera.

use std::net::{Ipv4Addr, SocketAddr, TcpListener};

use crate::channel::error::{ChannelError, Situation};

/// El puerto fijo del protocolo 3 (`DEFAULT_WEBSOCKET_PORT`,
/// `ProtocolInvocationLauncher.java:87`). **Nunca se ata** (ID-215).
pub const THE_PORT_OF_THE_THIRD_PROTOCOL: u16 = 63117;

/// Ata el primero de los puertos sorteados que esté libre.
///
/// Devuelve el escuchador **ya enlazado**, que es lo que
/// [`crate::channel::server::serve`] recibe (ID-213): quien ata y quien sirve
/// son dos pasos, y por eso una prueba puede pasarle un puerto efímero sin
/// montar nada.
pub fn bind_first_free(ports: &[u16]) -> Result<TcpListener, ChannelError> {
    let mut refused = Vec::new();

    for port in ports
        .iter()
        .copied()
        .filter(|port| *port != THE_PORT_OF_THE_THIRD_PROTOCOL)
    {
        match TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))) {
            Ok(listener) => return Ok(listener),
            Err(error) => refused.push(format!("{port}: {error}")),
        }
    }

    Err(ChannelError::new(
        Situation::NoDrawnPortIsFree,
        if refused.is_empty() {
            "la invocacion no trajo ningun puerto al que atarse".to_owned()
        } else {
            refused.join("; ")
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada B**: se atan puertos de verdad, siempre en el *loopback* y
    /// siempre efímeros.
    fn an_occupied_port() -> (TcpListener, u16) {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .expect("el sistema deberia dar un puerto efimero");
        let port = listener
            .local_addr()
            .expect("un escuchador atado tiene direccion")
            .port();
        (listener, port)
    }

    #[test]
    fn the_first_free_of_the_drawn_ports_is_the_one_that_is_bound() {
        let (occupied, taken) = an_occupied_port();
        let (free, available) = an_occupied_port();
        drop(free);

        let listener = bind_first_free(&[taken, available]).expect("el segundo estaba libre");

        assert_eq!(
            listener.local_addr().expect("atado").port(),
            available,
            "se prueban en el orden en que la sede los mando"
        );
        drop(occupied);
    }

    #[test]
    fn the_channel_only_listens_on_the_loopback() {
        let (free, available) = an_occupied_port();
        drop(free);

        let listener = bind_first_free(&[available]).expect("estaba libre");

        assert_eq!(
            listener.local_addr().expect("atado").ip(),
            Ipv4Addr::LOCALHOST,
            "un canal atado a 0.0.0.0 estaria abierto a la red local"
        );
    }

    #[test]
    fn with_every_drawn_port_taken_there_is_no_channel() {
        let (occupied, taken) = an_occupied_port();

        let error = bind_first_free(&[taken]).expect_err("el unico puerto estaba ocupado");

        assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
        assert!(error.detail().contains(&taken.to_string()));
        drop(occupied);
    }

    /// El puerto del protocolo 3 no se ata **aunque la sede lo sortee**
    /// (ID-215): rfirma no habla ese protocolo, y un canal ahí sería rfirma
    /// haciéndose pasar por el AutoFirma que sí lo habla.
    #[test]
    fn the_port_of_the_third_protocol_is_never_bound() {
        let error = bind_first_free(&[THE_PORT_OF_THE_THIRD_PROTOCOL])
            .expect_err("ese puerto no se ata jamas");

        assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
        assert!(
            !error.detail().contains("63117"),
            "no es que estuviera ocupado: es que ni se intenta"
        );
    }

    #[test]
    fn a_launch_without_drawn_ports_binds_nothing() {
        let error = bind_first_free(&[]).expect_err("sin puertos no hay canal");

        assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
    }
}
