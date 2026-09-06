//! **Los dos motores del puente, servidos desde el hilo del aislado** (RD-06,
//! ID-252, ID-266).
//!
//! [`FilterEngine`] y [`PolicyEngine`] se declaran una sola vez, en
//! [`super::filtering`] y [`super::policies`]; aquí viven sus adaptadores
//! sobre el puente nativo, y sólo aquí. Son dos capas de la misma cosa:
//!
//! - **sobre [`NativeBridge`]**, que es quien sabe filtrar y expandir
//!   políticas y que sólo existe dentro del hilo del aislado;
//! - **sobre [`Isolate`]**, que es lo que tiene a mano una orden: cada llamada
//!   se manda al hilo, y **la doble `Result` del aislado se resuelve aquí**
//!   —el hilo que ya no está es el puente que no contesta, y para el trámite
//!   es la firma que no sale—, no en la orden.
//!
//! Vive en `app/` y no junto a `isolate.rs` porque la flecha va hacia dentro
//! (ADR-0017): un módulo de infraestructura no nombra a `app/`, y el puerto se
//! declara en `app/`.

use crate::ffi::{BridgeError, ExpandRequest, FilterRequest, NativeBridge};
use crate::isolate::{Isolate, IsolateGone};

use super::filtering::FilterEngine;
use super::policies::PolicyEngine;

impl FilterEngine for NativeBridge {
    fn select(
        &self,
        filter_properties: &str,
        certificates_b64: &str,
    ) -> Result<Vec<usize>, BridgeError> {
        self.filter_certificates(FilterRequest {
            filter_properties,
            certificates_b64,
        })
    }
}

impl PolicyEngine for NativeBridge {
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError> {
        self.expand_extra_params(ExpandRequest {
            extra_params,
            format,
        })
    }
}

/// Lo que devuelve el hilo del aislado, aplanado: el hilo que ya no está es el
/// puente que no contesta.
fn ran<T: Send + 'static>(
    outcome: Result<Result<T, BridgeError>, IsolateGone>,
) -> Result<T, BridgeError> {
    outcome.unwrap_or_else(|_| {
        Err(BridgeError::Failed(
            "el hilo del isolate ya no esta".to_owned(),
        ))
    })
}

impl FilterEngine for Isolate {
    fn select(
        &self,
        filter_properties: &str,
        certificates_b64: &str,
    ) -> Result<Vec<usize>, BridgeError> {
        let properties = filter_properties.to_owned();
        let certificates = certificates_b64.to_owned();
        ran(self.run(move |bridge| FilterEngine::select(bridge, &properties, &certificates)))?
    }
}

impl PolicyEngine for Isolate {
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError> {
        let declared = extra_params.to_owned();
        let format = format.to_owned();
        ran(self.run(move |bridge| PolicyEngine::expand(bridge, &declared, &format)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La doble `Result` se resuelve aquí y no en la orden**: un aislado
    /// cuyo puente no abre contesta con la situación del puente, y una orden no
    /// tiene que saber que hay un hilo en medio.
    #[test]
    fn a_bridge_that_does_not_open_is_a_bridge_error_and_not_a_thread_error() {
        let isolate =
            Isolate::start_with(|| Err(BridgeError::Failed("no hay libreria".to_owned())));

        let refused = FilterEngine::select(&isolate, "", "").expect_err("el puente no abre");
        assert!(matches!(refused, BridgeError::Failed(_)), "{refused:?}");

        let refused = PolicyEngine::expand(&isolate, "", "pades").expect_err("el puente no abre");
        assert!(matches!(refused, BridgeError::Failed(_)), "{refused:?}");
    }
}
