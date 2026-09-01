//! El módulo PKCS#11 contra un token de verdad. **Grada B** (SoftHSM): carril
//! rápido, segundos (ADR-0014, TD-01).
//!
//! # Cómo se monta el token
//!
//! ```sh
//! sudo apt install -y softhsm2 opensc
//! just token          # o: bash testdata/softhsm/provision-token.sh
//! ```
//!
//! El script es idempotente y `just check` lo ejecuta solo, así que en la
//! práctica basta con tener los paquetes. Provisiona el token `rfirma-test`
//! (PIN `1234`) desde `testdata/fnmt/`, que es material público de la FNMT:
//!
//! | `CKA_ID` | etiqueta | qué tiene | para qué |
//! | --- | --- | --- | --- |
//! | `01` | `FNMT-ACTIVO-99999999R` | clave + certificado | camino feliz |
//! | `02` | `FNMT-CADUCADO-99999999R` | clave + certificado | caducó en 2020 |
//! | `03` | `FNMT-REVOCADO-99999999R` | clave + certificado | revocado en 2024, en vigor |
//! | `04` | `FNMT-GEMELO-99999999R` | clave + certificado | par de claves del activo |
//! | `05` | `FNMT-GEMELO-99999999R` | clave + certificado | par de claves del caducado |
//!
//! Los dos gemelos comparten `CKA_LABEL` y no comparten nada más: son la
//! reproducción de lo que hay en un perfil de Firefox de verdad, donde dos
//! claves privadas llevan la misma etiqueta.
//!
//! Los cinco llevan clave (#100): sin ella el filtro de certificados
//! firmables (ID-07) los haría desaparecer del listado, y el caducado y el
//! revocado existen justamente para que el listado tenga que clasificarlos.
//!
//! El detalle del entorno está en `docs/research/token-pkcs11-pruebas.md`.
//!
//! # Por qué estas pruebas inician sesión para listar
//!
//! [`pkcs11::list_certificates`] no pide el PIN (ID-08): el filtro de #100
//! busca la clave privada sin sesión, y en NSS y en una tarjeta real eso basta
//! para saber que existe. **SoftHSM no se comporta así.** Sus objetos
//! `CKO_PRIVATE_KEY` llevan `CKA_PRIVATE` de forma incondicional —comprobado
//! con `pkcs11-tool`, con `p11tool --write` sin `--mark-private`, y con un
//! `C_SetAttributeValue` directo, que devuelve `CKR_ATTRIBUTE_READ_ONLY`—, así
//! que sin sesión SoftHSM no enseña ninguna clave, tenga par o no. Con el
//! filtro tal cual, `list_certificates` contra este token da siempre la lista
//! vacía, y así lo comprueba `listing_without_a_session_finds_nothing_on_softhsm`.
//!
//! Por eso el resto de estas pruebas usa
//! [`pkcs11::list_certificates_authenticated_for_test`], que aplica el
//! **mismo** filtro con la sesión ya autenticada: sirve para comprobar la
//! clasificación y la firma sin depender de esta peculiaridad de SoftHSM. La
//! prueba de que el filtro de verdad no pide el PIN, y de que sí encuentra las
//! claves cuando el módulo lo permite, vive contra el almacén NSS en
//! `tests/nss_store.rs`.
//!
//! # Lo que aquí no se comprueba
//!
//! Que el revocado esté revocado: eso es preguntárselo al OCSP de la FNMT, o
//! sea red, o sea **grada D**, que va al cron y nunca a un PR (TD-08). Aquí solo
//! se comprueba que un certificado en vigor no se confunde con uno caducado.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rfirma_lib::pkcs11::{
    self, CertificateRef, CertificateStatus, Situation, TokenCertificate, TokenError,
};
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;
use sha2::{Digest, Sha256};
use x509_cert::der::{Decode, Encode};

const TOKEN: &str = "rfirma-test";
const PIN: &str = "1234";
const ACTIVE: &str = "FNMT-ACTIVO-99999999R";
const EXPIRED: &str = "FNMT-CADUCADO-99999999R";
const REVOKED: &str = "FNMT-REVOCADO-99999999R";
/// Los dos certificados que comparten etiqueta y no comparten clave.
const TWIN: &str = "FNMT-GEMELO-99999999R";
const TWIN_OF_THE_ACTIVE_KEY: u8 = 0x04;
const TWIN_OF_THE_EXPIRED_KEY: u8 = 0x05;

