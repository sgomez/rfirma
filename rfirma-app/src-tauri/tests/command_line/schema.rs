//! La validación del `--json` de una orden contra su JSON Schema, la misma en las pruebas unitarias y en las de integración.

use std::path::PathBuf;

use serde_json::Value;

const SCHEMAS_URI: &str = "https://rfirma.sgomez.me/schemas/command-line/";

struct SchemasOnDisk;

impl jsonschema::Retrieve for SchemasOnDisk {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let file = uri
            .as_str()
            .strip_prefix(SCHEMAS_URI)
            .ok_or_else(|| format!("{uri} no es un esquema de la línea de órdenes"))?;
        Ok(schema_in(file))
    }
}

fn schema_in(file: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schemas")
        .join("command-line")
        .join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("no se lee {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} no es JSON: {error}", path.display()))
}

/// El stdout de `<command> --json`, comprobado contra el esquema de la orden: una línea compacta que lo cumple.
pub fn conforming_json(command: &str, stdout: &[u8]) -> Value {
    let schema = schema_in(&format!("{command}.schema.json"));
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .with_retriever(SchemasOnDisk)
        .build(&schema)
        .unwrap_or_else(|error| panic!("el esquema de {command} no compila: {error}"));
    let text = std::str::from_utf8(stdout).expect("stdout es UTF-8");
    let line = text
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("stdout acaba en un salto de línea: {text:?}"));
    assert!(!line.contains('\n'), "stdout es una sola línea: {text:?}");
    let output: Value = serde_json::from_str(line).expect("stdout es JSON");
    let violations: Vec<String> = validator
        .iter_errors(&output)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        violations.is_empty(),
        "{output} no cumple el esquema de {command}: {violations:#?}"
    );
    output
}
