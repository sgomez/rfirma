//! La capa que habla con el token PKCS#11: la **única** parte de rfirma que
//! toca la clave privada.
//!
//! Hace tres cosas y ninguna más: [`list_certificates`] enumera lo que hay en el
//! token, [`sign`] delega en él la firma de un bloque de bytes, y
//! [`error`](self::error) traduce sus códigos a situaciones que una persona
//! entienda.
//!
//! Es la **fase 2** de la firma trifásica (ID-14, ADR-0001): Java hace la
//! prefirma y la postfirma, y la clave privada nunca sale del token ni llega al
//! isolate de GraalVM. Aquí no hay PDF, ni CAdES, ni orquestación de fases: eso
//! es de otros módulos.
//!
//! # El mecanismo es contraintuitivo
//!
//! Se firma con `CKM_SHA256_RSA_PKCS` y se le pasan los bytes **sin hashear**:
//! hashea él, construye el `DigestInfo` y aplica el relleno (ID-16). Firmar un
//! hash con `CKM_RSA_PKCS` produce una firma matemáticamente válida que ningún
//! validador CAdES/PAdES reconoce, y el PDF *parece* firmado. La razón completa,
//! con las citas del cliente original, está en
//! `docs/research/pkcs11-mecanismo-firma.md`.

pub mod certificate;
pub mod error;
pub mod secret;
pub mod stores;

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock};

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::error::{Error, RvError};
use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, ObjectClass};
use cryptoki::session::{Session, UserType};
use cryptoki::slot::Slot;
use cryptoki::types::AuthPin;

pub use certificate::{CertificateRef, CertificateStatus, TokenCertificate};
pub use error::{Situation, TokenError};
pub use secret::{SecretOnTheReaderKeypad, StoreSecret};
pub use stores::{Store, StoreClass};

/// El mecanismo del ID-16, en un solo sitio para que cambiarlo sea un cambio
/// visible y no un descuido en una llamada perdida.
const SIGNING_MECHANISM: Mechanism<'static> = Mechanism::Sha256RsaPkcs;

/// Los certificados de **todos** los almacenes, concatenados.
///
/// Un almacén que no cargue no tumba a los demás (ID-03): no tener Firefox
/// instalado no puede dejar sin tarjeta a quien la tiene. Pero el fallo no se
/// tira a la basura: si **ningún** almacén ha llegado a abrirse, lo que se
/// cuenta es **ese** fallo, y no un «no hay ninguno» que sería mentira. Sin
/// almacenes tampoco se calla: quedarse sin dónde buscar es un fallo de
/// configuración, no una lista vacía.
///
/// Lo que decide si hubo fallo es **si algún almacén cargó**, no si la lista
/// final quedó vacía: desde el filtro de certificados firmables (#100), un
/// almacén puede abrirse bien y no tener ningún certificado firmable que
/// enseñar —un perfil de Firefox con solo autoridades de certificación, por
/// ejemplo—, y eso no es un fallo de ningún otro almacén que sí haya
/// fallado al cargar.
pub fn list_certificates_across(stores: &[Store]) -> Result<Vec<TokenCertificate>, TokenError> {
    if stores.is_empty() {
        return Err(TokenError::new(
            Situation::ModuleNotFound,
            "no hay ningun modulo PKCS#11 donde buscar certificados",
        ));
    }

    let mut found = Vec::new();
    let mut any_loaded = false;
    let mut refused: Option<TokenError> = None;

    for store in stores {
        match list_certificates(store) {
            Ok(certificates) => {
                any_loaded = true;
                found.extend(certificates);
            }
            Err(error) => refused = refused.or(Some(error)),
        }
    }

    match refused {
        Some(error) if !any_loaded => Err(error),
        _ => Ok(found),
    }
}

