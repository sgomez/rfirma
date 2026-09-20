//! Señales, veredictos y acciones del panel de estado.

use serde::{Deserialize, Serialize};

/// Las cuatro señales del panel de estado.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Signal {
    /// Qué versión hay instalada y si hay una más nueva.
    Version,
    /// Qué aplicación abre las sedes.
    SiteSignature,
    /// Certificado propio de rFirma para el canal local.
    LocalCaCertificate,
    /// Certificados personales en los almacenes.
    UserCertificates,
}

/// Los cinco veredictos del panel de estado.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    /// Todo en orden.
    Correct,
    /// Requiere atención pero no impide firmar.
    Attention,
    /// Avería o estado incorrecto.
    Incorrect,
    /// No aplica en este entorno o configuración.
    NotApplicable,
    /// Midiendo o comprobando el estado.
    Checking,
}

/// Los tres tipos de acción que puede ofrecer una señal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionKind {
    /// Reparación del estado averiado.
    Repair,
    /// Elección entre varias opciones.
    Choice,
    /// Enlace externo para actualizar o consultar.
    Link,
}

/// Acción declarada que acompaña al veredicto de una señal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusAction {
    /// Tipo declarado de acción.
    pub kind: ActionKind,
    /// Destino o identificador de la acción.
    pub target: String,
}

/// Fila de estado de una señal para la ventana.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalRow {
    /// Señal evaluada.
    pub signal: Signal,
    /// Valor en texto plano para la celda.
    pub value: String,
    /// Veredicto calculado en Rust.
    pub verdict: Verdict,
    /// Acción disponible si la hay.
    pub action: Option<StatusAction>,
    /// Detalle desplegable, para las señales que lo tienen.
    pub detail: Option<SignalDetail>,
    /// Candidatas entre las que elegir, para la señal `Firma en sedes`.
    pub candidates: Option<Vec<SiteSignatureCandidate>>,
    /// Aviso de reiniciar Firefox, tras instalar con el navegador vivo (ADR-0005).
    pub restart_firefox_notice: bool,
}

/// Candidata a firmar en sedes, para el desplegable de la señal `Firma en sedes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteSignatureCandidate {
    /// Identificador del manejador en el escritorio.
    pub id: String,
    /// Nombre visible, tal y como lo dio el escritorio.
    pub name: String,
    /// Si es la candidata elegida hoy.
    pub selected: bool,
}

/// Sitio donde puede haber un certificado, para el detalle desplegable de una señal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreBrand {
    /// Perfil de usuario de Firefox o derivados.
    Firefox,
    /// Almacén NSS compartido de la familia Chrome/Chromium.
    Chrome,
    /// Base de datos NSS genérica del sistema.
    Nssdb,
    /// Tarjeta o token físico.
    Card,
    /// Fichero PKCS#12 que se instaló en rFirma.
    Installed,
}

/// Un almacén, con su marca y si la señal es de confianza en él.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreDetail {
    /// Marca del almacén.
    pub brand: StoreBrand,
    /// Si la señal es de confianza en este almacén.
    pub trusted: bool,
}

/// Un sitio y cuántos certificados firmables propios tiene.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreCertificates {
    /// Marca del sitio.
    pub brand: StoreBrand,
    /// Cuántos certificados firmables hay en él.
    pub certificates: usize,
}

/// Lo que cuelga de una señal: dónde se confía en la CA, o cuántos certificados hay en cada sitio.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SignalDetail {
    /// Almacén a almacén, si la CA de rFirma está en él.
    Trust {
        /// Los almacenes medidos.
        stores: Vec<StoreDetail>,
    },
    /// Sitio a sitio, cuántos certificados propios tiene.
    Certificates {
        /// Los sitios con al menos un certificado.
        stores: Vec<StoreCertificates>,
    },
}
