//! El sujeto de la tanda resuelto a partir de su perfil: el binario, el envoltorio que lo aísla, la
//! raíz con la que sirve el canal y el almacén; no lo arranca.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::baseline::Profile;
use crate::errand::the_published_client;

/// Donde deja la raíz de confianza la instalación de AutoFirma, en el orden en que se busca.
const THE_AUTOFIRMA_ROOTS: &[&str] = &[
    "/usr/lib/Autofirma/Autofirma_ROOT.cer",
    "/etc/ssl/certs/Autofirma_ROOT.pem",
    "/usr/share/ca-certificates/Autofirma/Autofirma_ROOT.crt",
];

/// El único transporte que habla esta fase; el relé por servidor intermedio no está sondeado.
pub(crate) const THE_TRANSPORT: &str = "websocket";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Subject {
    pub(crate) profile: Profile,
    pub(crate) binary: PathBuf,
    pub(crate) launcher: PathBuf,
    pub(crate) trust_root: PathBuf,
    pub(crate) store: String,
}

/// Lo que imprime `scripts/isolated-store.sh`: el envoltorio, la raíz que sirve si depende del
/// perfil y el módulo PKCS#11 del almacén.
#[derive(Debug, PartialEq, Eq)]
struct IsolatedStore {
    launcher: PathBuf,
    served_root: Option<PathBuf>,
    module: String,
}

/// Las coordenadas que se deducen solas al crear un informe; la versión del sujeto, no.
#[derive(Debug, Serialize)]
pub(crate) struct DeducedCoordinates {
    os: String,
    os_version: String,
    transport: &'static str,
    store: String,
}

/// Resuelve el sujeto de `profile`, con el binario y la raíz dados a mano si los hay; cada queja
/// dice qué falta.
pub(crate) fn resolve(
    profile: Profile,
    binary: Option<PathBuf>,
    trust_root: Option<PathBuf>,
) -> Result<Subject, Vec<String>> {
    let binary = match binary {
        Some(binary) => binary,
        None => the_binary_on_the_path(profile.name()).ok_or_else(|| {
            vec![format!(
                "no encuentro «{}» en el PATH: da la ruta del binario a mano",
                profile.name()
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
    let isolated = the_isolated_store_of(&binary, profile)?;
    let trust_root = trust_root
        .or_else(|| the_served_root(profile, isolated.served_root.clone(), Path::is_file))
        .ok_or_else(|| vec![the_missing_root_complaint(profile, &binary)])?;
    preflight()?;
    Ok(Subject {
        profile,
        binary,
        launcher: isolated.launcher,
        trust_root,
        store: format!("softhsm2:{}", isolated.module),
    })
}

pub(crate) fn the_deduced_coordinates(subject: &Subject) -> DeducedCoordinates {
    DeducedCoordinates {
        os: uname("-s"),
        os_version: uname("-r"),
        transport: THE_TRANSPORT,
        store: subject.store.clone(),
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

fn the_isolated_store_of(binary: &Path, profile: Profile) -> Result<IsolatedStore, Vec<String>> {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/isolated-store.sh");
    let output = Command::new(&script)
        .arg(binary)
        .arg(profile.name())
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
    let [_kind, launcher, served_root, module] = lines[..] else {
        return Err(format!(
            "isolated-store.sh debía imprimir cuatro líneas y ha impreso {}",
            lines.len()
        ));
    };
    Ok(IsolatedStore {
        launcher: PathBuf::from(launcher),
        served_root: (!served_root.is_empty()).then(|| PathBuf::from(served_root)),
        module: module.to_owned(),
    })
}

/// La raíz con la que sirve el canal: la CA que rFirma crea en su perfil aislado, o la que la
/// instalación de AutoFirma deja en el sistema.
fn the_served_root(
    profile: Profile,
    served_root: Option<PathBuf>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    match profile {
        Profile::Rfirma => served_root,
        Profile::Autofirma => THE_AUTOFIRMA_ROOTS
            .iter()
            .map(PathBuf::from)
            .find(|candidate| exists(candidate)),
    }
}

fn the_missing_root_complaint(profile: Profile, binary: &Path) -> String {
    match profile {
        Profile::Rfirma => format!(
            "{} no ha dejado su CA local en el perfil aislado: da la raíz a mano",
            binary.display()
        ),
        Profile::Autofirma => format!(
            "no hay raíz de confianza con la que hablarle a {}: no está en {}; da la raíz a mano",
            binary.display(),
            THE_AUTOFIRMA_ROOTS.join(", ")
        ),
    }
}

/// Lo que hace falta además del sujeto para sondearlo: Node y el cliente publicado.
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
        let store = the_isolated_store_in(
            "rfirma\n/p/launch-subject\n/p/local-ca.crt.pem\n/usr/lib/softhsm/libsofthsm2.so\n",
        )
        .unwrap();

        assert_eq!(
            store,
            IsolatedStore {
                launcher: PathBuf::from("/p/launch-subject"),
                served_root: Some(PathBuf::from("/p/local-ca.crt.pem")),
                module: "/usr/lib/softhsm/libsofthsm2.so".to_owned(),
            }
        );
    }

    #[test]
    fn an_empty_served_root_is_no_root() {
        let store = the_isolated_store_in("autofirma\n/p/launch-subject\n\n/m.so\n").unwrap();

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
            Profile::Rfirma,
            Some(PathBuf::from("/p/local-ca.crt.pem")),
            |_| true,
        );

        assert_eq!(root, Some(PathBuf::from("/p/local-ca.crt.pem")));
    }

    #[test]
    fn autofirma_serves_with_the_first_root_its_installation_left() {
        let root = the_served_root(Profile::Autofirma, None, |candidate| {
            candidate.ends_with("Autofirma_ROOT.pem")
        });

        assert_eq!(
            root,
            Some(PathBuf::from("/etc/ssl/certs/Autofirma_ROOT.pem"))
        );
    }

    #[test]
    fn autofirma_without_an_installed_root_names_where_it_looked() {
        assert_eq!(the_served_root(Profile::Autofirma, None, |_| false), None);
        let complaint = the_missing_root_complaint(Profile::Autofirma, Path::new("/bin/autofirma"));

        assert!(complaint.contains("/usr/lib/Autofirma/Autofirma_ROOT.cer"));
    }
}