/// Los certificados **firmables** que hay en cualquier token del módulo, sin
/// iniciar sesión.
///
/// No pide el PIN a propósito: los certificados son objetos públicos, y su
/// estado —caducado, en vigor— tiene que poder decidirse **antes** de que a
/// nadie se le pida nada.
///
/// La lista se filtra a los que tienen una clave privada emparejada por
/// `CKA_ID` en el mismo token (ID-07): un perfil de Firefox corriente trae
/// más de cien certificados y solo un puñado son firmables, el resto son
/// autoridades y certificados de páginas web. **En una tarjeta sin sesión el
/// filtro no se aplica** ([`signable_certificates`]): ahí no hay nada que
/// esconder porque todo lo que hay ya es firmable, y sin sesión iniciada una
/// tarjeta corriente —SoftHSM entre ellas, comprobado— no enseña ninguna
/// `CKO_PRIVATE_KEY`; filtrar igual la dejaría sin listar nada. Si la tarjeta
/// sí acaba con sesión iniciada (ver más abajo), el filtro se aplica también
/// a ella.
///
/// En un almacén NSS sí se aplica siempre, y **sin vuelta atrás** (ID-190):
/// antes, si la ranura no enseñaba ninguna clave visible, se devolvía el
/// almacén entero sin filtrar, y esa vuelta atrás era la causa real de que un
/// perfil de Firefox con contraseña maestra enseñase sus ciento y pico
/// entradas con las CA dentro. Ahora, sin sesión iniciada, esa ranura se
/// queda sin certificados firmables. Antes de filtrar, cada ranura que exige
/// sesión y no tiene teclado propio la inicia a ciegas —sin PIN que ofrecer,
/// ver [`log_in_before_listing`]—; el intento solo tiene éxito en el almacén
/// sin protección real que describe ID-195 —un perfil NSS sin contraseña
/// maestra, o el NSS propio de un `.p12` instalado—, así que contra una
/// contraseña maestra de verdad falla en silencio, sin propagar el error: no
/// hay PIN que pedir aquí, y `CKR_OK` de una tarjeta compartiendo el turno
/// con el mismo módulo tampoco es un fallo de esta ranura. El perfil
/// desechable de las pruebas se monta con `--empty-password`, que es justo el
/// caso en el que las claves ya se ven sin necesitar sesión
/// (`CKF_LOGIN_REQUIRED` a `false`), así que ninguna prueba de grada B
/// ejercita el intento a ciegas contra un token que de verdad lo rechace
/// —medido contra SoftHSM y contra un perfil NSS con contraseña real en
/// `docs/research/token-flags-login.md`—.
pub fn list_certificates(store: impl Into<Store>) -> Result<Vec<TokenCertificate>, TokenError> {
    let store = store.into();
    // Bajo el mismo turno que la firma: abrir un almacén NSS es inicializar el
    // módulo con **su** perfil, y dos almacenes a la vez sobre el mismo
    // `libsoftokn3.so` se pisarían.
    with_token_turn(|| list_holding_the_turn(&store))
}

fn list_holding_the_turn(store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
    the_store_is_really_there(store)?;
    let context = context(store)?;
    let mut found = Vec::new();

    for slot in usable_slots(&context)? {
        let info = context.get_token_info(slot)?;
        let token_label = info.label().trim().to_owned();
        let session = context.open_ro_session(slot)?;

        // ID-190, segunda mitad: si la ranura exige sesión, se inicia antes de
        // listar. Aquí no hay PIN que ofrecer —listar no lo pide, ver la nota
        // de [`list_certificates`]—, así que el intento es a ciegas y solo
        // tiene éxito en el almacén sin protección real que describe ID-195.
        let logged_in = log_in_before_listing(&session, &info);
        // El `?` de abajo, si `signable_certificates` falla, se lleva la
        // función sin pasar por el `logout()` de más abajo. No hace falta
        // repetir aquí el «salga bien o mal» de `sign_holding_the_turn`: el
        // `Drop` de `Session` cierra la única sesión de la ranura al salir de
        // este bucle, y con ella el estado de login que hubiera quedado.
        found.extend(signable_certificates(
            &session,
            store,
            &token_label,
            logged_in,
        )?);
        if logged_in {
            // Cerrada en cuanto ha servido para listar: dejarla abierta sería
            // desbloquear el token para todo el proceso sin que nadie lo haya
            // pedido (mismo motivo que el logout de sign_holding_the_turn).
            let _ = session.logout();
        }
    }

    Ok(found)
}

