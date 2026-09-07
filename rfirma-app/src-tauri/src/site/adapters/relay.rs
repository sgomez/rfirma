//! Transporte del servidor intermedio: sin canal que sostener, la operación llega ya resuelta desde la propia invocación (ADR-0005, ADR-0017).

use std::sync::Arc;

use crate::site::application::errand::{Inbox, ReplyHandle, Transport};
use crate::site::domain::channel::{
    ChannelDuty, ChannelError, ChannelLocation, OpenChannel, Shutdown, Situation,
};
use crate::site::domain::protocol::{decrypt, AfirmaUrl, Refusal, RelayChannelInfo};
use crate::site::domain::relay_error::{RelayError, Situation as RelaySituation};
use crate::site::ports::Servlets;

use super::frontier::code_of_relay;

/// El servidor intermedio: descarga y descifra la petición una vez, la entrega por el mismo
/// buzón que el `wss`, y su asa de respuesta sube la contestación cifrada y termina el proceso.
pub struct Relay {
    servlets: Arc<dyn Servlets + Send + Sync>,
    inbox: Inbox,
    exit: Arc<dyn Fn() + Send + Sync>,
    on_upload_failure: Arc<dyn Fn(Refusal) + Send + Sync>,
}

impl Relay {
    /// Un transporte de servidor intermedio sobre los servlets, el buzón y los dos avisos dados:
    /// el cierre del proceso tras subir, y el que se da cuando la subida se rechaza.
    pub fn new(
        servlets: Arc<dyn Servlets + Send + Sync>,
        inbox: Inbox,
        exit: Arc<dyn Fn() + Send + Sync>,
        on_upload_failure: Arc<dyn Fn(Refusal) + Send + Sync>,
    ) -> Self {
        Self {
            servlets,
            inbox,
            exit,
            on_upload_failure,
        }
    }
}

impl Transport for Relay {
    fn open(
        &self,
        location: &ChannelLocation,
        _duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError> {
        let ChannelLocation::Relay(info) = location else {
            return Err(ChannelError::new(
                Situation::Relay,
                "esta ubicacion de canal no es la de un servidor intermedio",
            ));
        };

        if info.active_wait {
            self.servlets
                .wait(&info.store_servlet, &info.id)
                .map_err(|error| ChannelError::refused(refusal_of(error)))?;
        }

        let operation = resolve_operation(info, self.servlets.as_ref())
            .map_err(|error| ChannelError::refused(refusal_of(error)))?;

        let servlets = Arc::clone(&self.servlets);
        let store_servlet = info.store_servlet.clone();
        let id = info.id.clone();
        let exit = Arc::clone(&self.exit);
        let on_upload_failure = Arc::clone(&self.on_upload_failure);
        let reply =
            ReplyHandle::of(
                move |text: String| match servlets.store(&store_servlet, &id, &text) {
                    Ok(()) => exit(),
                    Err(error) => on_upload_failure(refusal_of(error)),
                },
            );

        (self.inbox)(operation, reply);

        Ok(OpenChannel::new(0, Shutdown::of(|| {})))
    }
}

/// Resuelve el `dat` de la operación: inline, o descargado y descifrado por `fileid` (ID-267).
fn resolve_operation(
    info: &RelayChannelInfo,
    servlets: &(dyn Servlets + Send + Sync),
) -> Result<AfirmaUrl, RelayError> {
    if info.operation.parameter("dat").is_some() {
        return Ok(info.operation.clone());
    }

    let fileid = info
        .fileid
        .as_deref()
        .expect("el arranque exige 'dat' o 'fileid'");
    let retrieve_servlet = info
        .retrieve_servlet
        .as_deref()
        .expect("el arranque exige 'rtservlet' cuando trae 'fileid'");

    let downloaded = servlets.retrieve(retrieve_servlet, fileid)?;
    let plain = decrypt(&downloaded, info.key.as_ref())?;
    let data = String::from_utf8(plain)
        .map_err(|error| RelayError::new(RelaySituation::DecryptionFailed, error.to_string()))?;

    Ok(info.operation.clone().with_parameter("dat", data))
}

fn refusal_of(error: RelayError) -> Refusal {
    Refusal::new(code_of_relay(error.situation()), error.detail().to_owned())
}

#[cfg(test)]
mod tests;
