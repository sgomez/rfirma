//! Transporte del servidor intermedio: sin canal que sostener, la operación llega ya resuelta desde la propia invocación (ADR-0005, ADR-0017).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::oneshot;

use crate::site::application::errand::{Acknowledgement, Inbox, ReplyHandle, Transport};
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

const ACTIVE_WAIT_INTERVAL: Duration = Duration::from_secs(10);

/// El servidor intermedio: descarga y descifra la petición una vez, la entrega por el mismo
/// buzón que el `wss`, y su asa de respuesta sube la contestación cifrada.
pub struct Relay {
    servlets: Arc<dyn Servlets + Send + Sync>,
    inbox: Inbox,
    on_upload_failure: Arc<dyn Fn(Refusal) + Send + Sync>,
}

impl Relay {
    /// Un transporte de servidor intermedio sobre los servlets, el buzón y el aviso que se da
    /// cuando la subida se rechaza.
    pub fn new(
        servlets: Arc<dyn Servlets + Send + Sync>,
        inbox: Inbox,
        on_upload_failure: Arc<dyn Fn(Refusal) + Send + Sync>,
    ) -> Self {
        Self {
            servlets,
            inbox,
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

        // Un rechazo se sube tal cual, sin esperar ni resolver operación: no hay trámite que
        // registrar. La subida no ocurre aquí: se deja como entrega pendiente y la dispara quien
        // atiende la invocación, con la llegada ya inmediata.
        match duty {
            ChannelDuty::Refuse(answer) => {
                let Some((store_servlet, id)) = info.request.store_target() else {
                    return Err(ChannelError::new(
                        Situation::Relay,
                        "la invocacion aun no dice donde subir la respuesta: 'stservlet' viene \
                         dentro del XML de parametros",
                    ));
                };
                let servlets = Arc::clone(&self.servlets);
                let on_upload_failure = Arc::clone(&self.on_upload_failure);
                let store_servlet = store_servlet.to_owned();
                let id = id.to_owned();
                let text = answer.on_the_wire();
                let delivery = Delivery::fallible(move || {
                    upload_answer(
                        servlets.as_ref(),
                        on_upload_failure.as_ref(),
                        &store_servlet,
                        &id,
                        &text,
                    )
                });
                return Ok(OpenChannel::with_delivery(0, Shutdown::of(|| {}), delivery));
            }
            ChannelDuty::Serve(_) => {}
        }

        let resolved =
            resolve(info, self.servlets.as_ref()).map_err(|failure| match failure.destination {
                Some((store_servlet, id)) => {
                    ChannelError::refused_at(failure.refusal, store_servlet, id)
                }
                None => ChannelError::refused(failure.refusal),
            })?;

        let heartbeat = spawn_heartbeat(
            resolved.active_wait,
            Arc::clone(&self.servlets),
            &resolved.store_servlet,
            &resolved.id,
        );

        let shutdown_heartbeat = heartbeat.clone();
        let shutdown = Shutdown::of(move || {
            if let Some(hb) = shutdown_heartbeat {
                hb.stop();
            }
        });

        let servlets = Arc::clone(&self.servlets);
        let on_upload_failure = Arc::clone(&self.on_upload_failure);
        let store_servlet = resolved.store_servlet;
        let id = resolved.id;
        let reply_heartbeat = heartbeat;
        let reply = ReplyHandle::of(move |text: String| {
            if let Some(hb) = &reply_heartbeat {
                hb.stop();
            }
            let _guard = reply_heartbeat.as_ref().map(|hb| hb.0.upload_lock.lock());
            match servlets.store(&store_servlet, &id, &text) {
                Ok(()) => Acknowledgement::immediate(),
                Err(error) => {
                    on_upload_failure(refusal_of(error));
                    Acknowledgement::never()
                }
            }
        });

        let inbox = self.inbox.clone();
        let operation = resolved.operation;
        let delivery = Delivery::of(move || inbox.deliver(operation, reply));

        Ok(OpenChannel::with_delivery(0, shutdown, delivery))
    }
}

/// Sube el rechazo o la respuesta ya codificada; `false`, tras avisar del fallo, si el servlet la rechaza.
fn upload_answer(
    servlets: &(dyn Servlets + Send + Sync),
    on_upload_failure: &(dyn Fn(Refusal) + Send + Sync),
    store_servlet: &str,
    id: &str,
    text: &str,
) -> bool {
    match servlets.store(store_servlet, id, text) {
        Ok(()) => true,
        Err(error) => {
            on_upload_failure(refusal_of(error));
            false
        }
    }
}

struct HeartbeatInner {
    stop: Mutex<Option<oneshot::Sender<()>>>,
    upload_lock: Mutex<()>,
}

#[derive(Clone)]
struct Heartbeat(Arc<HeartbeatInner>);

impl Heartbeat {
    fn new(stop: oneshot::Sender<()>) -> Self {
        Self(Arc::new(HeartbeatInner {
            stop: Mutex::new(Some(stop)),
            upload_lock: Mutex::new(()),
        }))
    }