/// Inicia sesión antes de listar cuando la ranura la exige (ID-190).
///
/// Sin ella, `private_key_ids` no ve ninguna `CKO_PRIVATE_KEY` en un token que
/// las protege, y antes de este cambio eso disparaba la vuelta atrás que aquí
/// se retira: «sin claves visibles, devuelvo todos los certificados». Ahora,
/// sin sesión iniciada, la ranura simplemente se queda sin certificados
/// firmables.
///
/// El intento de `C_Login` es a ciegas, sin secreto (`login(User, None)`):
/// solo tiene éxito en el almacén sin protección real que describe ID-195 —un
/// perfil NSS sin contraseña maestra, o el NSS propio de un `.p12`
/// instalado—. En cualquier otro —una contraseña maestra de verdad, o un
/// módulo como SoftHSM que exige el PIN incluso para el intento vacío— falla
/// en silencio: no hay PIN que pedir aquí, y propagar el error dejaría sin
/// listar nada a quien tiene certificados en otro almacén.
///
/// **Se salta por completo cuando la ranura tiene teclado propio**
/// (`protected_authentication_path`, ID-189): ahí `login(User, None)` no
/// falla en silencio, cede el turno al pinpad y se queda bloqueada hasta que
/// se teclee el PIN o el lector expire —justo lo que
/// `docs/research/token-flags-login.md` documenta—. `list_certificates` es el
/// camino de arranque, y colgarlo sobre un teclado sin nada en pantalla que
/// lo explique va en contra del ID-190 y del ID-195.
fn log_in_before_listing(session: &Session, info: &cryptoki::slot::TokenInfo) -> bool {
    if !should_attempt_blind_login(info.login_required(), info.protected_authentication_path()) {
        return false;
    }

    match session.login(UserType::User, None) {
        Ok(()) => true,
        // Ya autenticados contra este token por otra biblioteca del mismo
        // proceso: no hay nada que iniciar, y tampoco nada que cerrar luego.
        // Coincide a propósito con el `Err(_)` de abajo —aquí no hay PIN
        // distinto que reintentar, a diferencia de `sign_holding_the_turn`,
        // donde este mismo error sí separa un camino real—: se deja el brazo
        // explícito para que quien lo lea no dé por hecho que el caso se
        // olvidó.
        Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => false,
        Err(_) => false,
    }
}

/// La decisión de [`log_in_before_listing`], separada de `Session` y
/// `TokenInfo` —ninguno de los dos se puede construir fuera de un módulo
/// PKCS#11 real— para poder fijarla con una prueba de unidad.
fn should_attempt_blind_login(login_required: bool, protected_authentication_path: bool) -> bool {
    login_required && !protected_authentication_path
}

