//! La transcripción de un caso del sondeo: los eventos del cliente publicado, legibles y en
//! disco a medida que llegan, para que un trámite que no termina deje transcrito hasta dónde
//! llegó.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::Value;

pub struct Transcript {
    writer: BufWriter<File>,
}

impl Transcript {
    /// Abre la transcripción de `case` junto al expediente `dossier`.
    pub fn open(dossier: &Path, case: &str) -> Result<Self, String> {
        let directory = transcripts_directory_of(dossier);
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("{} no se pudo crear: {error}", directory.display()))?;
        let path = directory.join(format!("{case}.jsonl"));
        let file = File::create(&path)
            .map_err(|error| format!("{} no se pudo crear: {error}", path.display()))?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    /// Añade `raw_event` a la transcripción, ya legible, y lo deja en disco antes de devolver
    /// el control.
    pub fn record(&mut self, raw_event: &str) -> Result<(), String> {
        writeln!(self.writer, "{}", legible(raw_event))
            .and_then(|()| self.writer.flush())
            .map_err(|error| format!("la transcripción no se pudo escribir: {error}"))
    }
}

fn transcripts_directory_of(dossier: &Path) -> PathBuf {
    let mut directory = dossier.as_os_str().to_owned();
    directory.push(".transcripts");
    PathBuf::from(directory)
}

/// `raw_event` con cada campo en base64 vuelto a su texto, o anotado como binario si no lo es.
fn legible(raw_event: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(raw_event) else {
        return raw_event.to_owned();
    };
    if let Value::Object(fields) = &mut value {
        for field in fields.values_mut() {
            if let Value::String(text) = field {
                if let Some(decoded) = decoded_base64_text(text) {
                    *field = Value::String(decoded);
                }
            }
        }
    }
    value.to_string()
}

fn decoded_base64_text(candidate: &str) -> Option<String> {
    if candidate.len() < 8 {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(candidate)
        .ok()?;
    match String::from_utf8(bytes) {
        Ok(text) => Some(text),
        Err(error) => Some(format!("<binario, {} bytes>", error.into_bytes().len())),
    }
}
