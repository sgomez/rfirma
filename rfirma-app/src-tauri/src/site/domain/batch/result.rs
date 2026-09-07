//! El resultado del lote cuando la prefirma no dio ninguna firma que postfirmar (`JSONBatchInfoParser`, 1.9.2).

use super::json::Json;
use super::presign::BatchDataResult;

/// El resultado del lote con los errores individuales que impidieron prefirmar nada.
pub fn build_result(errors: &[BatchDataResult]) -> Vec<u8> {
    let mut signs = Vec::with_capacity(errors.len());
    for result in errors {
        let mut object = vec![
            ("id".to_owned(), Json::String(result.id().to_owned())),
            (
                "result".to_owned(),
                Json::String(result.result().as_str().to_owned()),
            ),
        ];
        if let Some(description) = result.description() {
            object.push((
                "description".to_owned(),
                Json::String(description.to_owned()),
            ));
        }
        signs.push(Json::Object(object));
    }

    Json::Object(vec![("signs".to_owned(), Json::Array(signs))])
        .to_json_string()
        .into_bytes()
}

/// El resultado del lote cuando la prefirma no devolvió ni firmas ni errores.
pub fn build_empty_result() -> Vec<u8> {
    Json::Object(vec![("signs".to_owned(), Json::Array(Vec::new()))])
        .to_json_string()
        .into_bytes()
}

#[cfg(test)]
mod tests;