/// Lo mismo que firma el recorrido real: un bloque DER de `SignedAttributes`
/// que nadie ha hasheado antes de llegar aquí. Que sea DER de verdad da igual
/// para lo que se prueba; que **no** sea un hash, no.
const PRESIGN: &[u8] = b"31 5f 30 18 06 09 2a 86 SignedAttributes de mentira, sin hashear";

fn module() -> PathBuf {
    let module = PathBuf::from(
        std::env::var("RFIRMA_PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
    );
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. Las pruebas de grada B necesitan SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just token",
        module.display()
    );
    module
}

/// Lista con el filtro de #100 aplicado, autenticada: ver la nota del módulo
/// sobre por qué esto no puede ser [`pkcs11::list_certificates`] a secas
/// contra SoftHSM.
fn certificates() -> Vec<TokenCertificate> {
    let found = pkcs11::list_certificates_authenticated_for_test(module(), PIN)
        .expect("no se ha podido listar el token");
    assert!(
        !found.is_empty(),
        "el token {TOKEN} esta vacio o no existe. Montalo con:\n  just token"
    );
    found
}

fn certificate_labelled(label: &str) -> TokenCertificate {
    certificates()
        .into_iter()
        .find(|certificate| certificate.reference().label() == label)
        .unwrap_or_else(|| {
            panic!("el token {TOKEN} no tiene ningun certificado {label}. Montalo con: just token")
        })
}

/// La referencia **tal y como sale del token**, con su `CKA_ID` incluido. No se
/// fabrica a mano: quien firma parte de lo que devolvió el listado, y una
/// referencia inventada aquí no probaría el mismo camino.
fn reference(label: &str) -> CertificateRef {
    certificate_labelled(label).reference().clone()
}

fn certificate_with_cka_id(cka_id: u8) -> TokenCertificate {
    certificates()
        .into_iter()
        .find(|certificate| certificate.reference().cka_id() == Some([cka_id].as_slice()))
        .unwrap_or_else(|| {
            panic!(
                "el token {TOKEN} no tiene ningun certificado con CKA_ID {cka_id:02x}. \
                 Montalo con: just token"
            )
        })
}