/// Los certificados de una ranura ya abierta que tienen clave privada
/// emparejada por `CKA_ID` en el mismo token (ID-07).
///
/// El filtro **no se aplica a una tarjeta sin sesión** ([`StoreClass::Card`]):
/// ahí no hay nada que esconder porque todo lo que hay ya es firmable —una
/// tarjeta o SoftHSM no traen autoridades sueltas mezcladas con las claves de
/// la persona—, y sin sesión iniciada una tarjeta corriente (SoftHSM entre
/// ellas, comprobado) no enseña ninguna `CKO_PRIVATE_KEY`: filtrar igual la
/// dejaría sin listar nada, que es justo el fallo silencioso que
/// [`list_certificates`] existe para evitar. **Cuando `log_in_before_listing`
/// sí ha conseguido sesión** (`logged_in`), el filtro se aplica también a la
/// tarjeta: las CA sueltas que algunas tarjetas llevan dentro pasarían sin
/// filtrar si no, y eso es justo el ID-07 que este filtro existe para cubrir.
///
/// En un almacén NSS sí se aplica siempre, sin vuelta atrás (ID-190): antes,
/// si la ranura no enseñaba ninguna clave visible, se devolvía el almacén
/// entero sin filtrar, y esa vuelta atrás era la causa real de que un perfil
/// de Firefox con contraseña maestra enseñase sus CA sueltas. Ahora, sin
/// sesión iniciada, la ranura NSS simplemente se queda sin certificados
/// firmables.
fn signable_certificates(
    session: &Session,
    store: &Store,
    token_label: &str,
    logged_in: bool,
) -> Result<Vec<TokenCertificate>, TokenError> {
    if store.class() == StoreClass::Card && !logged_in {
        return all_certificates_in_session(session, store, token_label);
    }

    let visible_private_keys = private_key_ids(session)?;

    let mut found = Vec::new();
    for certificate in all_certificates_in_session(session, store, token_label)? {
        // Sin clave privada con su mismo CKA_ID en el mismo token, no se puede
        // firmar con él: es justo lo que este filtro (ID-07) tiene que
        // descartar.
        if certificate
            .reference()
            .cka_id()
            .is_some_and(|cka_id| visible_private_keys.contains(cka_id))
        {
            found.push(certificate);
        }
    }

    Ok(found)
}

/// Los `CKA_ID` de las `CKO_PRIVATE_KEY` visibles en la sesión, en una sola
/// consulta al token.
///
/// Antes esto era un `find_objects` por certificado —137 en el perfil de
/// Firefox de 137 entradas que motiva el #100—; leer las claves de la ranura
/// de una vez a un conjunto cuesta una sola consulta y de paso es lo que hace
/// falta para distinguir «esta ranura no tiene ninguna clave visible» de «esta
/// ranura tiene claves y esta en concreto no empareja».
///
/// No lee ningún atributo protegido de la clave, solo su `CKA_ID`: la
/// existencia de la clave no es un secreto, solo su valor lo es (ID-08).
fn private_key_ids(session: &Session) -> Result<HashSet<Vec<u8>>, TokenError> {
    let mut ids = HashSet::new();

    for object in session.find_objects(&[Attribute::Class(ObjectClass::PRIVATE_KEY)])? {
        // Una clave que el token enumera pero cuyo CKA_ID no deja leer se
        // salta, no se propaga: el módulo PKCS#11 lo pone el fabricante de la
        // tarjeta, y el peor caso de saltarla es un certificado de más en la
        // lista, mientras que el de propagar es que no se pueda listar nada
        // —ni en esta ranura ni en los almacenes que vengan detrás— con un
        // error que habla del atributo de una clave.
        let Ok(attributes) = session.get_attributes(object, &[AttributeType::Id]) else {
            continue;
        };

        for attribute in attributes {
            // Un CKA_ID vacío no empareja nada: es el mismo caso que un
            // certificado sin CKA_ID, ver all_certificates_in_session.
            if let Attribute::Id(bytes) = attribute {
                if !bytes.is_empty() {
                    ids.insert(bytes);
                }
            }
        }
    }

    Ok(ids)
}

/// Los certificados de una ranura ya abierta, **sin** aplicar el filtro de
/// clave privada.
///
/// Compartida entre [`signable_certificates`] y
/// [`list_certificates_unfiltered_for_test`], que existe para que las pruebas
/// puedan construir la referencia a un certificado que el filtro de #100 deja
/// fuera del listado a propósito —una CA suelta, por ejemplo— y comprobar qué
/// pasa al intentar firmar con ella.
fn all_certificates_in_session(
    session: &Session,
    store: &Store,
    token_label: &str,
) -> Result<Vec<TokenCertificate>, TokenError> {
    let mut found = Vec::new();

    for object in session.find_objects(&[Attribute::Class(ObjectClass::CERTIFICATE)])? {
        let attributes = session.get_attributes(
            object,
            &[
                AttributeType::Label,
                AttributeType::Value,
                AttributeType::Id,
            ],
        )?;

        let mut label = None;
        let mut der = None;
        let mut cka_id = None;
        for attribute in attributes {
            match attribute {
                Attribute::Label(bytes) => {
                    label = Some(String::from_utf8_lossy(&bytes).trim().to_owned())
                }
                Attribute::Value(bytes) => der = Some(bytes),
                // Un CKA_ID vacío no empareja nada: es lo mismo que no
                // tenerlo, y guardarlo como si lo tuviera haría que la
                // búsqueda de la clave devolviese la primera que pasara.
                Attribute::Id(bytes) if !bytes.is_empty() => cka_id = Some(bytes),
                _ => {}
            }
        }

        // Un certificado sin CKA_LABEL no se puede reencontrar por etiqueta,
        // que es lo único que el ADR-0010 deja persistir, así que no se
        // ofrece: enseñarlo sería prometer algo que no podemos cumplir.
        if let (Some(label), Some(der)) = (label, der) {
            if !label.is_empty() {
                found.push(TokenCertificate::new(
                    CertificateRef::new(store, token_label, label, cka_id),
                    der,
                ));
            }
        }
    }

    Ok(found)
}

