//! El hilo dueño del isolate de GraalVM (ADR-0003, ADR-0004).

use std::sync::mpsc::{channel, Sender};
use std::thread;

use crate::ffi::{BridgeError, NativeBridge};

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
mod tests {
    use super::Isolate;
    use crate::ffi::BridgeError;

    fn a_failing_isolate() -> Isolate {
        Isolate::start_with(|| Err(BridgeError::Failed("no hay librería".to_owned())))
    }

    #[test]
    fn a_failure_to_open_is_told_to_every_caller_and_not_only_to_the_first() {
        let isolate = a_failing_isolate();

        for _ in 0..3 {
            let answer = isolate.run(|_| ()).expect("el hilo sigue vivo");
            assert!(answer.is_err(), "la librería sigue sin estar");
        }
    }

    #[test]
    fn the_library_is_opened_lazily_and_only_once() {
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = std::sync::Arc::clone(&attempts);
        let isolate = Isolate::start_with(move || {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(BridgeError::Failed("no hay librería".to_owned()))
        });

        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 0);
        for _ in 0..3 {
            let _ = isolate.run(|_| ());
        }
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn the_handle_can_be_shared_between_threads() {
        let isolate = a_failing_isolate();
        let elsewhere = isolate.clone();

        let joined = std::thread::spawn(move || elsewhere.run(|_| ()).is_ok())
            .join()
            .expect("el hilo de prueba no entra en pánico");

        assert!(joined);
    }
}