fn epoch(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

// ---------------------------------------------------------------------------
// Listar
// ---------------------------------------------------------------------------

#[test]
fn listing_gives_back_what_it_takes_to_find_each_certificate_again() {
    let certificate = certificate_labelled(ACTIVE);
    let reference = certificate.reference();

    assert_eq!(reference.module(), module().as_path());
    assert_eq!(reference.token_label(), TOKEN);
    assert_eq!(reference.label(), ACTIVE);
    // El CKA_ID es la coordenada que empareja el certificado con su clave, y
    // sin ella lo persistido no basta para reencontrar este certificado y no
    // otro con su misma etiqueta.
    assert_eq!(reference.cka_id(), Some([0x01].as_slice()));
}

/// El titular se lee del DER cuando hace falta pintarlo, y **no** viaja dentro
/// de la referencia: lo que se persiste son cuatro coordenadas y nada más (ID-32,
/// ADR-0010).
///
/// Que la etiqueta de este token concreto lleve el DNI dentro no es asunto
/// nuestro: la etiqueta la pone quien provisiona el token, y es justamente el
/// dato que hay que guardar para reencontrar el certificado.
#[test]
fn the_holder_is_readable_for_display_but_is_not_part_of_the_reference() {
    let certificate = certificate_labelled(ACTIVE);

    let subject = certificate.subject().expect("el DER deberia leerse");
    assert!(
        subject.contains("EIDAS CERTIFICADO PRUEBAS"),
        "titular leido: {subject}"
    );

    let reference = format!("{:?}", certificate.reference());
    assert!(
        !reference.contains("EIDAS CERTIFICADO PRUEBAS"),
        "la referencia persistible no puede llevar el titular: {reference}"
    );
}

/// El emisor sale del **issuer** del DER, y en un certificado de persona física
/// de la FNMT no hay otro sitio de donde sacarlo: su subject **no lleva `O=`**.
/// Leerlo de ahí dejaba el panel en «Emitido por » y nada más.
#[test]
fn the_issuer_is_the_authority_and_the_subject_has_no_organisation_to_confuse_it_with() {
    let certificate = certificate_labelled(ACTIVE);

    let issuer = certificate.issuer().expect("el DER deberia leerse");
    let subject = certificate.subject().expect("el DER deberia leerse");

    assert!(
        issuer.contains("AC FNMT Usuarios"),
        "emisor leido: {issuer}"
    );
    assert!(
        !subject.contains("O="),
        "el subject de este certificado no lleva organizacion: {subject}"
    );
}

#[test]
fn listing_does_not_need_the_pin() {
    // certificates() usa la variante autenticada de proposito (ver la nota del
    // modulo); lo que esta prueba comprueba es que la funcion publica de
    // verdad, pkcs11::list_certificates, no le pide el PIN a nadie: se llama
    // sin sesion y no falla.
    assert!(pkcs11::list_certificates(module()).is_ok());
}

/// La otra cara de la nota del módulo, fijada en una prueba: sin sesión,
/// SoftHSM no enseña ninguna clave privada, así que el filtro de #100 deja la
/// lista vacía contra este token **siempre**, tengan o no clave los
/// certificados. No es un fallo de rfirma — es lo que hace SoftHSM con sus
/// objetos `CKO_PRIVATE_KEY`, y es justo el motivo de que el resto de estas
/// pruebas pase por `list_certificates_authenticated_for_test`.
#[test]
fn listing_without_a_session_finds_nothing_on_softhsm() {
    let found = pkcs11::list_certificates(module()).expect("no deberia fallar sin PIN");
    assert!(
        found.is_empty(),
        "SoftHSM ha dejado de esconder sus claves privadas sin sesion: si esto \
         cambia, el escape de list_certificates_authenticated_for_test ya no \
         hace falta y estas pruebas pueden volver a pkcs11::list_certificates"
    );
}

// ---------------------------------------------------------------------------
// Clasificar el certificado, antes de pedir el PIN
// ---------------------------------------------------------------------------

#[test]
fn an_expired_certificate_is_told_apart_from_a_token_failure() {
    let status = certificate_labelled(EXPIRED).status();

    // Ni es un TokenError ni pretende serlo: es un estado del certificado.
    match status {
        CertificateStatus::Expired { not_after } => {
            // 2020-11-08 12:48:35 GMT, segun testdata/fnmt/README.md.
            assert_eq!(not_after, 1_604_839_715);
        }
        other => panic!("el certificado caducado se ha clasificado como {other:?}"),
    }
}

#[test]
fn a_certificate_in_date_is_usable_even_if_its_issuer_revoked_it() {
    // El revocado sigue en vigor hasta 2028: sin red no se puede saber mas, y
    // fingir lo contrario seria mentir. La revocacion la decide el OCSP, que es
    // grada D (TD-08).
    for label in [ACTIVE, REVOKED] {
        assert!(
            certificate_labelled(label).status().is_usable(),
            "{label} deberia estar en vigor"
        );
    }
}

#[test]
fn the_same_certificate_changes_status_with_the_clock_and_not_with_the_token() {
    let certificate = certificate_labelled(ACTIVE);

    // 2028-10-30 10:06:59 GMT es su notAfter (testdata/fnmt/README.md).
    assert!(certificate.status_at(epoch(1_856_513_218)).is_usable());
    assert!(matches!(
        certificate.status_at(epoch(1_856_513_220)),
        CertificateStatus::Expired { .. }
    ));
    assert!(matches!(
        certificate.status_at(epoch(0)),
        CertificateStatus::NotYetValid { .. }
    ));
}

// ---------------------------------------------------------------------------
// Firmar
// ---------------------------------------------------------------------------

fn verifying_key(certificate: &TokenCertificate) -> VerifyingKey<Sha256> {
    let parsed =
        x509_cert::Certificate::from_der(certificate.der()).expect("el DER deberia parsearse");
    let spki = parsed
        .tbs_certificate()
        .subject_public_key_info()
        .to_der()
        .expect("el SPKI deberia serializarse");
    let public_key = RsaPublicKey::from_public_key_der(&spki).expect("clave publica RSA");
    VerifyingKey::<Sha256>::new(public_key)
}

#[test]
fn signing_produces_a_signature_that_the_certificate_public_key_verifies() {
    let certificate = certificate_labelled(ACTIVE);
    let raw = pkcs11::sign(&reference(ACTIVE), PIN, PRESIGN).expect("la firma deberia salir");

    // RSA 2048: la firma cruda mide exactamente el modulo.
    assert_eq!(raw.len(), 256);

    let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
    verifying_key(&certificate)
        .verify(PRESIGN, &signature)
        .expect("la firma no verifica contra la clave publica del certificado");
}

/// La prueba que el sub-issue #49 pide explícitamente: si alguien cambia el
/// mecanismo a `CKM_RSA_PKCS` sobre un hash, esto tiene que ponerse rojo.
///
/// Firmar `SHA-256(PRESIGN)` con `CKM_RSA_PKCS` es lo que haría ese cambio, y
/// produce una firma RSA impecable —el token no protesta— que **ningún**
/// validador CAdES reconoce, porque le falta el envoltorio `DigestInfo`. La
/// única defensa es esta: comprobar que ese resultado NO verifica y que el
/// nuestro SÍ.
#[test]
fn signing_a_hash_with_the_bare_rsa_mechanism_would_not_verify() {
    let certificate = certificate_labelled(ACTIVE);
    let key = verifying_key(&certificate);

    let ours = pkcs11::sign(&reference(ACTIVE), PIN, PRESIGN).expect("la firma deberia salir");
    let over_a_hash = sign_with_bare_rsa_pkcs(&Sha256::digest(PRESIGN));

    assert!(
        key.verify(PRESIGN, &Signature::try_from(ours.as_slice()).unwrap())
            .is_ok(),
        "la firma de rfirma tiene que verificar sobre los bytes SIN hashear"
    );
    assert!(
        key.verify(
            PRESIGN,
            &Signature::try_from(over_a_hash.as_slice()).unwrap()
        )
        .is_err(),
        "CKM_RSA_PKCS sobre un hash ha verificado: el mecanismo del ID-16 ya no \
         es el que dice serlo"
    );
    assert_ne!(ours, over_a_hash);
}

/// El mecanismo descartado, invocado a mano contra el mismo token. No usa el
/// módulo de rfirma a propósito: es el contraejemplo, no una capacidad.
///
/// Lo que sí toma prestado de rfirma es el **turno del token**: iniciar sesión
/// por fuera de él sería cruzarse con las firmas de verdad —doce pruebas en
/// hilos del mismo proceso— y hacer que el `login` de aquí le devuelva
/// `CKR_USER_ALREADY_LOGGED_IN` a un `pkcs11::sign` con PIN equivocado, o que
/// su `logout()` cierre esta sesión entre el `login` y el `sign`. El candado no
/// es reentrante: dentro del cierre no puede llamarse a `pkcs11::sign`.
fn sign_with_bare_rsa_pkcs(data: &[u8]) -> Vec<u8> {
    pkcs11::with_token_turn(|| sign_with_bare_rsa_pkcs_holding_the_turn(data))
}

fn sign_with_bare_rsa_pkcs_holding_the_turn(data: &[u8]) -> Vec<u8> {
    use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
    use cryptoki::mechanism::Mechanism;
    use cryptoki::object::{Attribute, ObjectClass};
    use cryptoki::session::UserType;
    use cryptoki::types::AuthPin;

    let context = Pkcs11::new(module()).expect("modulo");
    // Este proceso ya inicializo el modulo por el camino de rfirma; SoftHSM
    // comparte estado por dlopen, asi que la segunda vez devuelve
    // CKR_CRYPTOKI_ALREADY_INITIALIZED y eso esta bien.
    let _ = context.initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK));

    let slot = context
        .get_slots_with_token()
        .expect("ranuras")
        .into_iter()
        .find(|slot| {
            context
                .get_token_info(*slot)
                .map(|info| info.label().trim() == TOKEN)
                .unwrap_or(false)
        })
        .expect("el token rfirma-test");

    let session = context.open_ro_session(slot).expect("sesion");
    // Con el turno cogido nadie mas del proceso tiene sesion iniciada contra
    // este token, asi que este login tiene que salir limpio.
    session
        .login(UserType::User, Some(&AuthPin::new(PIN.into())))
        .expect("el login del contraejemplo, con el turno del token cogido");
    let key = session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Label(ACTIVE.as_bytes().to_vec()),
        ])
        .expect("busqueda")
        .into_iter()
        .next()
        .expect("la clave del camino feliz");

    let signature = session
        .sign(&Mechanism::RsaPkcs, key, data)
        .expect("CKM_RSA_PKCS deberia firmar cualquier bloque que le quepa");

    // Se cierra la sesion autenticada antes de soltar el turno, igual que hace
    // pkcs11::sign: dejarla abierta dejaria el token desbloqueado para las
    // pruebas que vengan detras.
    let _ = session.logout();

    signature
}

