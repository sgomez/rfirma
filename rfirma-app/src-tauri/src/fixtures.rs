//! Andamios y datos de prueba compartidos entre casos de uso.

use std::path::Path;
use std::sync::Mutex;

use crate::desktop::adapters::paths::Paths;
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;
use crate::signing::adapters::orders::{PlacementOrder, SigningOrder, VisibleFieldsOrder};
use crate::signing::domain::bridge::{BridgeError, PreSignature};
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::domain::{CompletedCycle, SessionSeal, TokenSignature};
use crate::signing::ports::{Bridge, IsolateHost};
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::tls_error::{Situation as TlsSituation, TlsError};
use crate::site::ports::LocalCaSlots;
use crate::Memory;

/// Un token sin certificados que no sabe firmar: cada almacén está vacío.
pub(crate) struct NoToken;

impl Token for NoToken {
    fn list(&self, _store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(Vec::new())
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Ok(StoreSecret::NotNeeded)
    }

    fn sign(
        &self,
        _reference: &CertificateRef,
        _pin: &str,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        Err(TokenError::new(
            Situation::CertificateNotFound,
            "este token no tiene con que firmar",
        ))
    }

    fn import_pkcs12(
        &self,
        _directory: &Path,
        _pkcs12: &[u8],
        _password: &str,
    ) -> Result<Store, TokenError> {
        Err(TokenError::new(
            Situation::Pkcs12Unreadable,
            "este token no importa nada",
        ))
    }
}

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

/// Las dos ranuras de la CA local en memoria, escribibles o no.
#[derive(Default)]
pub(crate) struct InMemoryCaSlots {
    serving: Mutex<Option<LocalCa>>,
    next: Mutex<Option<LocalCa>>,
    unwritable: bool,
}

impl InMemoryCaSlots {
    /// Unas ranuras en las que no se puede escribir, como un disco de solo lectura.
    pub(crate) fn unwritable() -> Self {
        Self {
            unwritable: true,
            ..Self::default()
        }
    }

    fn writing(&self) -> Result<(), TlsError> {
        if self.unwritable {
            return Err(TlsError::new(
                TlsSituation::MaterialUnwritable,
                "estas ranuras no dejan escribir",
            ));
        }
        Ok(())
    }
}

impl LocalCaSlots for InMemoryCaSlots {
    fn serving(&self) -> Result<Option<LocalCa>, TlsError> {
        Ok(crate::lock(&self.serving).clone())
    }

    fn write_serving(&self, ca: &LocalCa) -> Result<(), TlsError> {
        self.writing()?;
        *crate::lock(&self.serving) = Some(ca.clone());
        Ok(())
    }

    fn next(&self) -> Result<Option<LocalCa>, TlsError> {
        Ok(crate::lock(&self.next).clone())
    }

    fn write_next(&self, ca: &LocalCa) -> Result<(), TlsError> {
        self.writing()?;
        *crate::lock(&self.next) = Some(ca.clone());
        Ok(())
    }

    fn promote_next(&self) -> Result<Option<LocalCa>, TlsError> {
        self.writing()?;
        let promoted = crate::lock(&self.next).take();
        if let Some(ca) = &promoted {
            *crate::lock(&self.serving) = Some(ca.clone());
        }
        Ok(promoted)
    }

    fn forget_next(&self) -> Result<(), TlsError> {
        self.writing()?;
        *crate::lock(&self.next) = None;
        Ok(())
    }
}

/// Construye un certificado de prueba con la etiqueta y DER proporcionados.
pub(crate) fn a_certificate(label: &str, der: &[u8]) -> TokenCertificate {
    a_certificate_with_id(label, 0x01, der)
}

/// Construye un certificado de prueba especificando su CKA_ID.
pub(crate) fn a_certificate_with_id(label: &str, cka_id: u8, der: &[u8]) -> TokenCertificate {
    TokenCertificate::new(
        CertificateRef::new(
            "/usr/lib/softhsm/libsofthsm2.so",
            "rfirma-test",
            label,
            vec![cka_id],
        ),
        der.to_vec(),
    )
}

/// Construye un certificado X.509 válido generado con la CA local de pruebas.
pub(crate) fn a_usable_certificate(label: &str) -> TokenCertificate {
    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let der = ca
        .certificate()
        .to_der()
        .expect("el certificado deberia poder salir en DER");
    a_certificate(label, &der)
}

/// Inicializa un registro de certificados listados y devuelve sus identificadores.
pub(crate) fn listed_from(certificates: &[TokenCertificate]) -> (ListedCertificates, Vec<String>) {
    let listed = ListedCertificates::new();
    let handles = listed.replace(
        certificates
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );
    (listed, handles)
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