/// Lo mismo que [`list_certificates`], pero sin aplicar el filtro de clave
/// privada (ID-07).
///
/// Existe solo para las pruebas: ninguna referencia a un certificado sin clave
/// —una CA suelta de un perfil NSS, por ejemplo— sale ya de
/// [`list_certificates`], así que una prueba que quiera comprobar qué pasa al
/// pedirle una firma a una de ellas necesita otra forma de conseguir esa
/// referencia. No hace falta sesión: como [`list_certificates`], esto tampoco
/// pide el PIN.
#[doc(hidden)]
pub fn list_certificates_unfiltered_for_test(
    store: impl Into<Store>,
) -> Result<Vec<TokenCertificate>, TokenError> {
    let store = store.into();
    with_token_turn(|| {
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let mut found = Vec::new();

        for slot in usable_slots(&context)? {
            let token_label = context.get_token_info(slot)?.label().trim().to_owned();
            let session = context.open_ro_session(slot)?;
            found.extend(all_certificates_in_session(&session, &store, &token_label)?);
        }

        Ok(found)
    })
}

/// La comprobación explícita que separa «no hay certificados» de «lo he abierto
/// mal».
///
/// Es el aviso que el #95 dejó escrito, y está medido: cuando a softoken se le
/// da un `configdir` que no lleva a ningún perfil **no siempre falla**. A veces
/// devuelve un `CKR_*` opaco desde `C_Initialize`, y a veces abre dos ranuras
/// que se anuncian como `token initialized` y no tienen ni un objeto dentro —es
/// lo que hace, sin más, cuando no se le pasan init args—. Las dos caras se ven
/// desde arriba igual que un Firefox recién instalado, que es justo la
/// confusión que este spec vino a cerrar.
///
/// Así que no se le pregunta a softoken: se mira si hay `cert9.db` detrás del
/// `configdir` **antes** de abrir nada. La respuesta es la misma siempre, y el
/// mensaje dice con qué init args se iba a abrir en vez de un código de error.
fn the_store_is_really_there(store: &Store) -> Result<(), TokenError> {
    let Some(init_args) = store.init_args() else {
        // Un módulo de tarjeta no lleva init args y no hay nada que comprobar:
        // un token vacío es una situación legítima, no una mala configuración.
        return Ok(());
    };

    let profile = configured_directory(init_args);
    if profile.is_some_and(|directory| Path::new(directory).join("cert9.db").is_file()) {
        return Ok(());
    }

    Err(TokenError::new(
        Situation::ModuleNotFound,
        format!(
            "los init args «{init_args}» no llevan a ningun perfil NSS: \
             detras del configdir no hay ningun cert9.db"
        ),
    ))
}

/// El `configdir` de unos init args, ya sin el prefijo `sql:` ni las comillas.
fn configured_directory(init_args: &str) -> Option<&str> {
    let value = init_args.split("configdir=").nth(1)?;
    let value = value.strip_prefix('\'')?;
    let value = value.split('\'').next()?;
    Some(value.strip_prefix("sql:").unwrap_or(value))
}