#[test]
fn signing_the_same_bytes_twice_gives_the_same_signature() {
    // PKCS#1 v1.5 es determinista. Si esto cambia, alguien ha cambiado el
    // relleno a PSS y la prefirma de Java ya no cuadra.
    let once = pkcs11::sign(&reference(ACTIVE), PIN, PRESIGN).expect("firma");
    let twice = pkcs11::sign(&reference(ACTIVE), PIN, PRESIGN).expect("firma");

    assert_eq!(once, twice);
}

/// **La prueba del #98.** Dos certificados con la **misma** `CKA_LABEL` y
/// distinto `CKA_ID` firman cada uno con **su** clave.
///
/// Es el fallo que no se nota: buscando la clave por etiqueta el token devuelve
/// una de las dos arbitrariamente, la firma sale, el PDF se cierra, y lo que ha
/// firmado es una clave que no es la del certificado que la persona eligió. Por
/// eso no basta con que cada firma verifique: hace falta comprobar también que
/// **no** verifica contra el otro certificado, que es lo que estaría pasando si
/// alguien volviera a emparejar por etiqueta.
#[test]
fn two_certificates_sharing_a_label_each_sign_with_their_own_key() {
    let one = certificate_with_cka_id(TWIN_OF_THE_ACTIVE_KEY);
    let other = certificate_with_cka_id(TWIN_OF_THE_EXPIRED_KEY);

    assert_eq!(one.reference().label(), TWIN);
    assert_eq!(other.reference().label(), TWIN);
    assert_ne!(one.reference().cka_id(), other.reference().cka_id());

    let signed_by_one = pkcs11::sign(one.reference(), PIN, PRESIGN).expect("firma del primero");
    let signed_by_other = pkcs11::sign(other.reference(), PIN, PRESIGN).expect("firma del segundo");

    for (certificate, signature, twin) in [
        (&one, &signed_by_one, &other),
        (&other, &signed_by_other, &one),
    ] {
        let signature = Signature::try_from(signature.as_slice()).expect("firma RSA");
        verifying_key(certificate)
            .verify(PRESIGN, &signature)
            .expect("cada gemelo tiene que firmar con la clave de SU certificado");
        assert!(
            verifying_key(twin).verify(PRESIGN, &signature).is_err(),
            "la firma verifica contra el otro gemelo: se esta emparejando por etiqueta"
        );
    }
}

