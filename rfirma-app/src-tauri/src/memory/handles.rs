//! Acuñado de identificadores opacos para la frontera con la interfaz (ADR-0011).

use std::sync::atomic::{AtomicU64, Ordering};

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

#[cfg(test)]
mod tests;