/// **Cómo hay que pedirle el secreto** al almacén de donde salió el
/// certificado, leyendo las banderas de su ranura (ID-189).
///
/// No inicia sesión ni la pide: `C_GetTokenInfo` es una consulta pública, y por
/// eso esto se puede preguntar **antes** del diálogo, que es justamente lo que
/// permite decidir si hace falta diálogo.
///
/// Aun así abre un almacén, y eso sí pasa por [`with_token_turn`]: para un
/// perfil NSS con init args `context()` no cachea, así que el `Arc<Pkcs11>` que
/// nace aquí llama a `C_Finalize` sobre el módulo al salir de la función y le
/// tiraría la sesión a quien estuviese dentro de su turno.
pub fn store_secret(reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
    with_token_turn(|| {
        let store = reference.store();
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let slot = slot_of(&context, reference.token_label())?;
        let info = context.get_token_info(slot)?;
        Ok(StoreSecret::of_token(
            info.login_required(),
            info.protected_authentication_path(),
        ))
    })
}

/// Firma `data` con la clave privada que acompaña al certificado referenciado.
///
/// `data` son los bytes **sin hashear** —los DER de `PRE` en el recorrido
/// trifásico— y lo que vuelve es la firma **cruda**, sin codificar: quien la
/// meta en el campo `PK1` es quien la pasa a Base64.
pub fn sign(reference: &CertificateRef, pin: &str, data: &[u8]) -> Result<Vec<u8>, TokenError> {
    with_token_turn(|| sign_holding_the_turn(reference, pin, data))
}

/// El turno del token: **todo** lo que inicie sesión contra él pasa por aquí.
///
/// El estado de sesión iniciada de PKCS#11 es *del token dentro del proceso*,
/// no de la sesión: dos inicios de sesión a la vez se pisan, y el segundo
/// recibe `CKR_USER_ALREADY_LOGGED_IN` en vez de autenticar. Peor: un
/// `logout()` de un lado cierra la sesión autenticada del otro.
///
/// Está público —y oculto de la documentación— porque las pruebas de
/// integración de `tests/pkcs11_token.rs` corren en hilos del mismo proceso y
/// una de ellas inicia sesión a mano, como contraejemplo del mecanismo del
/// ID-16. Sin este turno compartido, ese contraejemplo se cruza con las firmas
/// de verdad y pone el carril rápido rojo de forma intermitente. **No es
/// reentrante**: nunca llames a [`sign`] ni a [`list_certificates`] desde
/// dentro del cierre.
#[doc(hidden)]
pub fn with_token_turn<T>(operation: impl FnOnce() -> T) -> T {
    static TURN: Mutex<()> = Mutex::new(());
    let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    operation()
}

fn sign_holding_the_turn(
    reference: &CertificateRef,
    pin: &str,
    data: &[u8],
) -> Result<Vec<u8>, TokenError> {
    let store = reference.store();
    the_store_is_really_there(&store)?;
    let context = context(&store)?;
    let slot = slot_of(&context, reference.token_label())?;
    let session = context.open_ro_session(slot)?;

    match session.login(UserType::User, Some(&AuthPin::new(pin.into()))) {
        Ok(()) => {}
        // Ya autenticados contra este token: el PIN no hace falta otra vez.
        //
        // Ojo con lo que eso implica: por esta rama **el PIN que ha escrito la
        // persona no se comprueba**, se firma con la sesión que abrió otro.
        // Dentro de rfirma es inalcanzable —`with_token_turn` serializa y el
        // `logout()` de abajo se ejecuta salga bien o mal—, así que solo llega
        // aquí si **otra biblioteca del mismo proceso** ha iniciado sesión
        // contra este token. Tratarlo como fallo dejaría sin firmar a quien
        // tenga el token compartido; tratarlo como éxito es lo que hacemos, y
        // el precio es ese PIN sin validar. Cuando llegue el diálogo del PIN,
        // esta es la distinción que hay que tener presente.
        Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => {}
        Err(other) => return Err(other.into()),
    }

    let signature = private_key(&session, reference).and_then(|key| {
        session
            .sign(&SIGNING_MECHANISM, key, data)
            .map_err(TokenError::from)
    });

    // Se cierra la sesión autenticada en cuanto deja de hacer falta, salga bien
    // o mal: dejarla abierta es dejar el token desbloqueado para todo el proceso.
    let _ = session.logout();

    signature
}