// ---------------------------------------------------------------------------
// Clasificar los errores del token
// ---------------------------------------------------------------------------

fn signing_error(reference: &CertificateRef, pin: &str) -> TokenError {
    pkcs11::sign(reference, pin, PRESIGN).expect_err("esto tenia que fallar")
}

#[test]
fn a_wrong_pin_is_a_situation_and_carries_its_raw_ckr_apart() {
    let error = signing_error(&reference(ACTIVE), "0000");

    assert_eq!(error.situation(), Situation::IncorrectPin);
    assert_eq!(error.ckr(), Some("CKR_PIN_INCORRECT"));
    assert!(error.detail().contains("CKR_PIN_INCORRECT"));
}

#[test]
fn a_token_that_is_not_there_is_told_apart_from_a_wrong_pin() {
    let absent = CertificateRef::new(module(), "no-existe-este-token", ACTIVE, vec![0x01]);
    let error = signing_error(&absent, PIN);

    assert_eq!(error.situation(), Situation::TokenAbsent);
    assert!(!error.detail().is_empty());
}

#[test]
fn a_module_that_is_not_there_is_not_a_token_error() {
    let error = pkcs11::list_certificates(Path::new("/usr/lib/no-hay-ningun-modulo-aqui.so"))
        .expect_err("un modulo inexistente no puede cargarse");

    assert_eq!(error.situation(), Situation::ModuleNotFound);
    assert_eq!(error.ckr(), None, "esto no viene de ningun CKR_*");
    assert!(error.detail().contains("no-hay-ningun-modulo-aqui.so"));
}

