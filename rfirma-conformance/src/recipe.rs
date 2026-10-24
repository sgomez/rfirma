//! La receta de cada clase de cliente: sus binarios candidatos, lo que dice de sí mismo y cómo se completa el perfil aislado de un almacén para lanzarlo.

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::candidates::{the_identity_of_rfirma, the_rfirma_candidates};
use crate::client::{
    the_binary_on_the_path, Candidate, ClientKind, Identity, IsolatedStore, Origin, Profile, Store,
};

/// Lo que la suite sabe de una clase de cliente para lanzarla contra un almacén montado.
pub(crate) trait Recipe: Sync {
    /// Escribe en el perfil lo que el cliente lee al arrancar y dice con qué entorno se lanza y con
    /// qué raíz sirve el canal; `given_root` manda sobre la suya.
    fn the_profile_of(
        &self,
        isolated: IsolatedStore,
        given_root: Option<PathBuf>,
    ) -> Result<Profile, String>;

    /// Los binarios de esta clase que se pueden medir, cada uno con su origen.
    fn the_candidates(&self) -> Vec<Candidate>;

    /// Lo que dice de sí mismo el cliente de `binary`, o por qué no se puede medir.
    fn the_identity_of(
        &self,
        binary: &Path,
        library_dir: Option<&Path>,
    ) -> Result<Identity, String>;
}

pub(crate) fn the_recipe_of(kind: ClientKind) -> &'static dyn Recipe {
    match kind {
        ClientKind::Rfirma => &RfirmaRecipe,
        ClientKind::Autofirma => &AutofirmaRecipe,
    }
}

struct RfirmaRecipe;

impl Recipe for RfirmaRecipe {
    fn the_profile_of(
        &self,
        isolated: IsolatedStore,
        given_root: Option<PathBuf>,
    ) -> Result<Profile, String> {
        let home = isolated.home;
        write_the_preferences_of_rfirma(&home, isolated.store)?;
        let local_ca = the_local_ca_of_rfirma(&home)?;
        let environment = vec![
            ("HOME", OsString::from(&home)),
            ("XDG_CONFIG_HOME", home.join(".config").into()),
            ("XDG_STATE_HOME", home.join(".local/state").into()),
            ("XDG_DATA_HOME", home.join(".local/share").into()),
            ("XDG_CACHE_HOME", home.join(".cache").into()),
            ("SOFTHSM2_CONF", isolated.softhsm_conf.into()),
        ];
        Ok(Profile {
            store: isolated.store,
            home,
            environment,
            trust_root: given_root.unwrap_or(local_ca),
        })
    }

    fn the_candidates(&self) -> Vec<Candidate> {
        the_rfirma_candidates()
    }

    fn the_identity_of(
        &self,
        binary: &Path,
        library_dir: Option<&Path>,
    ) -> Result<Identity, String> {
        the_identity_of_rfirma(binary, library_dir)
    }
}

/// Las preferencias de rFirma como si la persona las hubiera elegido: castellano, tema claro, sin
/// cuenta atrás de consentir ni asistente del primer arranque, y SHA-1 permitido salvo en
/// `rsa_without_sha1`.
fn write_the_preferences_of_rfirma(home: &Path, store: Store) -> Result<(), String> {
    let dir = home.join(".config/rfirma");
    let preferences = serde_json::json!({
        "version": 1,
        "consent_countdown": false,
        "honour_automatic_selection": true,
        "setup_wizard_seen": true,
        "allow_sha1": store != Store::RsaWithoutSha1,
        "language": "es",
        "theme": "light",
    });
    std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(dir.join("config.json"), format!("{preferences}\n")))
        .map_err(|error| format!("las preferencias de rFirma en {}: {error}", dir.display()))
}