/// Las ranuras con un token **ya inicializado**.
///
/// No vale `get_slots_with_token`: SoftHSM anuncia también su ranura libre, y un
/// lector de tarjetas anuncia la suya vacía. Abrir sesión contra ellas devuelve
/// `CKR_TOKEN_NOT_RECOGNIZED` y convertiría «hay un lector sin tarjeta» en un
/// fallo del programa.
fn usable_slots(context: &Pkcs11) -> Result<Vec<Slot>, TokenError> {
    Ok(context.get_slots_with_initialized_token()?)
}

fn slot_of(context: &Pkcs11, token_label: &str) -> Result<Slot, TokenError> {
    for slot in usable_slots(context)? {
        if context.get_token_info(slot)?.label().trim() == token_label {
            return Ok(slot);
        }
    }
    Err(TokenError::new(
        Situation::TokenAbsent,
        format!("no hay ningun token etiquetado {token_label}"),
    ))
}

/// La clave privada del certificado, buscada por `CKA_ID` y **nunca** por
/// etiqueta (ID-06).
///
/// La diferencia no es de estilo. En un perfil de Firefox de verdad hay dos
/// claves privadas distintas con la **misma** `CKA_LABEL`, así que buscar por
/// etiqueta devuelve una de las dos arbitrariamente y se firma con una clave que
/// no es la del certificado elegido. La firma sale, verifica contra otra clave
/// pública, y nadie se entera. PKCS#11 empareja certificado y clave por
/// `CKA_ID` —es lo que hace el propio NSS— y aquí se hace lo mismo.
///
/// Una referencia sin `CKA_ID` —recordada por una versión anterior al #98— no
/// se resuelve por etiqueta como respaldo: eso sería volver justo al fallo que
/// esto cierra. Se rechaza, y quien la tenga la recupera volviendo a listar el
/// token, que devuelve la referencia completa.
fn private_key(
    session: &Session,
    reference: &CertificateRef,
) -> Result<cryptoki::object::ObjectHandle, TokenError> {
    let label = reference.label();
    let cka_id = reference.cka_id().ok_or_else(|| {
        TokenError::new(
            Situation::CertificateNotFound,
            format!("la referencia a {label} no lleva CKA_ID: vuelve a listar el token"),
        )
    })?;

    session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Id(cka_id.to_vec()),
        ])?
        .into_iter()
        .next()
        .ok_or_else(|| {
            TokenError::new(
                Situation::CertificateNotFound,
                format!("el token no tiene ninguna clave privada con el CKA_ID de {label}"),
            )
        })
}

/// El contexto de un módulo, cacheado para todo el proceso.
///
/// No es una optimización: `C_Initialize` **solo se puede llamar una vez por
/// proceso y módulo**, y la segunda devuelve `CKR_CRYPTOKI_ALREADY_INITIALIZED`.
/// Como el `.so` lo comparte todo el proceso vía `dlopen`, abrir un contexto por
/// operación rompería en cuanto haya dos.
fn context(store: &Store) -> Result<Arc<Pkcs11>, TokenError> {
    static MODULES: OnceLock<Mutex<HashMap<PathBuf, Arc<Pkcs11>>>> = OnceLock::new();

    // Un almacén con init args **no se cachea**, y esto es lo contrario de una
    // optimización perdida.
    //
    // Los init args se le pasan a `C_Initialize`, que es por proceso y por
    // módulo. Todos los perfiles de Firefox de una máquina se abren por el
    // mismo `libsoftokn3.so`, así que un contexto cacheado por ruta de módulo
    // devolvería el **primer** perfil abierto cada vez que se pidiera
    // cualquiera de los otros: dos perfiles distintos enseñarían los mismos
    // certificados, duplicados. Se abre uno, se lee y se cierra —el `Drop` de
    // `Pkcs11` llama a `C_Finalize`— antes de pasar al siguiente, y por eso
    // todo lo que abre un almacén tiene que hacerlo dentro de
    // [`with_token_turn`].
    if let Some(init_args) = store.init_args() {
        return Ok(Arc::new(initialized(store.path(), Some(init_args))?));
    }

    let mut loaded = MODULES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        // Un envenenamiento aquí significa que otro hilo entró en pánico con el
        // candado cogido. El mapa sigue siendo válido —solo guarda contextos ya
        // inicializados—, así que perder el proceso por eso sería peor.
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(context) = loaded.get(store.path()) {
        return Ok(Arc::clone(context));
    }

    let context = Arc::new(initialized(store.path(), None)?);
    loaded.insert(store.path().to_path_buf(), Arc::clone(&context));

    Ok(context)
}

