use super::ListedCertificates;
use super::{
    certificate_behind, certificates_with_their_chains, listed_rows, remember_the_certificate,
    usable_certificate,
};
use crate::identity::application::tests::{
    a_certificate, a_certificate_with_id, listed_from, NoToken, TestAuthority,
};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::CertificateMemory as _;
use crate::identity::ports::Token;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::tests::a_memory;

#[test]
fn with_nowhere_to_look_the_listing_says_so_instead_of_coming_back_empty() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let failure = listed_rows(
        &NoToken,
        &[],
        &home.path().join("certificates"),
        &ListedCertificates::new(),
        &a_memory(home.path()),
    )
    .expect_err("no hay donde buscar");

    assert!(!failure.detail().is_empty(), "con su detalle crudo");
}

#[test]
fn refuses_a_certificate_that_is_no_longer_in_the_token() {
    let certificates = [a_certificate("FIRMA", &[])];
    let (listed, handles) = listed_from(&certificates);

    let failure = usable_certificate(&[], &handles[0], &listed).expect_err("ya no esta");

    assert_eq!(failure.situation(), Situation::CertificateNotFound);
    assert!(failure.detail().contains("FIRMA"), "{}", failure.detail());
}

#[test]
fn refuses_a_handle_that_is_not_from_the_last_listing() {
    let listed = ListedCertificates::new();

    let failure = usable_certificate(&[], "00000000000000000000000000000000", &listed)
        .expect_err("no es de la ultima busqueda");

    assert_eq!(failure.situation(), Situation::CertificateNotFound);
}

#[test]
fn two_certificates_with_the_same_label_are_chosen_apart() {
    let certificates = [
        a_certificate_with_id("FNMT-GEMELO-99999999R", 0x04, &[]),
        a_certificate_with_id("FNMT-GEMELO-99999999R", 0x05, &[]),
    ];
    let (listed, handles) = listed_from(&certificates);

    let first = certificate_behind(&certificates, &handles[0], &listed).expect("el primero");
    let second = certificate_behind(&certificates, &handles[1], &listed).expect("el segundo");

    assert_ne!(handles[0], handles[1]);
    assert_eq!(first.reference().cka_id(), Some([0x04].as_slice()));
    assert_eq!(second.reference().cka_id(), Some([0x05].as_slice()));
}

#[test]
fn looks_at_the_status_again_between_listing_and_signing() {
    let certificates = [a_certificate("FIRMA", &[0x00, 0x01, 0x02])];
    let (listed, handles) = listed_from(&certificates);

    let failure =
        usable_certificate(&certificates, &handles[0], &listed).expect_err("no es legible");

    assert_eq!(failure.situation(), Situation::CertificateNotFound);
    assert!(
        failure.detail().contains("Unreadable"),
        "{}",
        failure.detail()
    );
}

#[test]
fn the_certificate_signed_with_is_written_into_the_state() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(documents.path());
    let used = a_certificate("FNMT-ACTIVO-99999999R", b"da igual");

    remember_the_certificate(&memory, used.reference());

    assert_eq!(
        memory.remembered_certificate().as_ref(),
        Some(used.reference()),
        "la proxima sesion tiene que encontrarlo"
    );
}

#[test]
fn the_certificate_is_not_remembered_with_the_activity_switch_off() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let paths = crate::desktop::adapters::paths::Paths::under(documents.path());
    let memory = a_memory(documents.path());
    let switched_off = Configuration {
        remember_activity: false,
        ..Configuration::default()
    };
    memory
        .remember_configuration(&switched_off)
        .expect("deberia guardarse la configuracion");
    let used = a_certificate("FNMT-ACTIVO-99999999R", b"da igual");

    remember_the_certificate(&memory, used.reference());

    assert!(
        !paths.state_file().exists(),
        "con el interruptor apagado no se escribe ningun certificado"
    );
    assert_eq!(memory.remembered_certificate(), None);
}

#[test]
fn turning_the_activity_switch_off_erases_the_certificate_already_remembered() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(documents.path());
    remember_the_certificate(
        &memory,
        a_certificate("FNMT-ACTIVO-99999999R", b"da igual").reference(),
    );

    memory
        .remember_configuration(&Configuration {
            remember_activity: false,
            ..Configuration::default()
        })
        .expect("deberia guardarse la configuracion");

    assert_eq!(memory.remembered_certificate(), None);
}

#[test]
fn a_remembered_certificate_that_is_gone_marks_no_row() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(documents.path());
    remember_the_certificate(
        &memory,
        a_certificate("EL-QUE-YA-NO-ESTA", b"da igual").reference(),
    );
    let remembered = memory.remembered_certificate().expect("algo se recordo");

    let present = a_certificate("FNMT-ACTIVO-99999999R", b"da igual");

    assert!(!remembered.is_the_same_as(present.reference()));
}

#[test]
fn a_first_run_has_no_remembered_certificate() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");

    assert_eq!(a_memory(documents.path()).remembered_certificate(), None);
}

/// El módulo de un token con la clave dentro.
const CARD: &str = "/usr/lib/softhsm/libsofthsm2.so";
/// El módulo del almacén NSS donde aterriza un `.p12` instalado.
const INSTALLED: &str = "/usr/lib/libsoftokn3.so";

/// Un token cuyos almacenes tienen lo que se le diga, firmable o no.
struct StoresWith {
    signable: Vec<TokenCertificate>,
    everything: Result<Vec<TokenCertificate>, TokenError>,
}