/// La promesa del ID-03: no tener un almacén instalado no puede dejar sin
/// certificados a quien sí tiene el otro.
///
/// Contra SoftHSM solo se puede comprobar la mitad de la promesa: que el
/// almacén roto no convierte el resultado en un error cuando el otro sí ha
/// cargado. La otra mitad —que el certificado del almacén que sí carga
/// **sale**— no se puede pedir aquí, porque `list_certificates` sin sesión
/// deja siempre vacío el SoftHSM de pruebas (ver la nota del módulo); esa
/// mitad la comprueba `a_profile_that_leads_nowhere_does_not_hide_the_one_that_works`
/// en `tests/nss_store.rs`, donde el almacén que sí carga muestra de verdad
/// su certificado.
#[test]
fn a_store_that_cannot_be_loaded_does_not_hide_the_ones_that_can() {
    let stores = vec![
        pkcs11::Store::module("/usr/lib/no-hay-ningun-modulo-aqui.so"),
        pkcs11::Store::module(module()),
    ];

    let found = pkcs11::list_certificates_across(&stores).expect(
        "el almacen que si carga tiene que seguir contando, aunque el filtro de #100 lo \
         deje sin certificados contra SoftHSM sin sesion",
    );

    assert!(
        found.is_empty(),
        "SoftHSM ha dejado de esconder sus claves privadas sin sesion: si esto \
         cambia, esta prueba puede volver a comprobar que ACTIVE sale de verdad"
    );
}

/// Y la otra mitad: cuando **nadie** ha podido cargar, lo que se cuenta es el
/// fallo y no un «no hay ninguno» que seria mentira.
#[test]
fn tells_the_failure_apart_from_an_empty_list_when_no_store_loads() {
    let stores = vec![
        pkcs11::Store::module("/usr/lib/no-hay-ningun-modulo-aqui.so"),
        pkcs11::Store::module("/usr/lib/tampoco-hay-este-otro.so"),
    ];

    let error = pkcs11::list_certificates_across(&stores)
        .expect_err("sin ningun almacen cargado no hay lista que devolver");

    assert_eq!(error.situation(), Situation::ModuleNotFound);
}

/// Sin almacenes tampoco se calla: quedarse sin donde buscar es un fallo de
/// configuracion, no una lista vacia.
#[test]
fn having_nowhere_to_look_is_a_failure_and_not_an_empty_list() {
    let error =
        pkcs11::list_certificates_across(&[]).expect_err("sin almacenes no hay donde buscar");

    assert_eq!(error.situation(), Situation::ModuleNotFound);
    assert!(!error.detail().is_empty());
}

#[test]
fn a_cka_id_that_is_not_in_the_token_says_so_instead_of_failing_generically() {
    let missing = CertificateRef::new(module(), TOKEN, "ETIQUETA-QUE-NO-EXISTE", vec![0xff]);
    let error = signing_error(&missing, PIN);

    assert_eq!(error.situation(), Situation::CertificateNotFound);
}

/// Una referencia recordada por una versión anterior al #98 no lleva `CKA_ID`.
/// Firmar con ella se rechaza en vez de caer de vuelta en la etiqueta: el
/// respaldo por etiqueta es justo el fallo que este cambio cierra.
#[test]
fn a_reference_without_a_cka_id_refuses_to_sign_instead_of_guessing_by_label() {
    let remembered = CertificateRef::new(module(), TOKEN, ACTIVE, None);

    let error = signing_error(&remembered, PIN);

    assert_eq!(error.situation(), Situation::CertificateNotFound);
    assert!(error.detail().contains("CKA_ID"), "{}", error.detail());
}

// Antes de #100, el caducado y el revocado entraban en el token SIN clave a
// proposito, y esta prueba pedia una firma con ellos para comprobar que eso
// es un fallo del token y no del certificado. Con el filtro de #100 esa
// referencia ya no la devuelve nunca el listado -un certificado sin clave no
// se ofrece-, y el propio script de aprovisionamiento les da su clave (misma
// razon: si no la tuvieran, el filtro los haria desaparecer y las pruebas de
// estado de mas arriba se quedarian sin sujeto). El token ya no tiene ningun
// certificado sin clave, asi que el escenario de esta prueba no se puede
// reproducir con una referencia real; lo que cubria -un CKA_ID sin clave
// emparejada- lo siguen comprobando
// a_cka_id_that_is_not_in_the_token_says_so_instead_of_failing_generically (un
// CKA_ID que no esta en el token) y
// a_reference_without_a_cka_id_refuses_to_sign_instead_of_guessing_by_label
// (una referencia sin CKA_ID).