/// Carga el módulo y llama a `C_Initialize`, con init args o sin ellos.
///
/// Los init args viajan por `pReserved` de `CK_C_INITIALIZE_ARGS`, que es donde
/// softoken los busca. **No** por la variable de entorno `NSS_LIB_PARAMS`, que
/// es la otra forma de decírselo: esa es del proceso entero y no de la llamada,
/// así que no sabría distinguir un perfil de otro (ID-05 pide varios) y además
/// escribir en el entorno con la aplicación ya arrancada y con hilos dentro no
/// es seguro. El contenido de la cadena es el mismo que la variable llevaría.
fn initialized(module: &Path, init_args: Option<&str>) -> Result<Pkcs11, TokenError> {
    let context = Pkcs11::new(module)?;
    let flags = CInitializeFlags::OS_LOCKING_OK;

    // La cadena tiene que seguir viva mientras dure `C_Initialize`: softoken la
    // lee ahí mismo y no se queda con el puntero.
    let reserved = init_args
        .map(|args| CString::new(args).map_err(|_| nul_inside(args)))
        .transpose()?;
    let arguments = match &reserved {
        // SAFETY: el puntero sale de una `CString` viva hasta el final de esta
        // función, y softoken espera exactamente eso en `pReserved`: una cadena
        // C terminada en cero con sus init args.
        Some(reserved) => unsafe {
            CInitializeArgs::new_with_reserved(
                flags,
                NonNull::new(reserved.as_ptr() as *mut c_void)
                    .expect("una CString nunca esta en la direccion cero"),
            )
        },
        None => CInitializeArgs::new(flags),
    };

    match context.initialize(arguments) {
        Ok(()) => {}
        // `C_Initialize` es por proceso, no por handle: si otra biblioteca del
        // mismo proceso ya cargó este módulo, el nuestro está inicializado y
        // tratarlo como fallo dejaría la aplicación sin token por buenas noticias.
        Err(Error::Pkcs11(RvError::CryptokiAlreadyInitialized, _)) => {}
        Err(other) => return Err(other.into()),
    }

    Ok(context)
}

fn nul_inside(init_args: &str) -> TokenError {
    TokenError::new(
        Situation::ModuleNotFound,
        format!("los init args «{init_args}» llevan un cero dentro"),
    )
}

#[cfg(test)]
mod log_in_before_listing_tests {
    use super::should_attempt_blind_login;

    #[test]
    fn skips_a_slot_that_does_not_require_login() {
        assert!(!should_attempt_blind_login(false, false));
    }

    #[test]
    fn attempts_a_blind_login_on_a_slot_without_a_reader_keypad() {
        assert!(should_attempt_blind_login(true, false));
    }

    #[test]
    fn skips_a_slot_with_a_reader_keypad_even_if_login_is_required() {
        // login(User, None) cede el turno al pinpad y se queda colgada hasta
        // que se teclee el PIN o el lector expire (ID-189): intentarlo aquí
        // bloquearía list_certificates sin nada en pantalla que lo explique.
        assert!(!should_attempt_blind_login(true, true));
    }
}
