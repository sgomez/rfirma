//! Las líneas de los almacenes del informe: los perfiles NSS, y el almacén de Windows con sus minidrivers y el de macOS con sus controladores.

use super::{section, DnieInTheReader, NssProfileState, NssStores, ReportOwner};

/// Lo que solo existe en Windows: el almacén del sistema y los minidrivers de tarjeta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsStores {
    /// Si el almacén raíz del usuario confía en el canal local; nunca es un perfil ignorado.
    pub local_channel: NssProfileState,
    /// Tipos de tarjeta que Windows tiene dados de alta.
    pub minidrivers: Vec<String>,
}

/// Lo que solo existe en macOS: la confianza del usuario y los controladores de tarjeta del sistema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacosStores {
    /// Si la confianza del usuario en el Almacén de macOS acepta el canal local.
    pub local_channel: NssProfileState,
    /// Controladores de tarjeta que macOS tiene instalados.
    pub card_drivers: Vec<String>,
}

/// Lo que solo existe donde rdnie lee el DNIe: la tarjeta que se ve y la versión compilada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DnieReading {
    /// Lo que rFirma ve en el lector.
    pub card: DnieInTheReader,
    /// Versión de rdnie con la que se compiló.
    pub rdnie_version: String,
}

pub(super) fn windows_sections(stores: Option<&WindowsStores>) -> [Option<String>; 2] {
    let Some(stores) = stores else {
        return [None, None];
    };
    let trust = format!("Raíz del usuario: {}", profile_state(&stores.local_channel));
    let minidrivers = if stores.minidrivers.is_empty() {
        vec!["tarjetas: ninguno registrado".to_owned()]
    } else {
        stores
            .minidrivers
            .iter()
            .map(|name| format!("{name}: registrado"))
            .collect()
    };
    [
        section("Almacén de Windows", &[trust]),
        section("Minidrivers", &minidrivers),
    ]
}

pub(super) fn macos_sections(stores: Option<&MacosStores>) -> [Option<String>; 2] {
    let Some(stores) = stores else {
        return [None, None];
    };
    let trust = format!(
        "Confianza del usuario: {}",
        profile_state(&stores.local_channel)
    );
    let drivers = if stores.card_drivers.is_empty() {
        vec!["tarjetas: ninguno instalado".to_owned()]
    } else {
        stores
            .card_drivers
            .iter()
            .map(|name| format!("{name}: instalado"))
            .collect()
    };
    [
        section("Almacén de macOS", &[trust]),
        section("Controladores de tarjeta", &drivers),
    ]
}

pub(super) fn dnie_section(reading: Option<&DnieReading>) -> Option<String> {
    section("DNIe", &dnie_lines(reading?))
}

fn dnie_lines(reading: &DnieReading) -> Vec<String> {
    let card = match &reading.card {
        DnieInTheReader::Seen(dnie) => vec![
            format!("ATR: {}", dnie.atr),
            format!("versión: {}", dnie.version),
            format!("juego de claves: {}", dnie.key_set),
        ],
        DnieInTheReader::Absent => vec!["tarjeta: no hay DNIe".to_owned()],
        DnieInTheReader::Unopened(error) => vec![format!("tarjeta: rdnie no la abre · {error}")],
    };
    card.into_iter()
        .chain(std::iter::once(format!("rdnie: {}", reading.rdnie_version)))
        .collect()
}

pub(super) fn nss_lines(stores: &NssStores, owner: &ReportOwner) -> Vec<String> {
    let rfirma_store = if stores.rfirma_store_installed {
        "instalado"
    } else {
        "no instalado"
    };
    stores
        .profiles
        .iter()
        .flat_map(|profile| {
            [
                format!(
                    "{}: {}",
                    profile.owner.label(),
                    profile_state(&profile.state)
                ),
                format!("  ruta: {}", owner.anonymized(&profile.directory)),
            ]
        })
        .chain(std::iter::once(format!(
            "Almacén de rFirma: {rfirma_store}"
        )))
        .collect()
}

fn profile_state(state: &NssProfileState) -> String {
    match state {
        NssProfileState::TrustsLocalChannel { until: Some(date) } => {
            format!("confía en el canal local · hasta {date}")
        }
        NssProfileState::TrustsLocalChannel { until: None } => {
            "confía en el canal local".to_owned()
        }
        NssProfileState::DoesNotTrustLocalChannel => "no confía en el canal local".to_owned(),
        NssProfileState::IgnoredWithoutCertificateDatabase => "ignorado · sin cert9.db".to_owned(),
    }
}
