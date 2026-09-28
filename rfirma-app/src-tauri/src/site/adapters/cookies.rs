//! El almacén de cookies de la operación de sede en curso, que comparten los cuatro clientes HTTP (ADR-0038).

use std::sync::{Arc, OnceLock, RwLock};

use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::HeaderValue;

/// Un almacén `ACCEPT_ALL` en memoria que se vacía entero al contestar la operación.
#[derive(Default)]
pub struct OperationCookies {
    jar: RwLock<Arc<Jar>>,
}

impl OperationCookies {
    /// El almacén del proceso, el que usan los clientes de producción.
    pub fn of_the_process() -> Arc<Self> {
        static STORE: OnceLock<Arc<OperationCookies>> = OnceLock::new();
        Arc::clone(STORE.get_or_init(Arc::default))
    }

    /// Suelta cuanto guardó la operación que acaba de contestarse.
    pub fn forget(&self) {
        *self
            .jar
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::default();
    }

    fn current(&self) -> Arc<Jar> {
        Arc::clone(
            &self
                .jar
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }
}

impl CookieStore for OperationCookies {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &url::Url) {
        self.current().set_cookies(cookie_headers, url);
    }

    fn cookies(&self, url: &url::Url) -> Option<HeaderValue> {
        self.current().cookies(url)
    }
}

#[cfg(test)]
mod tests;
