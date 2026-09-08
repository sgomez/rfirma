//! Los dobles de los puertos de `signing`, la memoria en un temporal y la prueba de que hubo un ciclo, compartidos por las gradas A de todos los contextos.

use std::path::Path;

use crate::desktop::adapters::paths::Paths;
use crate::signing::adapters::memory::Memory;
use crate::signing::adapters::orders::{PlacementOrder, SigningOrder, VisibleFieldsOrder};
use crate::signing::domain::bridge::{
    BridgeError, Format, PostSignRequest, PreSignBlock, PreSignRequest, PreSignature,
    SignatureOperation,
};
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::domain::{CompletedCycle, SessionSeal};
use crate::signing::ports::{Bridge, DocumentBytes, IsolateHost};

/// Un hilo del puente cuya librería no abre: lo que la grada A tiene en vez del isolate.
pub(crate) struct NoIsolate;

impl IsolateHost for NoIsolate {
    fn run<T: Send + 'static>(
        &self,
        _task: impl FnOnce(&dyn Bridge) -> T + Send + 'static,
    ) -> Result<Result<T, BridgeError>, IsolateGone> {
        Ok(Err(BridgeError::Failed(
            "no hay libreria en grada A".to_owned(),
        )))
    }
}

/// La firma CAdES del banco de referencia, la que el puente doblado devuelve cuando no se le pide un PDF.
pub(crate) const A_CADES_SIGNATURE: &[u8] =
    include_bytes!("../../../../../testdata/reference/cades-implicit.p7s");

/// Una fase del ciclo tal y como le llegó al puente.
pub(crate) struct BridgeCall {
    /// El formato con el que se le pidió.
    pub(crate) format: Format,
    /// Qué se le pidió hacer con el documento.
    pub(crate) operation: SignatureOperation,
    /// El algoritmo con el que se le pidió la prefirma.
    pub(crate) algorithm: String,
    /// El bloque `java.util.Properties` que cruzó.
    pub(crate) extra_params: String,
}

/// Un puente que resuelve las dos fases con firmas del banco de referencia y apunta lo que le llega.
#[derive(Default)]
pub(crate) struct ABridgeThatSigns {
    calls: std::sync::Mutex<Vec<BridgeCall>>,
}

impl ABridgeThatSigns {
    /// Las fases que cruzaron, en orden.
    pub(crate) fn calls(&self) -> std::sync::MutexGuard<'_, Vec<BridgeCall>> {
        crate::lock(&self.calls)
    }
}

impl Bridge for ABridgeThatSigns {
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError> {
        request.format.bridged()?;
        crate::lock(&self.calls).push(BridgeCall {
            format: request.format,
            operation: request.operation,
            algorithm: request.algorithm.to_owned(),
            extra_params: request.extra_params.to_owned(),
        });
        Ok(PreSignature {
            session: "<xml/>".to_owned(),
            blocks: vec![a_block()],
            stamp: SessionSeal::from_bridge("el sello de la prefirma"),
        })
    }

    fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError> {
        request.format.bridged()?;
        Ok(match request.format {
            Format::Pades => b"%PDF-1.7 firmado".to_vec(),
            _ => A_CADES_SIGNATURE.to_vec(),
        })
    }
}

/// El hilo del puente con el doble detrás: corre la tarea en el sitio, que en grada A no hay isolate.
pub(crate) struct AnIsolateWith<'a>(pub(crate) &'a ABridgeThatSigns);

impl IsolateHost for AnIsolateWith<'_> {
    fn run<T: Send + 'static>(
        &self,
        task: impl FnOnce(&dyn Bridge) -> T + Send + 'static,
    ) -> Result<Result<T, BridgeError>, IsolateGone> {
        Ok(Ok(task(self.0)))
    }
}

/// Los documentos que la grada A da a firmar, sin disco detrás.
#[derive(Default)]
pub(crate) struct DocumentsInMemory(std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>);

impl DocumentsInMemory {
    /// Con ese documento dentro.
    pub(crate) fn with(mut self, path: impl Into<std::path::PathBuf>, bytes: &[u8]) -> Self {
        self.0.insert(path.into(), bytes.to_vec());
        self
    }
}

impl DocumentBytes for DocumentsInMemory {
    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.0
            .get(path)
            .cloned()
            .ok_or_else(|| "no such file or directory".to_owned())
    }
}

/// Crea una memoria aislada bajo la ruta temporal indicada.
pub(crate) fn a_memory(root: &Path) -> Memory {
    Memory::at(&Paths::under(root))
}

/// Genera una orden de firma completa de partida para pruebas.
pub(crate) fn an_order() -> SigningOrder {
    SigningOrder {
        document: "/run/user/1000/doc/1e8b83b9/contrato.pdf".to_owned(),
        certificate: "FIRMA".to_owned(),
        placement: Some(PlacementOrder {
            page: 1,
            pages: crate::signing::domain::PageSet::only_page(1),
            page_count: 3,
            media_box: [0.0, 0.0, 595.0, 842.0],
            rotation: 0,
            rect: [72.0, 500.0, 272.0, 600.0],
        }),
        fields: VisibleFieldsOrder {
            signer_name: true,
            issuer: true,
            signed_at: true,
            reason: true,
        },
        reason: String::new(),
        signed_at: "31/08/26, 12:00:00".to_owned(),
        rubric: None,
        language: "es".to_owned(),
        allow_unregistered_signatures: false,
    }
}

/// El único bloque a firmar del ciclo doblado.
fn a_block() -> PreSignBlock {
    PreSignBlock {
        id: "001".to_owned(),
        pre: b"123".to_vec(),
    }
}

/// Un ciclo trifásico terminado con una firma inventada, para quien necesite la prueba de que hubo uno.
pub(crate) fn a_completed_cycle() -> CompletedCycle {
    let stamp = SessionSeal::from_bridge("el sello de la prefirma");
    let presigned = PreSignature {
        session: "<xml/>".to_owned(),
        blocks: vec![a_block()],
        stamp: stamp.clone(),
    };
    presigned
        .sealed_with(presigned.invented_signatures(), &stamp)
        .expect("el sello es el mismo")
        .completed_with(b"%PDF-1.7 firmado".to_vec())
}
