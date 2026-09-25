//! El testigo: lo que la sesión le pide a la persona que está delante mientras corre una
//! comprobación, sin decir por dónde se lo pide.

use std::time::Instant;

use crate::catalogue::Assistance;
use crate::livelog::{LiveLogSink, Provenance};

/// El seam entre quien corre las comprobaciones y la persona; la consola es su adaptador real.
pub(crate) trait Witness: Send + Sync {
    fn started_at(&self) -> Instant;

    /// Por donde llegan las líneas de la comprobación en curso.
    fn log_sink(&self) -> LiveLogSink;

    /// Cuenta a la persona lo que va a pasar y espera a que dé paso; `false` si lo salta.
    fn brief(&self, check: &str, briefing: &str) -> bool;

    /// Para la cola antes de abrir `tranche` hasta que la persona dice que está; `false` si no.
    fn stand_by(&self, check: &str, tranche: Assistance) -> bool;

    /// Avisa de un fallo de la suite, no del cliente, en la comprobación `check`.
    fn suite_failure(&self, check: &str, why: &str);

    fn driver_spawned(&self, pid: u32);

    fn driver_finished(&self);

    fn aborted(&self) -> bool;

    /// Un diagnóstico de la suite, en el registro de la comprobación en curso.
    fn harness(&self, text: &str) {
        self.log_sink()
            .push(Provenance::Suite, self.started_at().elapsed(), text);
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use std::sync::Mutex;
    use std::time::Instant;

    use super::Witness;
    use crate::catalogue::Assistance;
    use crate::livelog::LiveLogSink;

    /// Un testigo que da paso siempre, salvo que se le diga que no está, y apunta lo que se le pidió.
    #[derive(Default)]
    pub(crate) struct FakeWitness {
        pub(crate) refuses_to_stand_by: bool,
        pub(crate) said: Mutex<Vec<String>>,
    }

    impl FakeWitness {
        pub(crate) fn said(&self) -> Vec<String> {
            self.said.lock().unwrap().clone()
        }

        fn note(&self, what: String) {
            self.said.lock().unwrap().push(what);
        }
    }

    impl Witness for FakeWitness {
        fn started_at(&self) -> Instant {
            Instant::now()
        }

        fn log_sink(&self) -> LiveLogSink {
            LiveLogSink::new(|_| {})
        }

        fn brief(&self, check: &str, _briefing: &str) -> bool {
            self.note(format!("brief {check}"));
            true
        }

        fn stand_by(&self, check: &str, tranche: Assistance) -> bool {
            self.note(format!("stand_by {check} {}", tranche.name()));
            !self.refuses_to_stand_by
        }

        fn suite_failure(&self, check: &str, _why: &str) {
            self.note(format!("suite_failure {check}"));
        }

        fn driver_spawned(&self, _pid: u32) {}

        fn driver_finished(&self) {}

        fn aborted(&self) -> bool {
            false
        }
    }
}
