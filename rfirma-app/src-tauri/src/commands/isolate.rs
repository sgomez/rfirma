//! El hilo que es dueño del isolate de GraalVM.
//!
//! [`NativeBridge`] **no es `Sync`, y es a propósito**: el `IsolateThread` que
//! crea `graal_create_isolate` pertenece al hilo que lo creó, y usarlo desde
//! otro es el segundo fallo silencioso de esa frontera. Las órdenes de Tauri,
//! en cambio, corren en el hilo que les toque del pool.
//!
//! La costura entre las dos cosas es este módulo: un hilo dedicado abre el
//! puente una sola vez y se queda esperando trabajos; las órdenes le mandan un
//! cierre y esperan la respuesta. Así el puente no cruza de hilo nunca, y de
//! propina las dos fases quedan **serializadas**, que es lo que quiere
//! `PadesBridge`: su cerrojo de la zona horaria por defecto es de la JVM entera.
//!
//! El puente se abre **perezosamente**, en el primer trabajo. Abrirlo al
//! arrancar costaría el `dlopen` de 27,7 MB y la creación del isolate a quien
//! solo quiere mirar un PDF, y dejaría la ventana sin abrir si la librería no
//! está —que es justo el caso que hay que contar con un mensaje, no con un
//! arranque fallido—.

use std::sync::mpsc::{channel, Sender};
use std::thread;

use crate::ffi::{BridgeError, NativeBridge};

/// Un trabajo para el hilo del isolate: un cierre que recibe el puente ya
/// abierto y contesta por su propio canal.
type Job = Box<dyn FnOnce(&Result<NativeBridge, BridgeError>) + Send>;

/// El asa del hilo del isolate.
///
/// Clonarla es barato y comparte el mismo hilo: es lo que se guarda en el
/// estado de Tauri.
#[derive(Clone)]
pub struct Isolate {
    jobs: Sender<Job>,
}

/// El hilo del isolate se ha muerto, así que no hay puente ni va a haberlo.
///
/// Solo puede pasar si el hilo entró en pánico, que con el puente detrás no es
/// un caso que se pueda ignorar en silencio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsolateGone;

impl std::fmt::Display for IsolateGone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("el hilo de la librería nativa se ha caído")
    }
}

impl std::error::Error for IsolateGone {}

impl Isolate {
    /// Arranca el hilo. No abre la librería todavía.
    pub fn start() -> Self {
        Self::start_with(NativeBridge::open)
    }

    /// El mismo arranque con el abridor puesto desde fuera, para poder probar
    /// el hilo sin `librfirma_crypto.so` delante.
    pub fn start_with(
        open: impl FnOnce() -> Result<NativeBridge, BridgeError> + Send + 'static,
    ) -> Self {
        let (jobs, queue) = channel::<Job>();
        thread::Builder::new()
            .name("rfirma-graal-isolate".to_owned())
            // El puente se abre dentro del hilo, y **solo** dentro: si se
            // abriera fuera y se mandara aquí, el isolate ya habría cambiado de
            // dueño antes del primer uso.
            .spawn(move || {
                let mut opener = Some(open);
                let mut bridge: Option<Result<NativeBridge, BridgeError>> = None;
                for job in queue {
                    if bridge.is_none() {
                        // Se intenta **una sola vez**: si la librería no está,
                        // no va a aparecer entre dos firmas, y repetir el
                        // `dlopen` en cada orden sería pagarlo para nada.
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

    /// Le pide al hilo que haga algo con el puente y espera el resultado.
    ///
    /// El cierre corre en el hilo del isolate, así que el puente no se mueve de
    /// sitio; lo que viaja es lo que el cierre devuelve.
    pub fn run<T: Send + 'static>(
        &self,
        task: impl FnOnce(&NativeBridge) -> T + Send + 'static,
    ) -> Result<Result<T, BridgeError>, IsolateGone> {
        let (answer, wait) = channel();
        self.jobs
            .send(Box::new(move |bridge| {
                let outcome = match bridge {
                    Ok(bridge) => Ok(task(bridge)),
                    // El fallo de apertura se cuenta tal cual cada vez: la
                    // librería que faltaba en el primer intento sigue faltando.
                    Err(error) => Err(describe(error)),
                };
                let _ = answer.send(outcome);
            }))
            .map_err(|_| IsolateGone)?;
        wait.recv().map_err(|_| IsolateGone)
    }
}

/// Vuelve a contar un fallo de apertura, que no se puede clonar.
///
/// [`BridgeError`] lleva dentro rutas y detalles de `dlopen` y no es `Clone`;
/// el hilo se queda con el original y cada llamada recibe una copia de su
/// texto, que es lo único que se acaba enseñando.
fn describe(error: &BridgeError) -> BridgeError {
    BridgeError::Failed(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::Isolate;
    use crate::ffi::BridgeError;

    /// **Grada A**: se prueba el hilo, no la librería. Con el abridor fallando
    /// no hace falta `librfirma_crypto.so`, y lo que se comprueba —que el
    /// puente no cruza de hilo y que un fallo de apertura se cuenta cada vez—
    /// es exactamente lo que este módulo decide.
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
        // Si se abriera al arrancar, esto ya habría contado uno antes del
        // primer `run`, y abrir cuesta un dlopen de 27,7 MB.
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
        // Es lo que hace falta para vivir en el estado de Tauri: las órdenes
        // corren en el hilo que les toque y el puente no se mueve del suyo.
        let isolate = a_failing_isolate();
        let elsewhere = isolate.clone();

        let joined = std::thread::spawn(move || elsewhere.run(|_| ()).is_ok())
            .join()
            .expect("el hilo de prueba no entra en pánico");

        assert!(joined);
    }
}