    fn stop(&self) {
        if let Ok(mut lock) = self.0.stop.lock() {
            if let Some(tx) = lock.take() {
                let _ = tx.send(());
            }
        }
    }
}

impl Drop for HeartbeatInner {
    fn drop(&mut self) {
        if let Ok(mut lock) = self.stop.lock() {
            if let Some(tx) = lock.take() {
                let _ = tx.send(());
            }
        }
    }
}

fn spawn_heartbeat(
    active_wait: bool,
    servlets: Arc<dyn Servlets + Send + Sync>,
    store_servlet: &str,
    id: &str,
) -> Option<Heartbeat> {
    if !active_wait {
        return None;
    }
    let (stop_tx, stop_rx) = oneshot::channel();
    let heartbeat = Heartbeat::new(stop_tx);
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(pulse_active_wait(
            servlets,
            store_servlet.to_owned(),
            id.to_owned(),
            heartbeat.clone(),
            stop_rx,
        ));
    }
    Some(heartbeat)
}

async fn pulse_active_wait(
    servlets: Arc<dyn Servlets + Send + Sync>,
    store_servlet: String,
    id: String,
    heartbeat: Heartbeat,
    mut stopped: oneshot::Receiver<()>,
) {
    let mut interval = tokio::time::interval_at(
        tokio::time::Instant::now() + ACTIVE_WAIT_INTERVAL,
        ACTIVE_WAIT_INTERVAL,
    );
    loop {
        tokio::select! {
            _ = &mut stopped => break,
            _ = interval.tick() => {
                let Ok(_guard) = heartbeat.0.upload_lock.lock() else {
                    break;
                };
                if !matches!(stopped.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Empty)) {
                    break;
                }
                let _ = servlets.wait(&store_servlet, &id);
            }
        }
    }
}

/// La operación que hay que atender y adónde va su respuesta, ya sin nada que recuperar.
struct ResolvedRelay {
    operation: AfirmaUrl,
    store_servlet: String,
    id: String,
    active_wait: bool,
}

/// Un fallo al resolver la operación, con el destino de subida cuando ya se conocía al fallar.
struct ResolutionFailure {
    refusal: Refusal,
    destination: Option<(String, String)>,
}

impl ResolutionFailure {
    fn without_destination(refusal: Refusal) -> Self {
        Self {
            refusal,
            destination: None,
        }
    }

    fn at(refusal: Refusal, store_servlet: &str, id: &str) -> Self {
        Self {
            refusal,
            destination: Some((store_servlet.to_owned(), id.to_owned())),
        }
    }
}

/// Resuelve la operación según de dónde venga: inline, con el documento por `fileid`, o con el
/// XML de parámetros entero por `fileid` (`ProtocolInvocationLauncher`, 1.9.2).
fn resolve(
    info: &RelayChannelInfo,
    servlets: &(dyn Servlets + Send + Sync),
) -> Result<ResolvedRelay, ResolutionFailure> {
    match &info.request {
        RelayRequest::Inline { store_servlet, id } => {
            wait_if_asked(info.active_wait, servlets, store_servlet, id)
                .map_err(|refusal| ResolutionFailure::at(refusal, store_servlet, id))?;
            Ok(ResolvedRelay {
                operation: info.operation.clone(),
                store_servlet: store_servlet.clone(),
                id: id.clone(),
                active_wait: info.active_wait,
            })
        }
        RelayRequest::DataByFileId {
            store_servlet,
            id,
            fileid,
            retrieve_servlet,
        } => {
            wait_if_asked(info.active_wait, servlets, store_servlet, id)
                .map_err(|refusal| ResolutionFailure::at(refusal, store_servlet, id))?;
            let document = recovered(servlets, retrieve_servlet, fileid, info.key.as_ref())
                .map_err(|refusal| ResolutionFailure::at(refusal, store_servlet, id))?;
            let dat = text_of(document)
                .map_err(|refusal| ResolutionFailure::at(refusal, store_servlet, id))?;
            Ok(ResolvedRelay {
                operation: info.operation.clone().with_parameter("dat", dat),
                store_servlet: store_servlet.clone(),
                id: id.clone(),
                active_wait: info.active_wait,
            })
        }
        RelayRequest::ParametersByFileId {
            fileid,
            retrieve_servlet,
        } => {
            let xml = recovered(servlets, retrieve_servlet, fileid, info.key.as_ref())
                .map_err(ResolutionFailure::without_destination)?;
            let operation = operation_of_the_parameters_xml(&xml)
                .map_err(ResolutionFailure::without_destination)?;
            let store_servlet = declared(&operation, "stservlet")
                .map_err(ResolutionFailure::without_destination)?;
            check_servlet_url(&store_servlet, Parameter::StoreServlet)
                .map_err(ResolutionFailure::without_destination)?;
            let id = declared(&operation, "id").map_err(ResolutionFailure::without_destination)?;
            let id = checked_identifier(id, Parameter::Identifier)
                .map_err(ResolutionFailure::without_destination)?;
            let active_wait = asks_for_active_wait(&operation);
            wait_if_asked(active_wait, servlets, &store_servlet, &id)
                .map_err(|refusal| ResolutionFailure::at(refusal, &store_servlet, &id))?;
            Ok(ResolvedRelay {
                operation,
                store_servlet,
                id,
                active_wait,
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
    if downloaded.len() > 8 && downloaded.to_ascii_lowercase().starts_with("err-") {
        return Err(refusal_of(RelayError::new(
            RelaySituation::ServletUnreachable,
            downloaded.trim().to_owned(),
        )));
    }
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
