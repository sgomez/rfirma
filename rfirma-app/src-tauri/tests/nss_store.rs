//! El almacén NSS de Mozilla —el perfil de Firefox— leído y firmado como lo
//! que es: **un módulo PKCS#11 más**. **Grada B** (ADR-0014, TD-02): carril
//! rápido, segundos.
//!
//! # Cómo se monta el perfil
//!
//! ```sh
//! sudo apt install -y libnss3 libnss3-tools
//! ```
//!
//! Y ya está: cada prueba se provisiona **su propio perfil desechable** en un
//! directorio temporal, con `testdata/nss/provision-profile.sh` y el material
//! público de la FNMT de `testdata/fnmt/`. El perfil real de Firefox de nadie
//! se toca jamás, ni se lee: aquí no aparece `~/.mozilla` por ninguna parte.
//!
//! Lo que hay dentro del perfil desechable:
//!
//! | apodo | qué tiene | para qué |
//! | --- | --- | --- |
//! | `EIDAS_CERTIFICADO_PRUEBAS___99999999R` | clave + certificado | en vigor: lista y firma |
//! | `EIDAS_CERTIFICADO_PRUEBAS___99999999R` | clave + certificado | caducó en 2020 |
//! | las dos CA de la FNMT | solo certificado | lo que un perfil de verdad tiene a cientos |
//!
//! Los dos primeros **comparten `CKA_LABEL`**: la FNMT le pone el mismo
//! `friendlyName` a los tres `.p12` del kit y NSS lo usa de apodo. No es un
//! defecto del material: es exactamente lo que hay en un perfil de Firefox de
//! verdad, donde dos claves privadas llevan la misma etiqueta, y es lo que hace
//! obligatorio emparejar por `CKA_ID` (#98, ID-06).
//!
//! # La contraseña maestra es la cadena vacía
//!
//! El perfil se crea con `certutil -N --empty-password`, que es el caso
//! corriente de un Firefox recién instalado. Para `C_Login` la cadena vacía
//! **no es lo mismo** que «sin PIN», y por eso este fichero incluye firmar y no
//! solo listar.

use std::path::{Path, PathBuf};
use std::process::Command;

use rfirma_lib::memory::ListedCertificates;
use rfirma_lib::pkcs11::{self, CertificateStatus, Situation, Store, StoreClass, TokenCertificate};
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;
use sha2::Sha256;
use x509_cert::der::{Decode, Encode};

/// El apodo que NSS le pone a los dos certificados de persona, sacado del
/// `friendlyName` de los `.p12` de la FNMT.
const HOLDER: &str = "EIDAS_CERTIFICADO_PRUEBAS___99999999R";
/// El token del perfil. Se llama igual en **todos** los perfiles del mundo, que
/// es justo por lo que la referencia necesita llevar también los init args.
const CERTIFICATE_DB: &str = "NSS Certificate DB";
/// La contraseña maestra de un Firefox recién instalado.
const NO_MASTER_PASSWORD: &str = "";

/// Lo mismo que firma el recorrido real: un bloque DER de `SignedAttributes`
/// que nadie ha hasheado antes de llegar aquí.
const PRESIGN: &[u8] = b"31 5f 30 18 06 09 2a 86 SignedAttributes de mentira, sin hashear";

fn softoken() -> PathBuf {
    pkcs11::stores::present_among(pkcs11::stores::CANDIDATE_SOFTOKENS, |path| path.is_file())
        .into_iter()
        .next()
        .expect(
            "falta libsoftokn3.so. Las pruebas de grada B del almacen NSS lo necesitan:\n  \
             sudo apt install -y libnss3 libnss3-tools",
        )
}

