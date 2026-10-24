//! El cliente a prueba resuelto según qué cliente es, y el perfil aislado de cada almacén, que se
//! prepara al pedirlo; no lo arranca.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::errand::the_published_client;
use crate::recipe::the_recipe_of;

/// Qué cliente es, que solo decide cómo se lanza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub(crate) enum ClientKind {
    Autofirma,
    Rfirma,
}

impl ClientKind {
    pub(crate) fn named(name: &str) -> Option<Self> {
        match name {
            "autofirma" => Some(Self::Autofirma),
            "rfirma" => Some(Self::Rfirma),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Autofirma => "autofirma",
            Self::Rfirma => "rfirma",
        }
    }
}

/// El almacén de un perfil aislado: `rsa` y `ec`, una NSS sin contraseña con un solo certificado;
/// `token`, SoftHSM con PIN; `token_apart`, el token sin registrar junto a una NSS con otro; `ed25519`,
/// un token sin registrar con una sola clave Ed25519; `several` y `expired`, NSS con varios, para los
/// filtros; `dnie`, el DNIe que la persona tiene en el lector, por la vía de la plataforma;
/// `rsa_without_sha1`, el de `rsa` con rFirma sin SHA-1 permitido en sus preferencias.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub(crate) enum Store {
    #[default]
    Rsa,
    Ec,
    Token,
    TokenApart,
    Ed25519,
    Several,
    Expired,
    Dnie,
    RsaWithoutSha1,
}

impl Store {
    pub(crate) const ALL: [Self; 9] = [
        Self::Rsa,
        Self::Ec,
        Self::Token,
        Self::TokenApart,
        Self::Ed25519,
        Self::Several,
        Self::Expired,
        Self::Dnie,
        Self::RsaWithoutSha1,
    ];

    pub(crate) fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|store| store.name() == name)
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Rsa => "rsa",
            Self::Ec => "ec",
            Self::Token => "token",
            Self::TokenApart => "token_apart",
            Self::Ed25519 => "ed25519",
            Self::Several => "several",
            Self::Expired => "expired",
            Self::Dnie => "dnie",
            Self::RsaWithoutSha1 => "rsa_without_sha1",
        }
    }
}

/// El cliente elegido: su clase, de dónde sale su binario, su biblioteca nativa, lo que dice de sí
/// mismo y la raíz dada a mano, si la hay; sus perfiles se preparan al pedirlos.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub(crate) struct Client {
    pub(crate) kind: ClientKind,
    pub(crate) origin: Origin,
    pub(crate) binary: PathBuf,
    pub(crate) library_dir: Option<PathBuf>,
    pub(crate) identity: Identity,
    pub(crate) trust_root: Option<PathBuf>,
}

/// De dónde sale el binario de un cliente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub(crate) enum Origin {
    Installed,
    LocalBuild,
    Worktree,
    Given,
}

/// Un binario que la suite ofrece para medir, con la biblioteca nativa con que se lanza.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub(crate) struct Candidate {
    pub(crate) kind: ClientKind,
    pub(crate) origin: Origin,
    pub(crate) binary: PathBuf,
    pub(crate) library_dir: Option<PathBuf>,
    pub(crate) identity: Identity,
}

/// Lo que un cliente dice de sí mismo: su versión y su canal; de AutoFirma, nada.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub(crate) struct Identity {
    pub(crate) version: Option<String>,
    pub(crate) channel: Option<String>,
}

/// El perfil aislado de un almacén: el HOME del cliente, el entorno con el que se lanza contra él y
/// la raíz con la que sirve el canal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Profile {
    pub(crate) store: Store,
    pub(crate) home: PathBuf,
    pub(crate) environment: Vec<(&'static str, OsString)>,
    pub(crate) trust_root: PathBuf,
}

/// Lo que imprime `scripts/isolated-store.sh`: el perfil, la configuración de SoftHSM que monta y
/// el almacén montado.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IsolatedStore {
    pub(crate) home: PathBuf,
    pub(crate) softhsm_conf: PathBuf,
    pub(crate) store: Store,
}

/// Las coordenadas que se deducen solas al crear un informe: el sistema y, si el cliente la dice, su
/// versión.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct DeducedCoordinates {
    os: String,
    os_version: String,
    client_version: String,
}

