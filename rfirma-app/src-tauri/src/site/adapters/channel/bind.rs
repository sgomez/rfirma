//! Selección y enlace del primer puerto libre sorteado por la sede (ADR-0005).

use std::net::{Ipv4Addr, SocketAddr, TcpListener};

use crate::site::domain::channel::{ChannelError, ChannelLocation, Situation};

pub use crate::site::domain::protocol::THE_PORT_OF_THE_THIRD_PROTOCOL;

/// Ata la ubicación indicada: el primero de los puertos sorteados que esté libre, o el puerto
/// fijo tal cual (ADR-0005).
pub fn bind_first_free(location: &ChannelLocation) -> Result<TcpListener, ChannelError> {
    match location {
        ChannelLocation::Fixed(port) => bind(*port).map_err(|error| {
            ChannelError::new(Situation::NoDrawnPortIsFree, format!("{port}: {error}"))
        }),
        ChannelLocation::Drawn(ports) => {
            let mut refused = Vec::new();

            for port in ports
                .iter()
                .copied()
                .filter(|port| *port != THE_PORT_OF_THE_THIRD_PROTOCOL)
            {
                match bind(port) {
                    Ok(listener) => return Ok(listener),
                    Err(error) => refused.push(format!("{port}: {error}")),
                }
            }

            Err(ChannelError::new(
                Situation::NoDrawnPortIsFree,
                if refused.is_empty() {
                    "no quedaba ningun puerto al que atarse".to_owned()
                } else {
                    refused.join("; ")
                },
            ))
        }
        ChannelLocation::Relay(_) => Err(ChannelError::new(
            Situation::NotListening,
            "el servidor intermedio no abre ningun socket al que atarse",
        )),
    }
}

fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
}

#[cfg(test)]
mod tests;
