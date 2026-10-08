//! El login que comparten las firmas de un ciclo o de un lote en una tarjeta: un solo `C_Login`, y el primer fallo corta las que quedan (ADR-0047).

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use cryptoki::session::Session;

use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::store::StoreClass;

/// Qué hace con el ámbito un `C_Login` rechazado: comprobar un secreto deja reintentar; firmar con él, no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refused {
    /// El secreto se estaba comprobando: la persona puede teclear otro.
    LeavesItOpen,
    /// El secreto ya se estaba usando: ninguna firma más del ámbito llega a la tarjeta.
    CutsIt,
}

enum Login {
    NotYet,
    Held {
        secret: ProtectedSecret,
        session: Session,
    },
    Cut(TokenError),
}

struct Scope {
    card: (PathBuf, String),
    depth: usize,
    login: Login,
}

impl Scope {
    fn replace(&mut self, login: Login) {
        if let Login::Held { session, .. } = std::mem::replace(&mut self.login, login) {
            let _ = session.logout();
        }
    }
}

static SCOPES: Mutex<Vec<Scope>> = Mutex::new(Vec::new());

fn scopes() -> MutexGuard<'static, Vec<Scope>> {
    SCOPES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn card_of(reference: &CertificateRef) -> Option<(PathBuf, String)> {
    let store = reference.store();
    (store.class() == StoreClass::Card).then(|| {
        (
            store.path().to_path_buf(),
            reference.token_label().to_owned(),
        )
    })
}

fn scope_of<'a>(scopes: &'a mut [Scope], card: &(PathBuf, String)) -> Option<&'a mut Scope> {
    scopes.iter_mut().find(|scope| &scope.card == card)
}

/// Abre el ámbito de la tarjeta del certificado, o lo anida en el que ya estuviera abierto.
pub(super) fn hold(reference: &CertificateRef) {
    let Some(card) = card_of(reference) else {
        return;
    };
    let mut scopes = scopes();
    match scope_of(&mut scopes, &card) {
        Some(scope) => scope.depth += 1,
        None => scopes.push(Scope {
            card,
            depth: 1,
            login: Login::NotYet,
        }),
    }
}

/// Cierra un nivel del ámbito; al cerrar el último, sale de la tarjeta.
pub(super) fn release(reference: &CertificateRef) {
    let Some(card) = card_of(reference) else {
        return;
    };
    let mut scopes = scopes();
    let Some(position) = scopes.iter().position(|scope| scope.card == card) else {
        return;
    };
    scopes[position].depth -= 1;
    if scopes[position].depth == 0 {
        scopes.remove(position).replace(Login::NotYet);
    }
}

/// El fallo que ya cortó el ámbito de la tarjeta, si lo hubo: con él no se toca la tarjeta.
pub(super) fn cut_short(reference: &CertificateRef) -> Option<TokenError> {
    let card = card_of(reference)?;
    let mut scopes = scopes();
    match &scope_of(&mut scopes, &card)?.login {
        Login::Cut(error) => Some(error.clone()),
        _ => None,
    }
}

/// `work` sobre la sesión del ámbito, abierta con `log_in` solo si aún no la hay para ese secreto; `None` fuera de un ámbito.
pub(super) fn within<T>(
    reference: &CertificateRef,
    secret: &ProtectedSecret,
    log_in: impl FnOnce() -> Result<Session, TokenError>,
    work: impl FnOnce(&Session) -> Result<T, TokenError>,
    refused: Refused,
) -> Option<Result<T, TokenError>> {
    let card = card_of(reference)?;
    let mut scopes = scopes();
    let scope = scope_of(&mut scopes, &card)?;

    let reused = match std::mem::replace(&mut scope.login, Login::NotYet) {
        Login::Cut(error) => {
            scope.login = Login::Cut(error.clone());
            return Some(Err(error));
        }
        Login::Held {
            secret: held,
            session,
        } if held == *secret => Some(session),
        Login::Held { session, .. } => {
            let _ = session.logout();
            None
        }
        Login::NotYet => None,
    };
    let session = match reused.map_or_else(log_in, Ok) {
        Ok(session) => session,
        Err(error) => return Some(refused_by_the_card(scope, error, refused)),
    };

    let done = work(&session);
    scope.replace(match &done {
        Ok(_) => Login::Held {
            secret: ProtectedSecret::new(secret.as_bytes()),
            session,
        },
        Err(error) => {
            let _ = session.logout();
            Login::Cut(error.clone())
        }
    });
    Some(done)
}

fn refused_by_the_card<T>(
    scope: &mut Scope,
    error: TokenError,
    refused: Refused,
) -> Result<T, TokenError> {
    if refused == Refused::CutsIt {
        scope.login = Login::Cut(error.clone());
    }
    Err(error)
}
