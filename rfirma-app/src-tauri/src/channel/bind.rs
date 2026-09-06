//! Selección y enlace del primer puerto libre sorteado por la sede (ADR-0005).

use std::net::{Ipv4Addr, SocketAddr, TcpListener};

use crate::channel::error::{ChannelError, Situation};

/// Puerto fijo del protocolo 3 que nunca se enlaza.
pub const THE_PORT_OF_THE_THIRD_PROTOCOL: u16 = 63117;

/// Ata el primero de los puertos sorteados que esté libre.
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
            "no quedaba ningun puerto al que atarse".to_owned()
        } else {
            refused.join("; ")
        },
    ))
}

#[cfg(test)]
mod tests;
