//! **Cómo se acuña un asa opaca** (ID-61, ADR-0011).
//!
//! Un asa es lo que cruza a la ventana en lugar de una ruta del anfitrión: los
//! documentos abiertos ([`super::opened`]) y los certificados listados
//! ([`super::listed`]) tienen cada uno su tabla, y las dos acuñan aquí. Está en
//! su propio módulo porque la propiedad que importa —**del asa no sale nada de
//! lo que representa**— es una sola y no puede tener dos implementaciones que
//! diverjan.

use std::sync::atomic::{AtomicU64, Ordering};

/// Cuántas asas se han acuñado en este proceso. Solo lo usa el amasado de
/// reserva de [`minted_without_the_system_csprng`], para que dos acuñadas en el
/// mismo instante no puedan colisionar.
static MINTED: AtomicU64 = AtomicU64::new(0);

/// Acuña un identificador opaco de 128 bits, en hexadecimal.
///
/// Los 128 bits salen del **CSPRNG del sistema** (`getrandom`), y **no se
/// derivan de lo que nombran**. Derivarlo —un hash de la ruta, o del módulo
/// PKCS#11 y la etiqueta— dejaría que la ventana comprobara candidatos por
/// fuerza bruta contra el asa, que es exactamente la fuga que el ADR-0011
/// cierra.
///
/// No sirve amasarlo con `RandomState`: `std` siembra sus claves una vez por
/// hilo y cada `RandomState::new()` posterior se limita a incrementar una, así
/// que todas las asas de la sesión saldrían de la misma semilla más un contador
/// y dos consecutivas no serían independientes.
pub fn mint() -> String {
    match (getrandom::u64(), getrandom::u64()) {
        (Ok(high), Ok(low)) => format!("{high:016x}{low:016x}"),
        _ => minted_without_the_system_csprng(),
    }
}

/// Cuando el CSPRNG del sistema no responde —no debería pasar en Linux, pero
/// `getrandom` puede fallar— se vuelve al amasado de `RandomState` más un
/// contador de proceso.
///
/// Es peor —misma semilla por hilo, así que la entropía no crece con cada
/// acuñado— pero mantiene lo que de verdad importa aquí: **sigue sin llevar
/// nada de lo que nombra dentro** y sigue sin repetirse dentro del proceso, que
/// es lo que la tabla necesita. Un `panic!` en su lugar tumbaría la orden que
/// abre el documento por un fallo del que no se recupera nadie.
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
mod tests {
    use super::*;

    /// **Grada A**: acuñar es puro.
    #[test]
    fn a_handle_is_thirty_two_hexadecimal_digits() {
        let handle = mint();

        assert_eq!(handle.len(), 32);
        assert!(handle
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn two_handles_are_never_the_same() {
        assert_ne!(mint(), mint());
    }

    /// El camino de reserva tiene que cumplir lo mismo que el bueno: es el que
    /// corre cuando el CSPRNG del sistema no responde.
    #[test]
    fn the_fallback_keeps_the_shape_and_the_difference() {
        let first = minted_without_the_system_csprng();
        let second = minted_without_the_system_csprng();

        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }
}
