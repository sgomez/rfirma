//! Adaptadores de los motores de filtrado y políticas sobre el puente nativo (ADR-0017).

use crate::signing::adapters::ffi::{BridgeError, ExpandRequest, FilterRequest, NativeBridge};
use crate::signing::adapters::isolate::{Isolate, IsolateGone};

use crate::signing::ports::FilterEngine;
use crate::signing::ports::PolicyEngine;

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
mod tests;