fn repository_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` es rfirma-app/src-tauri; la raiz esta dos arriba.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("la raiz del repositorio")
        .to_path_buf()
}

/// Un perfil NSS recién provisionado en un directorio temporal.
///
/// Devuelve el `TempDir` junto al almacén: en cuanto se suelte, el perfil
/// desaparece. Por eso las pruebas lo atan a una variable y no lo descartan.
fn a_disposable_profile() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let script = repository_root().join("testdata/nss/provision-profile.sh");

    let provisioned = Command::new("bash")
        .arg(&script)
        .arg(directory.path())
        .output()
        .expect("deberia poder ejecutarse el script de aprovisionamiento");

    assert!(
        provisioned.status.success(),
        "no se ha podido montar el perfil NSS con {}:\n{}\n{}",
        script.display(),
        String::from_utf8_lossy(&provisioned.stdout),
        String::from_utf8_lossy(&provisioned.stderr),
    );

    let store = Store::nss(softoken(), directory.path());
    (directory, store)
}

/// Lo mismo que [`a_disposable_profile`], pero con una contraseña maestra DE
/// VERDAD: el borde que el intento de sesión a ciegas antes de listar no
/// puede salvar sin PIN (ID-190, #259). Con ella `CKF_LOGIN_REQUIRED` pasa a
/// `true` y las tres entradas del perfil —los dos certificados de persona y
/// la CA suelta— dejan de verse sin sesión iniciada.
fn a_disposable_profile_with_a_master_password() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let script = repository_root().join("testdata/nss/provision-profile.sh");

    let provisioned = Command::new("bash")
        .arg(&script)
        .arg(directory.path())
        .arg("secreto")
        .output()
        .expect("deberia poder ejecutarse el script de aprovisionamiento");

    assert!(
        provisioned.status.success(),
        "no se ha podido montar el perfil NSS con {}:\n{}\n{}",
        script.display(),
        String::from_utf8_lossy(&provisioned.stdout),
        String::from_utf8_lossy(&provisioned.stderr),
    );

    let store = Store::nss(softoken(), directory.path());
    (directory, store)
}

fn certificates(store: &Store) -> Vec<TokenCertificate> {
    pkcs11::list_certificates(store).expect("el perfil NSS deberia listarse")
}

/// El certificado de persona que está en vigor. Se elige por **estado** y no por
/// etiqueta a propósito: los dos la comparten.
fn the_valid_one(store: &Store) -> TokenCertificate {
    certificates(store)
        .into_iter()
        .find(|certificate| certificate.status().is_usable())
        .expect("el perfil tenia que traer un certificado en vigor")
}

// ---------------------------------------------------------------------------
// Listar
// ---------------------------------------------------------------------------

/// La promesa entera del ID-01: NSS entra por el mismo `list_certificates` que
/// una tarjeta, y lo que devuelve son referencias con las mismas coordenadas.
#[test]
fn a_firefox_profile_is_listed_like_any_other_pkcs11_store() {
    let (_profile, store) = a_disposable_profile();

    let found = certificates(&store);

    assert!(
        found
            .iter()
            .any(|certificate| certificate.reference().label() == HOLDER),
        "el certificado del titular tenia que salir; salieron: {:?}",
        found
            .iter()
            .map(|certificate| certificate.reference().label())
            .collect::<Vec<_>>()
    );
    let valid = the_valid_one(&store);
    assert_eq!(valid.reference().token_label(), CERTIFICATE_DB);
    assert!(
        valid.reference().cka_id().is_some(),
        "sin CKA_ID no hay forma de emparejar el certificado con su clave"
    );
}

/// La prueba del #100: la lista solo trae certificados firmables. La CA suelta
/// del perfil no tiene clave privada y no sale; el del titular, que sí tiene,
/// sale. Sin esto, un perfil de Firefox de verdad enseñaría un centenar de
/// autoridades y certificados de páginas web por cada certificado firmable.
#[test]
fn a_certificate_without_a_private_key_is_filtered_out_and_the_holders_is_not() {
    let (_profile, store) = a_disposable_profile();

    let found = certificates(&store);

    assert!(
        found
            .iter()
            .any(|certificate| certificate.reference().label() == HOLDER),
        "el certificado del titular tiene clave y tenia que salir"
    );
    assert!(
        !found
            .iter()
            .any(|certificate| certificate.reference().label().contains("AC ")),
        "la CA suelta no tiene clave privada y no deberia salir en la lista firmable"
    );

    // Y sin pedir el PIN: el filtro busca la clave sin iniciar sesion (ID-08).
    // Si esto necesitase el PIN, no habria forma de decidir que enseñar antes
    // de pedirselo a nadie.
}

/// La vuelta atrás retirada (ID-190, #259): con una contraseña maestra DE
/// VERDAD, sin `C_Login` correcto no se ve ninguna `CKO_PRIVATE_KEY` —el
/// intento a ciegas antes de listar falla en silencio, `CKR_PIN_INCORRECT`
/// medido en `docs/research/token-flags-login.md`—, así que la lista sale
/// **vacía** en vez de traer las tres entradas del perfil con la CA dentro.
/// Antes de este cambio, la ranura sin ninguna clave visible se dejaba pasar
/// entera sin filtrar, y eso era justo el fallo que enseñaba ciento y pico
/// entradas en un Firefox de verdad.
#[test]
fn a_profile_with_a_real_master_password_lists_nothing_without_it() {
    let (_profile, store) = a_disposable_profile_with_a_master_password();

    let found = pkcs11::list_certificates(&store).expect("listar no deberia pedir la contrasena");

    assert!(
        found.is_empty(),
        "sin la contrasena maestra no deberia verse ninguna clave privada, \
         asi que la lista firmable tenia que salir vacia: {found:?}"
    );
}

/// El titular, el DNI y el emisor se leen del DER igual que en una tarjeta: no
/// hay una segunda implementación para NSS.
#[test]
fn the_holder_and_the_issuer_are_read_the_same_as_from_a_card() {
    let (_profile, store) = a_disposable_profile();

    let certificate = the_valid_one(&store);

    let subject = certificate.subject().expect("el DER deberia leerse");
    assert!(
        subject.contains("EIDAS CERTIFICADO PRUEBAS"),
        "titular leido: {subject}"
    );
    assert!(
        subject.contains("IDCES-99999999R"),
        "el DNI tenia que leerse del subject: {subject}"
    );
    let issuer = certificate.issuer().expect("el DER deberia leerse");
    assert!(
        issuer.contains("AC FNMT Usuarios"),
        "emisor leido: {issuer}"
    );
}

/// Un certificado caducado de NSS sale marcado como caducado, igual que uno de
/// tarjeta, y **sin** haber pedido ningún secreto para saberlo.
#[test]
fn an_expired_certificate_from_nss_is_marked_as_expired() {
    let (_profile, store) = a_disposable_profile();

    let expired: Vec<_> = certificates(&store)
        .into_iter()
        .filter(|certificate| matches!(certificate.status(), CertificateStatus::Expired { .. }))
        .collect();

    assert!(
        expired
            .iter()
            .any(|certificate| certificate.reference().label() == HOLDER),
        "el certificado que caduco en 2020 tenia que salir marcado como caducado"
    );
}

/// Los dos certificados de persona comparten etiqueta y no comparten `CKA_ID`.
/// Es lo que hay en un perfil de verdad, y es lo que obliga al ID-06.
#[test]
fn two_certificates_share_the_label_and_are_told_apart_by_their_cka_id() {
    let (_profile, store) = a_disposable_profile();

    let same_label: Vec<_> = certificates(&store)
        .into_iter()
        .filter(|certificate| certificate.reference().label() == HOLDER)
        .collect();

    assert_eq!(
        same_label.len(),
        2,
        "el perfil tenia que traer los dos certificados del titular"
    );
    assert_ne!(
        same_label[0].reference().cka_id(),
        same_label[1].reference().cka_id(),
        "dos certificados con la misma etiqueta tienen que distinguirse por CKA_ID"
    );
}

/// Con las etiquetas repetidas de un perfil de verdad, cada fila tiene que
/// llevar a **su** certificado: buscando por etiqueta se cogía siempre el
/// primero, y el segundo era inelegible aunque se enseñara en la lista.
#[test]
fn each_certificate_sharing_a_label_comes_back_by_its_own_handle() {
    let (_profile, store) = a_disposable_profile();
    let found = certificates(&store);
    let listed = ListedCertificates::new();

    let handles = listed.replace(
        found
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );

    let holders: Vec<(&String, &TokenCertificate)> = handles
        .iter()
        .zip(found.iter())
        .filter(|(_, certificate)| certificate.reference().label() == HOLDER)
        .collect();
    assert_eq!(holders.len(), 2, "el perfil tenia que traer los dos");
    assert_ne!(holders[0].0, holders[1].0, "dos filas, dos asas");
    for (handle, certificate) in holders {
        assert_eq!(listed.get(handle).as_ref(), Some(certificate.reference()));
    }
}

/// El asa de un certificado de NSS no lleva dentro el `configdir` del perfil,
/// que es lo que de verdad hay que no puede cruzar (ADR-0011).
#[test]
fn the_handle_carries_nothing_of_the_profile_it_came_from() {
    let (profile, store) = a_disposable_profile();
    let listed = ListedCertificates::new();

    let handles = listed.replace([the_valid_one(&store).reference().clone()]);

    let handle = &handles[0];
    assert_eq!(handle.len(), 32);
    assert!(handle.chars().all(|letter| letter.is_ascii_hexdigit()));
    for leak in ["/", "tmp", HOLDER, CERTIFICATE_DB] {
        assert!(!handle.contains(leak), "el asa «{handle}» lleva «{leak}»");
    }
    let name = profile
        .path()
        .file_name()
        .expect("el perfil tiene nombre")
        .to_string_lossy()
        .into_owned();
    assert!(!handle.contains(&name), "el asa lleva el nombre del perfil");
}

/// La clase del almacén es lo único suyo que puede cruzar, y sale de **de quién
/// es el perfil**: uno bajo `~/.mozilla/firefox` es de Firefox, uno en
/// `~/.pki/nssdb` es de Chrome, y este —desechable, en un temporal— no se
/// atribuye a nadie.
#[test]
fn an_nss_store_says_its_class_and_never_its_configdir() {
    let (profile, store) = a_disposable_profile();

    let class = the_valid_one(&store).reference().store().class();

    assert_eq!(class, StoreClass::Nssdb);
    assert_eq!(
        Store::nss(
            softoken(),
            &profile.path().join(".mozilla/firefox/x.default")
        )
        .class(),
        StoreClass::Firefox
    );
    assert_eq!(
        Store::nss(softoken(), &profile.path().join(".pki/nssdb")).class(),
        StoreClass::Chrome
    );
}

/// Varios perfiles no es un caso excepcional (ID-05), y es donde se rompería una
/// implementación que cachease el contexto del módulo por su ruta: los dos
/// perfiles se abren por el **mismo** `libsoftokn3.so`, así que un contexto
/// reutilizado devolvería el primer perfil las dos veces.
#[test]
fn two_profiles_are_read_as_two_stores_and_not_as_the_first_one_twice() {
    let (_first, one) = a_disposable_profile();
    let (_second, other) = a_disposable_profile();

    let both = pkcs11::list_certificates_across(&[one.clone(), other.clone()])
        .expect("los dos perfiles tenian que listarse");
    let alone = certificates(&one);

    assert_eq!(
        both.len(),
        alone.len() * 2,
        "dos perfiles tienen que dar el doble de certificados que uno"
    );
    assert_ne!(one.init_args(), other.init_args());
}

/// La comprobación explícita que pide el spec. Abrir softoken con init args que
/// no llevan a ningún perfil no da un fallo fiable: unas veces devuelve un
/// `CKR_*` opaco y otras dos ranuras que se anuncian como `token initialized`
/// con cero objetos dentro. Las dos caras se ven desde arriba igual que un
/// Firefox recién instalado, así que lo que no puede pasar es que salga una
/// lista vacía.
#[test]
fn opening_the_store_with_the_wrong_init_args_is_a_failure_and_not_an_empty_list() {
    let nowhere = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let store = Store::nss(softoken(), nowhere.path());

    let error = pkcs11::list_certificates(&store)
        .expect_err("un almacen abierto donde no hay perfil no puede parecer un perfil vacio");

    assert_eq!(error.situation(), Situation::ModuleNotFound);
    assert!(
        error.detail().contains("configdir"),
        "el detalle tiene que decir con que init args se iba a abrir: {}",
        error.detail()
    );
}

/// Y la otra mitad del ID-03: un perfil que no lleva a ninguna parte no puede
/// dejar sin certificados a quien sí tiene otro almacén.
#[test]
fn a_profile_that_leads_nowhere_does_not_hide_the_one_that_works() {
    let (_profile, works) = a_disposable_profile();
    let nowhere = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");

    let found =
        pkcs11::list_certificates_across(&[Store::nss(softoken(), nowhere.path()), works.clone()])
            .expect("el perfil que si abre tiene que seguir contando");

    assert!(found
        .iter()
        .any(|certificate| certificate.reference().label() == HOLDER));
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

/// El criterio de aceptación central: se firma con el certificado de Firefox y
/// la firma verifica contra **su** clave pública.
///
/// La contraseña maestra que se pasa es la cadena vacía, y llega hasta
/// `C_Login` como tal: si en algún punto del camino se convirtiera en «sin
/// PIN», softoken no autenticaría y esto se pondría rojo.
#[test]
fn signing_with_an_nss_certificate_verifies_against_its_public_key() {
    let (_profile, store) = a_disposable_profile();
    let certificate = the_valid_one(&store);

    let raw = pkcs11::sign(certificate.reference(), NO_MASTER_PASSWORD, PRESIGN)
        .expect("un perfil sin contrasena maestra tiene que poder firmar con la cadena vacia");

    // RSA 2048: la firma cruda mide exactamente el modulo.
    assert_eq!(raw.len(), 256);
    let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
    verifying_key(&certificate)
        .verify(PRESIGN, &signature)
        .expect("la firma no verifica contra la clave publica del certificado");
}

/// El certificado se reencuentra **desde lo que se persiste** (ADR-0010): la
/// referencia se serializa y se vuelve a leer, y con eso solo se firma.
///
/// Sin los init args dentro de la referencia esto no se puede cumplir: el
/// módulo y la etiqueta del token son iguales en todos los perfiles de Firefox
/// de la máquina, y una referencia recordada no sabría a cuál volver.
#[test]
fn a_remembered_nss_certificate_still_signs_after_a_round_trip_through_the_state_file() {
    let (_profile, store) = a_disposable_profile();
    let certificate = the_valid_one(&store);

    let written = serde_json::to_string(certificate.reference()).expect("deberia serializarse");
    let remembered: pkcs11::CertificateRef =
        serde_json::from_str(&written).expect("deberia leerse");

    let raw = pkcs11::sign(&remembered, NO_MASTER_PASSWORD, PRESIGN)
        .expect("la referencia recordada tenia que volver a encontrar su perfil");

    let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
    verifying_key(&certificate)
        .verify(PRESIGN, &signature)
        .expect("la firma no verifica contra la clave publica del certificado");
}

/// Firmar con un certificado que **no** tiene clave privada en el perfil —una
/// CA suelta, de las que un Firefox de verdad tiene a cientos— se rechaza
/// diciendo qué falta, no con un fallo genérico.
///
/// La CA ya no sale de `certificates()`: el filtro de #100 la descarta
/// justamente porque no tiene clave, que es lo que esta prueba quiere
/// comprobar. Su referencia sale de
/// [`pkcs11::list_certificates_unfiltered_for_test`], el escape que existe
/// para este caso.
#[test]
fn a_certificate_without_a_private_key_says_so_instead_of_failing_generically() {
    let (_profile, store) = a_disposable_profile();
    let authority = pkcs11::list_certificates_unfiltered_for_test(&store)
        .expect("el perfil deberia listarse sin filtrar")
        .into_iter()
        .find(|certificate| certificate.reference().label().contains("AC "))
        .expect("el perfil tenia que traer alguna CA suelta");

    let error = pkcs11::sign(authority.reference(), NO_MASTER_PASSWORD, PRESIGN)
        .expect_err("una CA no tiene con que firmar");

    assert_eq!(error.situation(), Situation::CertificateNotFound);
}