/// Resuelve el cliente de clase `kind`, con el binario y la raíz dados a mano si los hay; un binario
/// que está entre los `candidates` se lanza como se ofreció. Cada queja dice qué falta.
pub(crate) fn resolve(
    kind: ClientKind,
    binary: Option<PathBuf>,
    trust_root: Option<PathBuf>,
    candidates: &[Candidate],
) -> Result<Client, Vec<String>> {
    let origin = if binary.is_some() {
        Origin::Given
    } else {
        Origin::Installed
    };
    let binary = match binary {
        Some(binary) => binary,
        None => the_binary_on_the_path(kind.name()).ok_or_else(|| {
            vec![format!(
                "no encuentro «{}» en el PATH: da la ruta del binario a mano",
                kind.name()
            )]
        })?,
    };
    if !is_executable(&binary) {
        return Err(vec![format!(
            "el cliente {} no existe o no se puede ejecutar",
            binary.display()
        )]);
    }
    if let Some(root) = &trust_root {
        if !root.is_file() {
            return Err(vec![format!(
                "la raíz de confianza {} no existe",
                root.display()
            )]);
        }
    }
    let offered = candidates
        .iter()
        .find(|candidate| candidate.kind == kind && candidate.binary == binary);
    let candidate = match offered {
        Some(candidate) => candidate.clone(),
        None => Candidate {
            identity: the_recipe_of(kind)
                .the_identity_of(&binary, None)
                .map_err(|complaint| vec![complaint])?,
            kind,
            origin,
            binary,
            library_dir: None,
        },
    };
    preflight()?;
    Ok(Client {
        kind,
        origin: candidate.origin,
        binary: candidate.binary,
        library_dir: candidate.library_dir,
        identity: candidate.identity,
        trust_root,
    })
}

/// El binario con que se restaura el cliente de un informe: el suyo si aún se ofrece; si no, el
/// candidato de su clase, su origen y su versión; si no, el suyo si aún se puede ejecutar.
pub(crate) fn the_binary_to_restore(
    kind: ClientKind,
    origin: Option<Origin>,
    binary: &Path,
    version: &str,
    candidates: &[Candidate],
) -> Option<PathBuf> {
    let of_its_kind = || {
        candidates
            .iter()
            .filter(move |offered| offered.kind == kind)
    };
    of_its_kind()
        .find(|offered| offered.binary == binary)
        .or_else(|| {
            of_its_kind().find(|offered| {
                Some(offered.origin) == origin
                    && offered
                        .identity
                        .version
                        .as_deref()
                        .is_none_or(|said| said == version)
            })
        })
        .map(|offered| offered.binary.clone())
        .or_else(|| is_executable(binary).then(|| binary.to_owned()))
}

/// Monta el almacén `store` en un perfil nuevo y lo completa con la receta del cliente.
pub(crate) fn prepare(client: &Client, store: Store) -> Result<Profile, Vec<String>> {
    let isolated = the_isolated_store_of(store)?;
    if isolated.store != store {
        return Err(vec![format!(
            "isolated-store.sh ha montado el almacén {} en vez de {}",
            isolated.store.name(),
            store.name()
        )]);
    }
    the_recipe_of(client.kind)
        .the_profile_of(isolated, client.trust_root.clone())
        .map(|profile| with_the_library_of(client, profile))
        .map_err(|complaint| vec![complaint])
}

fn with_the_library_of(client: &Client, mut profile: Profile) -> Profile {
    if let Some(dir) = &client.library_dir {
        profile
            .environment
            .push(("RFIRMA_LIB_DIR", dir.as_os_str().to_owned()));
    }
    profile
}

pub(crate) fn the_deduced_coordinates(client: &Client) -> DeducedCoordinates {
    DeducedCoordinates {
        os: uname("-s"),
        os_version: uname("-r"),
        client_version: client.identity.version.clone().unwrap_or_default(),
    }
}

fn uname(flag: &str) -> String {
    Command::new("uname")
        .arg(flag)
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}

pub(crate) fn the_binary_on_the_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?
        .to_str()?
        .split(':')
        .map(|directory| Path::new(directory).join(name))
        .find(|candidate| is_executable(candidate))
}

fn the_isolated_store_of(store: Store) -> Result<IsolatedStore, Vec<String>> {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/isolated-store.sh");
    let output = Command::new(&script)
        .arg(store.name())
        .output()
        .map_err(|error| vec![format!("{} no arrancó: {error}", script.display())])?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .lines()
            .map(str::to_owned)
            .collect());
    }
    the_isolated_store_in(&String::from_utf8_lossy(&output.stdout))
        .map_err(|complaint| vec![complaint])
}

fn the_isolated_store_in(printed: &str) -> Result<IsolatedStore, String> {
    let lines: Vec<&str> = printed.lines().collect();
    let [home, softhsm_conf, store] = lines[..] else {
        return Err(format!(
            "isolated-store.sh debía imprimir tres líneas y ha impreso {}",
            lines.len()
        ));
    };
    let store = Store::named(store)
        .ok_or_else(|| format!("isolated-store.sh ha montado un almacén desconocido: «{store}»"))?;
    Ok(IsolatedStore {
        home: PathBuf::from(home),
        softhsm_conf: PathBuf::from(softhsm_conf),
        store,
    })
}

