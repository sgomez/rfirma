//! Adaptador `reqwest` sobre los dos servlets del lote remoto: prefirma y postfirma (`BatchSigner`, 1.9.2).

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine as _;

use crate::site::domain::batch::{BatchFormat, TriphaseData};
use crate::site::domain::batch_error::{BatchError, Situation};
use crate::site::ports::BatchServices;

/// Los dos servlets del lote remoto de producción, sobre `reqwest::blocking` con la validación TLS del sistema.
pub struct RelayBatchServices {
    client: reqwest::blocking::Client,
}

impl Default for RelayBatchServices {
    fn default() -> Self {
        Self {
            client: super::service_client(),
        }
    }
}

impl RelayBatchServices {
    fn post(
        &self,
        url: reqwest::Url,
        body: String,
        unreachable: Situation,
        invalid: Situation,
    ) -> Result<Vec<u8>, BatchError> {
        let client = self.client.clone();
        execute_outside_tokio(move || {
            let response = client
                .post(url)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(body)
                .send()
                .map_err(|error| BatchError::new(unreachable, error.to_string()))?;
            let status = response.status();
            if super::is_rejection(status) {
                return Err(BatchError::answered(
                    invalid,
                    status.as_u16(),
                    super::rejection_detail(response),
                ));
            }

            response
                .bytes()
                .map(|bytes| bytes.to_vec())
                .map_err(|error| BatchError::new(invalid, error.to_string()))
        })
    }
}

fn execute_outside_tokio<T: Send + 'static>(action: impl FnOnce() -> T + Send + 'static) -> T {
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::Builder::new()
            .name("batch-service-io".into())
            .spawn(action)
            .expect("el hilo para la llamada HTTP del lote remoto se crea")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    } else {
        action()
    }
}

impl BatchServices for RelayBatchServices {
    fn presign(
        &self,
        url: &str,
        format: BatchFormat,
        lote_base64: &str,
        certs: &[Vec<u8>],
    ) -> Result<Vec<u8>, BatchError> {
        let url = parsed_batch_url(url, Situation::PresignerUnreachable)?;
        self.post(
            url,
            compose_body(format, lote_base64, certs, None),
            Situation::PresignerUnreachable,
            Situation::InvalidPresignResponse,
        )
    }

    fn postsign(
        &self,
        url: &str,
        format: BatchFormat,
        lote_base64: &str,
        certs: &[Vec<u8>],
        tridata: &TriphaseData,
    ) -> Result<Vec<u8>, BatchError> {
        let url = parsed_batch_url(url, Situation::PostsignerUnreachable)?;
        self.post(
            url,
            compose_body(format, lote_base64, certs, Some(tridata)),
            Situation::PostsignerUnreachable,
            Situation::InvalidPostsignResponse,
        )
    }
}

/// El cuerpo del POST como `BatchSigner`, en formulario y no en la query: `xml|json`, `certs` en base64 URL-safe separados por `;`, y `tridata` en la postfirma (`getCertChainAsBase64`, 1.9.2).
fn compose_body(
    format: BatchFormat,
    lote_base64: &str,
    certs: &[Vec<u8>],
    tridata: Option<&TriphaseData>,
) -> String {
    let certs_base64 = certs
        .iter()
        .map(|cert| URL_SAFE.encode(cert))
        .collect::<Vec<_>>()
        .join(";");

    let lote_base64 = url_safe_batch(lote_base64);
    let mut query = format!("{}={lote_base64}&certs={certs_base64}", format.param_name());

    if let Some(tridata) = tridata {
        let serialized = match format {
            BatchFormat::Xml => tridata.to_xml(),
            BatchFormat::Json => tridata.to_json(),
        };
        query.push_str("&tridata=");
        query.push_str(&URL_SAFE.encode(serialized.as_bytes()));
    }

    query
}

/// `BatchSigner` no recodifica el lote: sustituye `+`→`-` y `/`→`_` sobre el base64 que llegó en `dat`.
fn url_safe_batch(lote_base64: &str) -> String {
    lote_base64.replace('+', "-").replace('/', "_")
}

/// La URL del servlet del lote ya leída; su forma la comprobó el dominio al leer la operación.
fn parsed_batch_url(url: &str, unreachable: Situation) -> Result<reqwest::Url, BatchError> {
    reqwest::Url::parse(url).map_err(|error| BatchError::new(unreachable, error.to_string()))
}

#[cfg(test)]
mod tests;
