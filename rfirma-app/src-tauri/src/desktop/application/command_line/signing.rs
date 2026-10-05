//! La firma de `sign` y `cosign` por la línea de órdenes, y lo que sale por stdout de ella según el documento que se pida.

use base64::Engine as _;

use super::*;

pub(super) fn sign(
    arguments: &[String],
    parsed: &SignArguments,
    operation: SignatureOperation,
    ports: &CommandLinePorts,
) -> Outcome {
    let asked = document_asked_by(arguments);
    match (signed(arguments, parsed, operation, ports), asked) {
        (Err(failed), Some(Asked::Xml)) => in_the_xml_response(failed, None),
        (Err(failed), _) => failed,
        (Ok(signed), Some(Asked::Json)) => Outcome {
            stdout: json_output::compact(&signed.as_json()),
            ..signed.outcome()
        },
        (Ok(signed), Some(Asked::Xml)) => {
            in_the_xml_response(signed.outcome(), signed.returned_document())
        }
        (Ok(signed), None) => signed.outcome(),
    }
}

/// Una firma que salió bien: con qué certificado, en qué formato y adónde fue.
struct Signed {
    certificate: TokenCertificate,
    format: SignatureFormat,
    destination: Destination,
}

enum Destination {
    File(String),
    Returned(Vec<u8>),
}

impl Signed {
    fn outcome(&self) -> Outcome {
        let message = match &self.destination {
            Destination::File(output) => format!("rfirma: firma guardada en «{output}»"),
            Destination::Returned(_) => "rfirma: firma generada".to_owned(),
        };
        Outcome {
            exit_code: SUCCEEDED,
            stdout: Vec::new(),
            stderr: vec![message],
        }
    }

    fn returned_document(&self) -> Option<&[u8]> {
        match &self.destination {
            Destination::File(_) => None,
            Destination::Returned(document) => Some(document),
        }
    }

    fn as_json(&self) -> SignedDocument {
        match &self.destination {
            Destination::File(output) => {
                SignedDocument::written_to(&self.certificate, self.format, Path::new(output))
            }
            Destination::Returned(document) => {
                SignedDocument::returned(&self.certificate, self.format, document)
            }
        }
    }
}

fn signed(
    arguments: &[String],
    parsed: &SignArguments,
    operation: SignatureOperation,
    ports: &CommandLinePorts,
) -> Result<Signed, Outcome> {
    let Some(selection) = &parsed.selection else {
        return Err(Outcome::not_yet_available(
            "elegir el certificado sin --alias",
        ));
    };
    let parameters = config::parameters_of(parsed.config.as_deref())
        .map_err(|reason| Outcome::failed(format!("rfirma: --config no se acepta ({reason})")))?;
    let input = Path::new(&parsed.input);
    let bytes = ports.files.read(input).map_err(|reason| {
        Outcome::failed(format!(
            "rfirma: no se puede leer «{}» ({reason})",
            parsed.input
        ))
    })?;
    let format = signature_format_of(parsed.format, &bytes);
    let (certificate, typed_in_the_window) =
        the_certificate_chosen_by(selection, input, arguments, ports)?;
    let document = ports
        .signer
        .sign(&CommandLineSigning {
            input,
            certificate: &certificate,
            format,
            operation,
            algorithm: parsed.algorithm,
            terminal: ports.terminal,
            parameters: &parameters,
            document_length: bytes.len(),
            password_fd: parsed.password_fd,
            descriptor: ports.descriptor,
            typed_in_the_window: typed_in_the_window.as_ref(),
        })
        .map_err(|reason| Outcome::failed(format!("rfirma: no se ha podido firmar ({reason})")))?;
    let destination = match &parsed.output {
        Some(output) => {
            ports
                .files
                .write(Path::new(output), &document)
                .map_err(|reason| {
                    Outcome::failed(format!(
                        "rfirma: no se puede escribir «{output}» ({reason})"
                    ))
                })?;
            Destination::File(output.clone())
        }
        None => Destination::Returned(document),
    };
    ports.signer.remember(&certificate);
    Ok(Signed {
        certificate,
        format,
        destination,
    })
}

pub(super) fn in_the_xml_response(outcome: Outcome, signature: Option<&[u8]>) -> Outcome {
    Outcome {
        stdout: response_of(&outcome, signature).to_xml(),
        ..outcome
    }
}

fn response_of(outcome: &Outcome, signature: Option<&[u8]>) -> Response {
    let message = outcome
        .stderr
        .iter()
        .map(|line| line.strip_prefix("rfirma: ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join(" ");
    let mut fields = vec![Field::One("msg", message)];
    if let Some(bytes) = signature {
        fields.push(Field::One(
            "sign",
            base64::engine::general_purpose::STANDARD.encode(bytes),
        ));
    }
    let result = if outcome.exit_code == SUCCEEDED {
        "true"
    } else {
        "false"
    };
    Response::new(result, fields)
}

fn signature_format_of(asked: Format, bytes: &[u8]) -> SignatureFormat {
    match asked {
        Format::Pades => SignatureFormat::Pades,
        Format::Cades => SignatureFormat::Cades,
        Format::Xades => SignatureFormat::Xades(XadesVariant::Enveloping),
        Format::Auto => match shape_of(bytes) {
            DetectedShape::Pdf => SignatureFormat::Pades,
            DetectedShape::Xml | DetectedShape::Invoice => {
                SignatureFormat::Xades(XadesVariant::Enveloping)
            }
            DetectedShape::Binary => SignatureFormat::Cades,
        },
    }
}