/// Lo que hace falta además del cliente para sondearlo: Node y el cliente publicado.
fn preflight() -> Result<(), Vec<String>> {
    let mut complaints = Vec::new();
    if Command::new("node").arg("--version").output().is_err() {
        complaints.push("falta Node en el PATH".to_owned());
    }
    let published_client = the_published_client();
    if !published_client.exists() {
        complaints.push(format!(
            "falta {}: ejecuta `just autoscript`",
            published_client.display()
        ));
    }
    if complaints.is_empty() {
        Ok(())
    } else {
        Err(complaints)
    }
}

pub(crate) fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Un perfil de pruebas del almacén `rsa` en `home`, sin entorno y con una raíz que no existe.
#[cfg(test)]
pub(crate) fn a_profile_in(home: &Path) -> Profile {
    Profile {
        store: Store::Rsa,
        home: home.to_owned(),
        environment: Vec::new(),
        trust_root: home.join("root.pem"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_three_lines_of_the_isolated_store() {
        let store = the_isolated_store_in("/p\n/p/softhsm/softhsm2.conf\nec\n").unwrap();

        assert_eq!(
            store,
            IsolatedStore {
                home: PathBuf::from("/p"),
                softhsm_conf: PathBuf::from("/p/softhsm/softhsm2.conf"),
                store: Store::Ec,
            }
        );
    }

    #[test]
    fn an_unknown_mounted_store_is_refused() {
        let complaint =
            the_isolated_store_in("/p\n/p/softhsm/softhsm2.conf\nsofthsm2:/m.so\n").unwrap_err();

        assert!(complaint.contains("softhsm2:/m.so"));
    }

    #[test]
    fn the_dnie_store_is_found_by_its_name() {
        assert_eq!(Store::named("dnie"), Some(Store::Dnie));
        assert_eq!(Store::Dnie.name(), "dnie");
    }

    #[test]
    fn every_store_is_found_by_its_name() {
        for store in Store::ALL {
            assert_eq!(Store::named(store.name()), Some(store));
        }
    }

    #[test]
    fn a_build_is_launched_with_the_native_library_of_its_checkout() {
        let client = Client {
            kind: ClientKind::Rfirma,
            origin: Origin::LocalBuild,
            binary: PathBuf::from("/src/rfirma/rfirma-app/src-tauri/target/release/rfirma"),
            library_dir: Some(PathBuf::from(
                "/src/rfirma/rfirma-native-bridge/target/lib/rfirma",
            )),
            identity: Identity::default(),
            trust_root: None,
        };

        let profile = with_the_library_of(&client, a_profile_in(Path::new("/p")));

        assert!(profile.environment.contains(&(
            "RFIRMA_LIB_DIR",
            OsString::from("/src/rfirma/rfirma-native-bridge/target/lib/rfirma")
        )));
    }

    fn a_candidate(origin: Origin, binary: &str, version: Option<&str>) -> Candidate {
        Candidate {
            kind: ClientKind::Rfirma,
            origin,
            binary: PathBuf::from(binary),
            library_dir: None,
            identity: Identity {
                version: version.map(str::to_owned),
                channel: None,
            },
        }
    }

    #[test]
    fn a_report_client_still_offered_is_restored_with_its_own_binary() {
        let candidates = [
            a_candidate(Origin::LocalBuild, "/a/rfirma", Some("1.0")),
            a_candidate(Origin::LocalBuild, "/b/rfirma", Some("1.0")),
        ];

        let restored = the_binary_to_restore(
            ClientKind::Rfirma,
            Some(Origin::LocalBuild),
            Path::new("/b/rfirma"),
            "1.0",
            &candidates,
        );

        assert_eq!(restored, Some(PathBuf::from("/b/rfirma")));
    }

    #[test]
    fn a_report_client_whose_binary_moved_is_restored_from_its_origin_and_version() {
        let candidates = [
            a_candidate(Origin::Installed, "/usr/bin/rfirma", Some("1.0")),
            a_candidate(Origin::Worktree, "/w/old/rfirma", Some("0.9")),
            a_candidate(Origin::Worktree, "/w/new/rfirma", Some("1.0")),
        ];

        let restored = the_binary_to_restore(
            ClientKind::Rfirma,
            Some(Origin::Worktree),
            Path::new("/w/gone/rfirma"),
            "1.0",
            &candidates,
        );

        assert_eq!(restored, Some(PathBuf::from("/w/new/rfirma")));
    }

    #[test]
    fn a_report_client_that_is_nowhere_is_not_restored() {
        let candidates = [a_candidate(
            Origin::Installed,
            "/usr/bin/rfirma",
            Some("2.0"),
        )];

        let restored = the_binary_to_restore(
            ClientKind::Rfirma,
            Some(Origin::Installed),
            Path::new("/nowhere/rfirma"),
            "1.0",
            &candidates,
        );

        assert_eq!(restored, None);
    }

    #[test]
    fn a_short_isolated_store_output_is_refused() {
        let complaint = the_isolated_store_in("/p\n").unwrap_err();

        assert!(complaint.contains("tres líneas"));
    }
}
