//! Transporte del servidor intermedio: sin canal que sostener, la operación llega ya resuelta desde la propia invocación (ADR-0005, ADR-0017).

use std::sync::Arc;

use crate::site::application::errand::{Inbox, ReplyHandle, Transport};
use crate::site::domain::channel::{
    ChannelDuty, ChannelError, ChannelLocation, Delivery, OpenChannel, Shutdown, Situation,
};
use crate::site::domain::protocol::{
    asks_for_active_wait, check_servlet_url, checked_identifier, decrypt,
    operation_of_the_parameters_xml, AfirmaUrl, CipherKey, Parameter, Refusal, RelayChannelInfo,
    RelayRequest,
};
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
        duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError> {
        let ChannelLocation::Relay(info) = location else {
            return Err(ChannelError::new(
                Situation::Relay,
                "esta ubicacion de canal no es la de un servidor intermedio",
            ));
        };

        // Un rechazo se sube tal cual, sin esperar ni resolver operación: no hay trámite que registrar.
        match duty {
            ChannelDuty::Refuse(answer) => {
                let Some((store_servlet, id)) = info.request.store_target() else {
                    return Err(ChannelError::new(
                        Situation::Relay,
                        "la invocacion aun no dice donde subir la respuesta: 'stservlet' viene \
                         dentro del XML de parametros",
                    ));
                };
                self.upload(store_servlet, id, answer.on_the_wire());
                return Ok(OpenChannel::new(0, Shutdown::of(|| {})));
            }
            ChannelDuty::Serve(_) => {}
        }

        let resolved = resolve(info, self.servlets.as_ref()).map_err(ChannelError::refused)?;

        let servlets = Arc::clone(&self.servlets);
        let exit = Arc::clone(&self.exit);
        let on_upload_failure = Arc::clone(&self.on_upload_failure);
        let store_servlet = resolved.store_servlet;
        let id = resolved.id;
        let reply =
            ReplyHandle::of(
                move |text: String| match servlets.store(&store_servlet, &id, &text) {
                    Ok(()) => exit(),
                    Err(error) => on_upload_failure(refusal_of(error)),
                },
            );

        let inbox = self.inbox.clone();
        let operation = resolved.operation;
        let delivery = Delivery::of(move || inbox.deliver(operation, reply));

        Ok(OpenChannel::with_delivery(0, Shutdown::of(|| {}), delivery))
    }
}

impl Relay {
    fn upload(&self, store_servlet: &str, id: &str, text: String) {
        match self.servlets.store(store_servlet, id, &text) {
            Ok(()) => (self.exit)(),
            Err(error) => (self.on_upload_failure)(refusal_of(error)),
        }
    }
}

/// La operación que hay que atender y adónde va su respuesta, ya sin nada que recuperar.
struct ResolvedRelay {
    operation: AfirmaUrl,
    store_servlet: String,
    id: String,
}

/// Resuelve la operación según de dónde venga: inline, con el documento por `fileid`, o con el
/// XML de parámetros entero por `fileid` (`ProtocolInvocationLauncher`, 1.9.2).
fn resolve(
    info: &RelayChannelInfo,
    servlets: &(dyn Servlets + Send + Sync),
) -> Result<ResolvedRelay, Refusal> {
    match &info.request {
        RelayRequest::Inline { store_servlet, id } => {
            wait_if_asked(info.active_wait, servlets, store_servlet, id)?;
            Ok(ResolvedRelay {
                operation: info.operation.clone(),
                store_servlet: store_servlet.clone(),
                id: id.clone(),
            })
        }
        RelayRequest::DataByFileId {
            store_servlet,
            id,
            fileid,
            retrieve_servlet,
        } => {
            wait_if_asked(info.active_wait, servlets, store_servlet, id)?;
            let document = recovered(servlets, retrieve_servlet, fileid, info.key.as_ref())?;
            Ok(ResolvedRelay {
                operation: info
                    .operation
                    .clone()
                    .with_parameter("dat", text_of(document)?),
                store_servlet: store_servlet.clone(),
                id: id.clone(),
            })
        }
        RelayRequest::ParametersByFileId {
            fileid,
            retrieve_servlet,
        } => {
            let xml = recovered(servlets, retrieve_servlet, fileid, info.key.as_ref())?;
            let operation = operation_of_the_parameters_xml(&xml)?;
            let store_servlet = declared(&operation, "stservlet")?;
            check_servlet_url(&store_servlet, Parameter::StoreServlet)?;
            let id = checked_identifier(declared(&operation, "id")?, Parameter::Identifier)?;
            wait_if_asked(
                asks_for_active_wait(&operation),
                servlets,
                &store_servlet,
                &id,
            )?;
            Ok(ResolvedRelay {
                operation,
                store_servlet,
                id,
            })
        }
    }
}

fn wait_if_asked(
    asked: bool,
    servlets: &(dyn Servlets + Send + Sync),
    store_servlet: &str,
    id: &str,
) -> Result<(), Refusal> {
    if !asked {
        return Ok(());
    }
    servlets.wait(store_servlet, id).map_err(refusal_of)
}

fn recovered(
    servlets: &(dyn Servlets + Send + Sync),
    retrieve_servlet: &str,
    fileid: &str,
    key: Option<&CipherKey>,
) -> Result<Vec<u8>, Refusal> {
    let downloaded = servlets
        .retrieve(retrieve_servlet, fileid)
        .map_err(refusal_of)?;
    decrypt(&downloaded, key).map_err(refusal_of)
}

fn text_of(bytes: Vec<u8>) -> Result<String, Refusal> {
    String::from_utf8(bytes).map_err(|error| {
        refusal_of(RelayError::new(
            RelaySituation::DecryptionFailed,
            error.to_string(),
        ))
    })
}

/// Un parámetro que el XML de parámetros tiene que traer para que haya adónde contestar.
fn declared(operation: &AfirmaUrl, name: &str) -> Result<String, Refusal> {
    operation
        .parameter(name)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            Refusal::params(format!(
                "el XML de parametros del servidor intermedio no trae '{name}'"
            ))
        })
}

fn refusal_of(error: RelayError) -> Refusal {
    Refusal::new(code_of_relay(error.situation()), error.detail().to_owned())
}

#[cfg(test)]
mod tests;
