//! El hilo dueño del isolate de GraalVM (ADR-0003, ADR-0004).

use std::sync::mpsc::{channel, Sender};
use std::thread;

use crate::signing::adapters::ffi::{BridgeError, NativeBridge};

type Job = Box<dyn FnOnce(&Result<NativeBridge, BridgeError>) + Send>;

/// Asa del hilo del isolate.
#[derive(Clone)]
pub struct Isolate {
    jobs: Sender<Job>,
}

/// Error cuando el hilo del isolate ha terminado inesperadamente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsolateGone;

impl std::fmt::Display for IsolateGone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("el hilo de la librería nativa se ha caído")
    }
}

impl std::error::Error for IsolateGone {}

impl Isolate {
    /// Arranca el hilo sin abrir la librería todavía.
    pub fn start() -> Self {
        Self::start_with(NativeBridge::open)
    }

    /// Arranca el hilo con un abridor personalizado para pruebas.
    pub fn start_with(
        open: impl FnOnce() -> Result<NativeBridge, BridgeError> + Send + 'static,
    ) -> Self {
        let (jobs, queue) = channel::<Job>();
        thread::Builder::new()
            .name("rfirma-graal-isolate".to_owned())
            .spawn(move || {
                let mut opener = Some(open);
                let mut bridge: Option<Result<NativeBridge, BridgeError>> = None;
                for job in queue {
                    if bridge.is_none() {
                        bridge = Some(match opener.take() {
                            Some(open) => open(),
                            None => Err(BridgeError::Failed(
                                "la librería nativa ya se intentó abrir".to_owned(),
                            )),
                        });
                    }
                    job(bridge.as_ref().expect("acaba de abrirse"));
                }
            })
            .expect("el sistema debería dejar crear un hilo");
        Self { jobs }
    }

    /// Ejecuta una tarea con el puente nativo en el hilo del isolate.
    pub fn run<T: Send + 'static>(
        &self,
        task: impl FnOnce(&NativeBridge) -> T + Send + 'static,
    ) -> Result<Result<T, BridgeError>, IsolateGone> {
        let (answer, wait) = channel();
        self.jobs
            .send(Box::new(move |bridge| {
                let outcome = match bridge {
                    Ok(bridge) => Ok(task(bridge)),
                    Err(error) => Err(describe(error)),
                };
                let _ = answer.send(outcome);
            }))
            .map_err(|_| IsolateGone)?;
        wait.recv().map_err(|_| IsolateGone)
    }
}

fn describe(error: &BridgeError) -> BridgeError {
    BridgeError::Failed(error.to_string())
}

#[cfg(test)]
mod tests;
