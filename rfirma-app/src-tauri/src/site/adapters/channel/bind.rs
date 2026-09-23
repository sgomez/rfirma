//! Selección y enlace del primer puerto libre sorteado por la sede, en los dos bucles locales (ADR-0005).

use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener};

use crate::site::domain::channel::{ChannelError, ChannelLocation, Situation};

pub use crate::site::domain::protocol::THE_PORT_OF_THE_THIRD_PROTOCOL;

/// Los escuchadores de un mismo puerto: el de `127.0.0.1` y, si el equipo lo tiene, el de `::1`.
#[derive(Debug)]
pub struct LoopbackListeners {
    ipv4: TcpListener,
    ipv6: Option<TcpListener>,
}

impl LoopbackListeners {
    /// El puerto que comparten.
    pub fn port(&self) -> io::Result<u16> {
        Ok(self.ipv4.local_addr()?.port())
    }

    /// Las direcciones en las que escuchan, la de IPv4 primero.
    pub fn addresses(&self) -> Vec<SocketAddr> {
        std::iter::once(&self.ipv4)
            .chain(&self.ipv6)
            .filter_map(|listener| listener.local_addr().ok())
            .collect()
    }

    /// Los mismos escuchadores sobre el reactor de tokio.
    pub fn into_async(self) -> io::Result<LoopbackAcceptor> {
        let on_tokio = |listener: TcpListener| {
            listener.set_nonblocking(true)?;
            tokio::net::TcpListener::from_std(listener)
        };
        Ok(LoopbackAcceptor {
            ipv4: on_tokio(self.ipv4)?,
            ipv6: self.ipv6.map(on_tokio).transpose()?,
        })
    }
}

impl From<TcpListener> for LoopbackListeners {
    fn from(ipv4: TcpListener) -> Self {
        Self { ipv4, ipv6: None }
    }
}

/// Los escuchadores ya en tokio, que aceptan por cualquiera de los dos bucles locales.
#[derive(Debug)]
pub struct LoopbackAcceptor {
    ipv4: tokio::net::TcpListener,
    ipv6: Option<tokio::net::TcpListener>,
}

impl LoopbackAcceptor {
    /// La siguiente conexión que llegue por cualquiera de ellos.
    pub async fn accept(&self) -> io::Result<(tokio::net::TcpStream, SocketAddr)> {
        match &self.ipv6 {
            None => self.ipv4.accept().await,
            Some(ipv6) => tokio::select! {
                accepted = self.ipv4.accept() => accepted,
                accepted = ipv6.accept() => accepted,
            },
        }
    }
}

/// Ata la ubicación indicada: el primero de los puertos sorteados que esté libre —de `wss` o de
/// `service`, con el mismo filtro—, o el puerto fijo tal cual (ADR-0005).
pub fn bind_first_free(location: &ChannelLocation) -> Result<LoopbackListeners, ChannelError> {
    match location {
        ChannelLocation::Fixed(port) => bind(*port).map_err(|error| {
            ChannelError::new(Situation::NoDrawnPortIsFree, format!("{port}: {error}"))
        }),
        ChannelLocation::Drawn(ports) | ChannelLocation::Service(ports) => bind_first_of(ports),
        ChannelLocation::Relay(_) => Err(ChannelError::new(
            Situation::NotListening,
            "el servidor intermedio no abre ningun socket al que atarse",
        )),
    }
}

fn bind_first_of(ports: &[u16]) -> Result<LoopbackListeners, ChannelError> {
    let mut refused = Vec::new();

    for port in ports
        .iter()
        .copied()
        .filter(|port| *port != THE_PORT_OF_THE_THIRD_PROTOCOL)
    {
        match bind(port) {
            Ok(listeners) => return Ok(listeners),
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

fn bind(port: u16) -> io::Result<LoopbackListeners> {
    let ipv4 = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))?;
    let ipv6 = match TcpListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, port))) {
        Ok(listener) => Some(listener),
        Err(error) if error.kind() == ErrorKind::AddrInUse => return Err(error),
        Err(_) => None,
    };
    Ok(LoopbackListeners { ipv4, ipv6 })
}

#[cfg(test)]
mod tests;
