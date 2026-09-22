//! El cliente a prueba resuelto según qué cliente es: el binario y un perfil aislado por almacén,
//! con su envoltorio y la raíz con la que sirve el canal; no lo arranca.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::errand::the_published_client;

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
/// `token`, SoftHSM con PIN.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub(crate) enum Store {
    #[default]
    Rsa,
    Ec,
    Token,
}

impl Store {
    pub(crate) const ALL: [Self; 3] = [Self::Rsa, Self::Ec, Self::Token];

    pub(crate) fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|store| store.name() == name)
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Rsa => "rsa",
            Self::Ec => "ec",
            Self::Token => "token",
        }
    }
}

/// Donde deja la raíz de confianza la instalación de AutoFirma, en el orden en que se busca.
const THE_AUTOFIRMA_ROOTS: &[&str] = &[
    "/usr/lib/Autofirma/Autofirma_ROOT.cer",
    "/etc/ssl/certs/Autofirma_ROOT.pem",
    "/usr/share/ca-certificates/Autofirma/Autofirma_ROOT.crt",
];

/// El único transporte que habla esta fase; el relé por servidor intermedio no está sondeado.
pub(crate) const THE_TRANSPORT: &str = "websocket";

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub(crate) struct Client {
    pub(crate) kind: ClientKind,
    pub(crate) binary: PathBuf,
    pub(crate) profiles: Vec<Profile>,
}

/// El perfil aislado de un almacén: el envoltorio que lanza al cliente contra él y la raíz con la
/// que sirve el canal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub(crate) struct Profile {
    pub(crate) store: Store,
    pub(crate) launcher: PathBuf,
    pub(crate) trust_root: PathBuf,
}

impl Client {
    pub(crate) fn profile(&self, store: Store) -> &Profile {
        self.profiles
            .iter()
            .find(|profile| profile.store == store)
            .expect("resolve monta un perfil por almacén")
    }
}

/// Lo que imprime `scripts/isolated-store.sh`: el envoltorio, la raíz que sirve si depende del
/// cliente y el almacén que ha montado.
#[derive(Debug, PartialEq, Eq)]
struct IsolatedStore {
    launcher: PathBuf,
    served_root: Option<PathBuf>,
    store: Store,
}

/// Las coordenadas que se deducen solas al crear un informe; la versión del cliente, no.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct DeducedCoordinates {
    os: String,
    os_version: String,
    transport: &'static str,
}

