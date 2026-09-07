//! Las asas opacas de la frontera con la ventana: cómo se acuñan y el mapa de cada asa a lo que nombra (ADR-0011).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static MINTED: AtomicU64 = AtomicU64::new(0);

/// Acuña un identificador opaco aleatorio de 128 bits en hexadecimal (ADR-0011).
pub fn mint() -> String {
    match (getrandom::u64(), getrandom::u64()) {
        (Ok(high), Ok(low)) => format!("{high:016x}{low:016x}"),
        _ => minted_without_the_system_csprng(),
    }
}

fn minted_without_the_system_csprng() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let counter = MINTED.fetch_add(1, Ordering::Relaxed);
    let half = |seed: u64| {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(counter);
        hasher.write_u64(seed);
        hasher.finish()
    };
    format!("{:016x}{:016x}", half(0), half(1))
}

/// Lo que la ventana puede nombrar en esta sesión, cada cosa tras su asa opaca (ADR-0011).
#[derive(Debug)]
pub struct Handles<T> {
    granted: Mutex<HashMap<String, Grant<T>>>,
    order: AtomicU64,
}

#[derive(Debug)]
struct Grant<T> {
    order: u64,
    value: T,
}

impl<T> Default for Handles<T> {
    fn default() -> Self {
        Self {
            granted: Mutex::new(HashMap::new()),
            order: AtomicU64::new(0),
        }
    }
}

impl<T: Clone> Handles<T> {
    /// Un mapa sin asas todavía.
    pub fn new() -> Self {
        Self::default()
    }

    /// Acuña un asa para el valor y la devuelve.
    pub fn mint(&self, value: T) -> String {
        let handle = mint();
        self.grant(handle.clone(), value);
        handle
    }

    /// Sustituye todo lo nombrable por los valores dados y devuelve sus asas, en orden.
    pub fn replace(&self, values: impl IntoIterator<Item = T>) -> Vec<String> {
        lock(&self.granted).clear();
        values.into_iter().map(|value| self.mint(value)).collect()
    }

    /// Lo que hay tras el asa, si sigue siendo nombrable.
    pub fn get(&self, handle: &str) -> Option<T> {
        lock(&self.granted)
            .get(handle)
            .map(|grant| grant.value.clone())
    }

    /// El asa acuñada más tarde entre las que cumplen la condición.
    pub fn last_where(&self, condition: impl Fn(&T) -> bool) -> Option<String> {
        lock(&self.granted)
            .iter()
            .filter(|(_, grant)| condition(&grant.value))
            .max_by_key(|(_, grant)| grant.order)
            .map(|(handle, _)| handle.clone())
    }

    /// Cuántas asas hay.
    pub fn len(&self) -> usize {
        lock(&self.granted).len()
    }

    /// Si no hay ninguna.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn grant(&self, handle: String, value: T) {
        let order = self.order.fetch_add(1, Ordering::Relaxed);
        lock(&self.granted).insert(handle, Grant { order, value });
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests;
