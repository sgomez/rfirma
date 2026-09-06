//! Adaptadores de los motores de filtrado y políticas sobre el puente nativo (ADR-0017).

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
