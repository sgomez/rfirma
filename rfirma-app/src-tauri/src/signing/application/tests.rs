//! Los dobles de los puertos de `signing`, la memoria en un temporal y la prueba de que hubo un ciclo, compartidos por las gradas A de todos los contextos.

use std::path::Path;

use crate::desktop::adapters::paths::Paths;
use crate::signing::adapters::memory::Memory;
use crate::signing::adapters::orders::{PlacementOrder, SigningOrder, VisibleFieldsOrder};
use crate::signing::domain::bridge::{BridgeError, PreSignature};
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::domain::{CompletedCycle, SessionSeal, TokenSignature};
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

/// Un ciclo trifásico terminado con una firma inventada, para quien necesite la prueba de que hubo uno.
pub(crate) fn a_completed_cycle() -> CompletedCycle {
    let stamp = SessionSeal::from_bridge("el sello de la prefirma");
    PreSignature {
        session: "<xml/>".to_owned(),
        pre_sign: b"123".to_vec(),
        stamp: stamp.clone(),
    }
    .sealed_with(&TokenSignature::invented(), &stamp)
    .expect("el sello es el mismo")
    .completed_with(b"%PDF-1.7 firmado".to_vec())
}
