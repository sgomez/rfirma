//! El reloj de inactividad del canal `service`: vence cuando pasa su plazo sin una orden válida en curso.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::{sleep_until, Instant};

use crate::lock;

/// Lo que el canal espera sin órdenes antes de cerrarse (`SOCKET_TIMEOUT`, `ServiceInvocationManager.java:127`).
pub const SOCKET_TIMEOUT: Duration = Duration::from_secs(90);

/// El reloj, compartido entre las conexiones de un mismo canal.
#[derive(Clone)]
pub struct IdleClock {
    shared: Arc<Shared>,
}

struct Shared {
    timeout: Duration,
    ticking: Mutex<Ticking>,
    moved: Notify,
}

struct Ticking {
    orders_in_flight: usize,
    deadline: Instant,
}

/// Una orden válida en curso: el reloj no corre hasta que se suelta (`timer.stop()` y `restart()`).
pub struct OrderInFlight {
    shared: Arc<Shared>,
}

impl IdleClock {
    /// Un reloj que empieza a correr ya.
    pub fn started(timeout: Duration) -> Self {
        Self {
            shared: Arc::new(Shared {
                timeout,
                ticking: Mutex::new(Ticking {
                    orders_in_flight: 0,
                    deadline: Instant::now() + timeout,
                }),
                moved: Notify::new(),
            }),
        }
    }

    /// Para el reloj mientras se atiende una orden válida.
    pub fn order_arrived(&self) -> OrderInFlight {
        lock(&self.shared.ticking).orders_in_flight += 1;
        self.shared.moved.notify_waiters();
        OrderInFlight {
            shared: Arc::clone(&self.shared),
        }
    }

    /// Termina cuando vence el plazo sin ninguna orden válida en curso.
    pub async fn expired(&self) {
        loop {
            let moved = self.shared.moved.notified();
            tokio::pin!(moved);
            moved.as_mut().enable();
            let deadline = {
                let ticking = lock(&self.shared.ticking);
                (ticking.orders_in_flight == 0).then_some(ticking.deadline)
            };
            match deadline {
                Some(deadline) if deadline <= Instant::now() => return,
                Some(deadline) => {
                    tokio::select! {
                        () = sleep_until(deadline) => {}
                        () = &mut moved => {}
                    }
                }
                None => moved.await,
            }
        }
    }
}

impl Drop for OrderInFlight {
    fn drop(&mut self) {
        let mut ticking = lock(&self.shared.ticking);
        ticking.orders_in_flight -= 1;
        if ticking.orders_in_flight == 0 {
            ticking.deadline = Instant::now() + self.shared.timeout;
        }
        drop(ticking);
        self.shared.moved.notify_waiters();
    }
}

#[cfg(test)]
mod tests;
