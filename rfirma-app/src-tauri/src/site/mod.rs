//! Contexto `site` (ADR-0017): la raíz de composición.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::PathBuf;
use std::sync::Arc;

use crate::crossing::Failure;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::adapters::isolate::Isolate;
use adapters::tls::LocalCaStore;
use application::errand::ErrandDesk;
pub use application::errand::LiveErrand;
use application::site::CodecTable;
use application::startup::{HeldChannel, LocalCaTrust};
use application::trust::{ProfileTrust, TrustOutcome, WithdrawOutcome};
use domain::tls_error::TlsError;
use domain::trust::Moment;

/// La raíz de `site`: el trámite vivo, el canal sostenido, la confianza de la CA local y la tabla
/// de códecs.
pub struct SiteRoot {
    /// El trámite vivo del proceso.
    pub errand: LiveErrand,
    /// El canal abierto con la sede, sostenido.
    pub held_channel: HeldChannel,
    /// La CA local, sus almacenes y los perfiles NSS.
    pub trust: LocalCaTrust,
    /// Las dos ranuras de la CA local en disco.
    pub ca_store: LocalCaStore,
    /// La tabla de códecs que la negociación elige según la forma de la invocación.
    pub codecs: CodecTable,
    /// Directorio para los documentos de paso.
    pub scratch_dir: PathBuf,
    /// Quien escribe y borra el fichero de paso.
    pub scratch: Arc<dyn ports::Scratch + Send + Sync>,
    /// Los dos servlets del lote remoto.
    pub batch: Arc<dyn ports::BatchServices + Send + Sync>,
    /// El servidor trifásico que la sede nombra en `serverUrl`.
    pub triphase: Arc<dyn ports::TriphaseServer + Send + Sync>,
    /// Diálogos del sistema operativo a través del portal.
    pub portal: Arc<dyn crate::documents::ports::PortalDialogs + Send + Sync>,
}

impl SiteRoot {
    /// Mide, sin escribir, en qué perfiles NSS es de confianza la CA local vigente (ID-346).
    pub fn measure_local_ca_trust(&self) -> Result<Vec<ProfileTrust>, TlsError> {
        application::trust::measure_local_ca_trust(
            self.trust.store.as_ref(),
            &self.trust.profiles,
            self.trust.stores.as_ref(),
        )
    }

    /// Instala o repara la CA local a petición del panel de estado, con el caso de uso del
    /// arranque.
    pub fn install_local_ca_trust(&self) -> Result<TrustOutcome, TlsError> {
        application::trust::refresh_local_ca_trust(
            self.trust.store.as_ref(),
            &self.trust.profiles,
            self.trust.stores.as_ref(),
            Moment::Startup,
        )
    }

    /// Retira la CA local de los almacenes NSS indicados, y sus ranuras si ninguno ha fallado.
    pub fn withdraw_local_ca_trust(
        &self,
        profiles: &[PathBuf],
    ) -> Result<WithdrawOutcome, TlsError> {
        application::trust::withdraw_everywhere(
            self.trust.store.as_ref(),
            profiles,
            self.trust.stores.as_ref(),
        )
    }

    /// Los perfiles NSS de navegadores detectados en esta máquina.
    pub fn nss_profiles(&self) -> &[PathBuf] {
        &self.trust.profiles
    }
}

/// La mesa del trámite sobre las raíces de producción.
pub type SiteDesk<'a> = ErrandDesk<'a, Isolate, Isolate, adapters::desk::Neighbours<'a>>;

/// Cierra lo consentido fuera del ciclo —el lote, remoto o local, o la firma contra el servidor trifásico— con el secreto que entró por la única puerta del PIN.
pub fn the_pending_signature_signed(
    desk: &SiteDesk<'_>,
    live: &LiveErrand,
    secret: &ProtectedSecret,
) -> Result<(), Failure> {
    let secret = secret
        .expose_secret()
        .map_err(|_| Failure::new("unknown", "el secreto tecleado no es texto válido"))?;
    if live.a_server_signature_is_pending() {
        return application::errand::finish_the_server_signature(desk, secret, live)
            .map_err(Failure::from);
    }
    if live.a_batch_is_pending() {
        return application::errand::finish_the_batch(desk, secret, live).map_err(Failure::from);
    }
    application::errand::finish_the_local_batch(desk, secret, live).map_err(Failure::from)
}