/// Resuelve el cliente de clase `kind`, con el binario y la raíz dados a mano si los hay; cada queja
/// dice qué falta.
pub(crate) fn resolve(
    kind: ClientKind,
    binary: Option<PathBuf>,
    trust_root: Option<PathBuf>,
) -> Result<Client, Vec<String>> {
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
    let profiles = Store::ALL
        .into_iter()
        .map(|store| the_profile_of(&binary, kind, store, trust_root.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    preflight()?;
    Ok(Client {
        kind,
        binary,
        profiles,
    })
}

fn the_profile_of(
    binary: &Path,
    kind: ClientKind,
    store: Store,
    trust_root: Option<PathBuf>,
) -> Result<Profile, Vec<String>> {
    let isolated = the_isolated_store_of(binary, kind, store)?;
    if isolated.store != store {
        return Err(vec![format!(
            "isolated-store.sh ha montado el almacén {} en vez de {}",
            isolated.store.name(),
            store.name()
        )]);
    }
    let trust_root = trust_root
        .or_else(|| the_served_root(kind, isolated.served_root, Path::is_file))
        .ok_or_else(|| vec![the_missing_root_complaint(kind, binary)])?;
    Ok(Profile {
        store,
        launcher: isolated.launcher,
        trust_root,
    })
}

pub(crate) fn the_deduced_coordinates() -> DeducedCoordinates {
    DeducedCoordinates {
        os: uname("-s"),
        os_version: uname("-r"),
        transport: THE_TRANSPORT,
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

fn the_binary_on_the_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?
        .to_str()?
        .split(':')
        .map(|directory| Path::new(directory).join(name))
        .find(|candidate| is_executable(candidate))
}

fn the_isolated_store_of(
    binary: &Path,
    kind: ClientKind,
    store: Store,
) -> Result<IsolatedStore, Vec<String>> {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/isolated-store.sh");
    let output = Command::new(&script)
        .arg(binary)
        .arg(kind.name())
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
    let [_kind, launcher, served_root, store] = lines[..] else {
        return Err(format!(
            "isolated-store.sh debía imprimir cuatro líneas y ha impreso {}",
            lines.len()
        ));
    };
    let store = Store::named(store)
        .ok_or_else(|| format!("isolated-store.sh ha montado un almacén desconocido: «{store}»"))?;
    Ok(IsolatedStore {
        launcher: PathBuf::from(launcher),
        served_root: (!served_root.is_empty()).then(|| PathBuf::from(served_root)),
        store,
    })
}

/// La raíz con la que sirve el canal: la CA que rFirma crea en su perfil aislado, o la que la
/// instalación de AutoFirma deja en el sistema.
fn the_served_root(
    kind: ClientKind,
    served_root: Option<PathBuf>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    match kind {
        ClientKind::Rfirma => served_root,
        ClientKind::Autofirma => THE_AUTOFIRMA_ROOTS
            .iter()
            .map(PathBuf::from)
            .find(|candidate| exists(candidate)),
    }
}

fn the_missing_root_complaint(kind: ClientKind, binary: &Path) -> String {
    match kind {
        ClientKind::Rfirma => format!(
            "{} no ha dejado su CA local en el perfil aislado: da la raíz a mano",
            binary.display()
        ),
        ClientKind::Autofirma => format!(
            "no hay raíz de confianza con la que hablarle a {}: no está en {}; da la raíz a mano",
            binary.display(),
            THE_AUTOFIRMA_ROOTS.join(", ")
        ),
    }
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

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_four_lines_of_the_isolated_store() {
        let store =
            the_isolated_store_in("rfirma\n/p/launch-subject\n/p/local-ca.crt.pem\nec\n").unwrap();

        assert_eq!(
            store,
            IsolatedStore {
                launcher: PathBuf::from("/p/launch-subject"),
                served_root: Some(PathBuf::from("/p/local-ca.crt.pem")),
                store: Store::Ec,
            }
        );
    }

    #[test]
    fn an_unknown_mounted_store_is_refused() {
        let complaint =
            the_isolated_store_in("rfirma\n/p/launch-subject\n\nsofthsm2:/m.so\n").unwrap_err();

        assert!(complaint.contains("softhsm2:/m.so"));
    }

    #[test]
    fn every_store_is_found_by_its_name() {
        for store in Store::ALL {
            assert_eq!(Store::named(store.name()), Some(store));
        }
    }

    #[test]
    fn an_empty_served_root_is_no_root() {
        let store = the_isolated_store_in("autofirma\n/p/launch-subject\n\ntoken\n").unwrap();

        assert_eq!(store.served_root, None);
    }

    #[test]
    fn a_short_isolated_store_output_is_refused() {
        let complaint = the_isolated_store_in("autofirma\n").unwrap_err();

        assert!(complaint.contains("cuatro líneas"));
    }

    #[test]
    fn rfirma_serves_with_the_root_born_in_its_profile() {
        let root = the_served_root(
            ClientKind::Rfirma,
            Some(PathBuf::from("/p/local-ca.crt.pem")),
            |_| true,
        );

        assert_eq!(root, Some(PathBuf::from("/p/local-ca.crt.pem")));
    }

    #[test]
    fn autofirma_serves_with_the_first_root_its_installation_left() {
        let root = the_served_root(ClientKind::Autofirma, None, |candidate| {
            candidate.ends_with("Autofirma_ROOT.pem")
        });

        assert_eq!(
            root,
            Some(PathBuf::from("/etc/ssl/certs/Autofirma_ROOT.pem"))
        );
    }

    #[test]
    fn autofirma_without_an_installed_root_names_where_it_looked() {
        assert_eq!(
            the_served_root(ClientKind::Autofirma, None, |_| false),
            None
        );
        let complaint =
            the_missing_root_complaint(ClientKind::Autofirma, Path::new("/bin/autofirma"));

        assert!(complaint.contains("/usr/lib/Autofirma/Autofirma_ROOT.cer"));
    }
}