/// La CA local con la forma de la que genera rFirma (ADR-0005): un arranque en seco ya no la crea,
/// y lanzarlo abriría su ventana principal.
fn the_local_ca_of_rfirma(home: &Path) -> Result<PathBuf, String> {
    let dir = home.join(".local/share/rfirma");
    let (key, certificate) = (dir.join("local-ca.key.pem"), dir.join("local-ca.crt.pem"));
    let unwritable = |error: std::io::Error| format!("la CA local en {}: {error}", dir.display());
    std::fs::create_dir_all(&dir).map_err(unwritable)?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).map_err(unwritable)?;
    let created = Command::new("openssl")
        .args(["req", "-x509", "-new", "-newkey", "ec"])
        .args(["-pkeyopt", "ec_paramgen_curve:prime256v1", "-nodes"])
        .arg("-keyout")
        .arg(&key)
        .arg("-out")
        .arg(&certificate)
        .args(["-days", "900", "-sha256", "-subj", "/CN=rFirma CA local"])
        .args(["-addext", "basicConstraints=critical,CA:TRUE,pathlen:0"])
        .args(["-addext", "keyUsage=critical,keyCertSign,cRLSign"])
        .args([
            "-addext",
            "nameConstraints=critical,permitted;DNS:localhost,permitted;IP:127.0.0.1/255.255.255.255,permitted;IP:::1/ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
        ])
        .args(["-addext", "subjectKeyIdentifier=hash"])
        .output()
        .map_err(|error| format!("falta: openssl ({error})"))?;
    if !created.status.success() {
        return Err(format!(
            "openssl no creó la CA local de rFirma: {}",
            String::from_utf8_lossy(&created.stderr).trim()
        ));
    }
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).map_err(unwritable)?;
    Ok(certificate)
}

struct AutofirmaRecipe;

/// Donde deja la raíz de confianza la instalación de AutoFirma, en el orden en que se busca.
const THE_AUTOFIRMA_ROOTS: &[&str] = &[
    "/usr/lib/Autofirma/Autofirma_ROOT.cer",
    "/etc/ssl/certs/Autofirma_ROOT.pem",
    "/usr/share/ca-certificates/Autofirma/Autofirma_ROOT.crt",
];

impl Recipe for AutofirmaRecipe {
    fn the_profile_of(
        &self,
        isolated: IsolatedStore,
        given_root: Option<PathBuf>,
    ) -> Result<Profile, String> {
        let trust_root = given_root
            .or_else(|| the_installed_autofirma_root(Path::is_file))
            .ok_or_else(the_missing_autofirma_root_complaint)?;
        let home = isolated.home;
        let mut user_home = OsString::from("-Duser.home=");
        user_home.push(&home);
        let environment = vec![
            ("HOME", OsString::from(&home)),
            ("JDK_JAVA_OPTIONS", user_home),
            ("SOFTHSM2_CONF", isolated.softhsm_conf.into()),
        ];
        Ok(Profile {
            store: isolated.store,
            home,
            environment,
            trust_root,
        })
    }

    fn the_candidates(&self) -> Vec<Candidate> {
        the_binary_on_the_path(ClientKind::Autofirma.name())
            .map(|binary| Candidate {
                kind: ClientKind::Autofirma,
                origin: Origin::Installed,
                binary,
                library_dir: None,
                identity: Identity::default(),
            })
            .into_iter()
            .collect()
    }

    fn the_identity_of(&self, _: &Path, _: Option<&Path>) -> Result<Identity, String> {
        Ok(Identity::default())
    }
}

fn the_installed_autofirma_root(exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    THE_AUTOFIRMA_ROOTS
        .iter()
        .map(PathBuf::from)
        .find(|candidate| exists(candidate))
}

fn the_missing_autofirma_root_complaint() -> String {
    format!(
        "no hay raíz de confianza con la que hablarle a AutoFirma: no está en {}; da la raíz a mano",
        THE_AUTOFIRMA_ROOTS.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autofirma_serves_with_the_first_root_its_installation_left() {
        let root =
            the_installed_autofirma_root(|candidate| candidate.ends_with("Autofirma_ROOT.pem"));

        assert_eq!(
            root,
            Some(PathBuf::from("/etc/ssl/certs/Autofirma_ROOT.pem"))
        );
    }

    #[test]
    fn autofirma_without_an_installed_root_names_where_it_looked() {
        assert_eq!(the_installed_autofirma_root(|_| false), None);

        assert!(the_missing_autofirma_root_complaint()
            .contains("/usr/lib/Autofirma/Autofirma_ROOT.cer"));
    }
}
