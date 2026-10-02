//! El `-config` de `sign`: las propiedades de la firma, leídas con las reglas de las `properties` de una sede.

use std::collections::BTreeMap;

use crate::site::domain::protocol::{
    pairs_of, visible_signature_of, without_the_launcher_keys, IfCancelled, SiteVisibleSignature,
};

/// Los parámetros de la firma que declara `-config`, o por qué no se aceptan.
pub fn parameters_of(config: Option<&str>) -> Result<BTreeMap<String, String>, String> {
    let Some(config) = config else {
        return Ok(BTreeMap::new());
    };
    let parameters: BTreeMap<String, String> =
        without_the_launcher_keys(pairs_of(&with_the_escaped_line_breaks_broken(config)))
            .into_iter()
            .collect();
    match visible_signature_of(&parameters) {
        Err(refusal) => Err(refusal.detail().to_owned()),
        Ok(SiteVisibleSignature::MarkedByThePerson(IfCancelled::Refuses)) => Err(
            "'visibleSignature=want' sin posición ni página pide marcar el área, \
             y la línea de órdenes no tiene ventana en la que marcarla"
                .to_owned(),
        ),
        Ok(_) => Ok(parameters),
    }
}

/// El original parte `-config` por el `\n` literal que deja la consola, no por el salto de línea.
fn with_the_escaped_line_breaks_broken(config: &str) -> String {
    config.replace("\\n", "\n")
}

#[cfg(test)]
mod tests;