impl StoresWith {
    /// Todo lo que hay en los almacenes y, de entre ello, lo que es firmable.
    fn holding(everything: Vec<TokenCertificate>, signable: Vec<TokenCertificate>) -> Self {
        Self {
            signable,
            everything: Ok(everything),
        }
    }

    fn a_store_that_cannot_be_opened(signable: Vec<TokenCertificate>) -> Self {
        Self {
            signable,
            everything: Err(TokenError::new(
                Situation::ModuleNotFound,
                "el almacen no se deja abrir",
            )),
        }
    }

    fn only(&self, store: &Store, certificates: &[TokenCertificate]) -> Vec<TokenCertificate> {
        certificates
            .iter()
            .filter(|certificate| certificate.reference().store() == *store)
            .cloned()
            .collect()
    }
}

impl Token for StoresWith {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self.only(store, &self.signable))
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        let everything = self.everything.as_ref().map_err(Clone::clone)?;
        Ok(self.only(store, everything))
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Ok(StoreSecret::NotNeeded)
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        _algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        Ok(())
    }

    fn sign(
        &self,
        _reference: &CertificateRef,
        _pin: &str,
        _algorithm: SignatureAlgorithm,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        Err(TokenError::new(
            Situation::CertificateNotFound,
            "este token no firma",
        ))
    }

    fn import_pkcs12(
        &self,
        _directory: &std::path::Path,
        _pkcs12: &[u8],
        _password: &str,
    ) -> Result<Store, TokenError> {
        Err(TokenError::new(
            Situation::Pkcs12Unreadable,
            "este token no importa nada",
        ))
    }
}

fn a_certificate_in(module: &str, label: &str, der: &[u8]) -> TokenCertificate {
    TokenCertificate::new(
        CertificateRef::new(module, "rfirma-test", label, vec![0x01]),
        der.to_vec(),
    )
}

#[test]
fn a_certificate_carries_the_issuer_that_its_own_store_has() {
    let root = TestAuthority::root("Raiz de pruebas");
    let authority = root.issues("AC de pruebas");
    let signer = authority.issues("Firmante de pruebas");
    let signable = vec![a_certificate_in(CARD, "FIRMA", &signer.der())];
    let token = StoresWith::holding(
        vec![
            signable[0].clone(),
            a_certificate_in(CARD, "AC", &authority.der()),
        ],
        signable,
    );

    let found = certificates_with_their_chains(&token, &[Store::module(CARD)])
        .expect("el almacen deberia listarse");

    assert_eq!(found[0].chain(), vec![signer.der(), authority.der()]);
}

#[test]
fn a_certificate_alone_in_its_store_goes_with_nothing_behind() {
    let root = TestAuthority::root("Raiz de pruebas");
    let authority = root.issues("AC de pruebas");
    let signer = authority.issues("Firmante de pruebas");
    let signable = vec![a_certificate_in(CARD, "FIRMA", &signer.der())];
    let token = StoresWith::holding(signable.clone(), signable);

    let found = certificates_with_their_chains(&token, &[Store::module(CARD)])
        .expect("el almacen deberia listarse");

    assert_eq!(found[0].chain(), vec![signer.der()]);
}

#[test]
fn the_issuer_of_another_store_does_not_complete_the_chain() {
    let root = TestAuthority::root("Raiz de pruebas");
    let authority = root.issues("AC de pruebas");
    let signer = authority.issues("Firmante de pruebas");
    let signable = vec![a_certificate_in(CARD, "FIRMA", &signer.der())];
    let token = StoresWith::holding(
        vec![
            signable[0].clone(),
            a_certificate_in(INSTALLED, "AC", &authority.der()),
        ],
        signable,
    );

    let found =
        certificates_with_their_chains(&token, &[Store::module(CARD), Store::module(INSTALLED)])
            .expect("los almacenes deberian listarse");

    assert_eq!(found[0].chain(), vec![signer.der()]);
}

#[test]
fn an_installed_p12_signs_with_the_chain_that_came_inside_it() {
    let root = TestAuthority::root("Raiz de pruebas");
    let authority = root.issues("AC de pruebas");
    let signer = authority.issues("Firmante de pruebas");
    let signable = vec![a_certificate_in(INSTALLED, "FIRMA", &signer.der())];
    let token = StoresWith::holding(
        vec![
            signable[0].clone(),
            a_certificate_in(INSTALLED, "AC", &authority.der()),
            a_certificate_in(INSTALLED, "RAIZ", &root.der()),
        ],
        signable,
    );

    let found = certificates_with_their_chains(&token, &[Store::module(INSTALLED)])
        .expect("el almacen instalado deberia listarse");

    assert_eq!(found[0].chain(), vec![signer.der(), authority.der()]);
}

#[test]
fn a_store_that_cannot_be_opened_leaves_the_signer_alone_instead_of_stopping_the_listing() {
    let root = TestAuthority::root("Raiz de pruebas");
    let authority = root.issues("AC de pruebas");
    let signer = authority.issues("Firmante de pruebas");
    let signable = vec![a_certificate_in(CARD, "FIRMA", &signer.der())];
    let token = StoresWith::a_store_that_cannot_be_opened(signable);

    let found = certificates_with_their_chains(&token, &[Store::module(CARD)])
        .expect("el listado sigue aunque no se puedan mirar las autoridades");

    assert_eq!(found[0].chain(), vec![signer.der()]);
}
