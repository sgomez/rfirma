//! Dónde busca la suite los binarios de rFirma —el instalado, el build local y el de un worktree— y cómo lee su identidad de `--version`.

use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::fs::File;
use std::io::Read;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::client::{
    is_executable, the_binary_on_the_path, Candidate, ClientKind, Identity, Origin,
};

const THE_CHUNK: usize = 1 << 20;
const THE_LONGEST_ASSET_NAME: usize = 256;
const THE_PATIENCE_OF_VERSION: Duration = Duration::from_secs(10);

/// Los rFirma que se pueden medir: el del `PATH`, el build local y el del árbol compartido de los
/// worktrees, en `release` y `debug`, los dos últimos con la biblioteca nativa de su checkout.
pub(crate) fn the_rfirma_candidates() -> Vec<Candidate> {
    let primary = the_primary_checkout_of(&the_suite_checkout());
    let mut places: Vec<(Origin, PathBuf, Option<PathBuf>)> = Vec::new();
    if let Some(installed) = the_binary_on_the_path(ClientKind::Rfirma.name()) {
        places.push((Origin::Installed, installed, None));
    }
    for profile in ["release", "debug"] {
        let local = primary
            .join("rfirma-app/src-tauri/target")
            .join(profile)
            .join("rfirma");
        let local_library = the_library_dir_of(&primary, &primary);
        places.push((Origin::LocalBuild, local, Some(local_library)));
        let shared = primary
            .join(".claude/worktrees/target")
            .join(profile)
            .join("rfirma");
        let built_from = the_checkout_that_built(&shared).unwrap_or_else(|| primary.clone());
        let shared_library = the_library_dir_of(&built_from, &primary);
        places.push((Origin::Worktree, shared, Some(shared_library)));
    }
    places
        .into_iter()
        .filter(|(_, binary, _)| is_executable(binary))
        .filter_map(|(origin, binary, library_dir)| {
            let identity = the_identity_of_rfirma(&binary, library_dir.as_deref()).ok()?;
            Some(Candidate {
                kind: ClientKind::Rfirma,
                origin,
                binary,
                library_dir,
                identity,
            })
        })
        .collect()
}

/// Lo que dice de sí mismo el rFirma de `binary`, o por qué no se puede medir: un build sin
/// `custom-protocol` abre su ventana contra el servidor de desarrollo, y sin él queda en blanco.
pub(crate) fn the_identity_of_rfirma(
    binary: &Path,
    library_dir: Option<&Path>,
) -> Result<Identity, String> {
    let serves_itself = File::open(binary)
        .map(|mut file| embeds_its_frontend(&mut file))
        .unwrap_or(false);
    if !serves_itself && !the_dev_server_is_up() {
        return Err(format!(
            "{} es un build sin custom-protocol y el servidor de desarrollo no está levantado: \
             compílalo con `just conformance-candidate` o levanta `just dev`",
            binary.display()
        ));
    }
    let printed = the_version_printed_by(binary, library_dir)?;
    the_identity_in(&printed).ok_or_else(|| {
        format!(
            "{} --version no dice qué rFirma es: «{}»",
            binary.display(),
            printed.trim()
        )
    })
}

fn the_version_printed_by(binary: &Path, library_dir: Option<&Path>) -> Result<String, String> {
    let mut command = Command::new(binary);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(dir) = library_dir {
        command.env("RFIRMA_LIB_DIR", dir);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("{} --version no arrancó: {error}", binary.display()))?;
    let deadline = Instant::now() + THE_PATIENCE_OF_VERSION;
    while child.try_wait().ok().flatten().is_none() {
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{} --version no ha terminado", binary.display()));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut printed = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut printed);
    }
    Ok(printed)
}

fn the_identity_in(version_line: &str) -> Option<Identity> {
    let rest = version_line.trim().strip_prefix("rfirma ")?;
    let (version, rest) = rest.split_once(" (")?;
    let channel = rest.split([',', ')']).next()?.trim();
    Some(Identity {
        version: Some(version.to_owned()),
        channel: (!channel.is_empty()).then(|| channel.to_owned()),
    })
}

/// Si el binario empotra la interfaz, que Tauri solo guarda con `custom-protocol`: sus recursos
/// van por nombre, `/assets/<nombre>.js`.
fn embeds_its_frontend(binary: &mut impl Read) -> bool {
    let mut window: Vec<u8> = Vec::with_capacity(THE_CHUNK + THE_LONGEST_ASSET_NAME);
    let mut chunk = vec![0u8; THE_CHUNK];
    loop {
        let read = match binary.read(&mut chunk) {
            Ok(0) | Err(_) => return false,
            Ok(read) => read,
        };
        window.extend_from_slice(&chunk[..read]);
        if names_a_script_asset(&window) {
            return true;
        }
        let kept = window.len().min(THE_LONGEST_ASSET_NAME);
        window.drain(..window.len() - kept);
    }
}

fn names_a_script_asset(bytes: &[u8]) -> bool {
    const PREFIX: &[u8] = b"/assets/";
    memchr::memmem::find_iter(bytes, PREFIX).any(|at| {
        let name: Vec<u8> = bytes[at + PREFIX.len()..]
            .iter()
            .copied()
            .take_while(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(byte))
            .collect();
        name.len() > ".js".len() && name.ends_with(b".js")
    })
}

fn the_dev_server_is_up() -> bool {
    let configuration = the_suite_checkout().join("rfirma-app/src-tauri/tauri.conf.json");
    let Some(address) = std::fs::read_to_string(configuration)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|config| config["build"]["devUrl"].as_str().map(str::to_owned))
    else {
        return false;
    };
    let authority = address
        .split_once("://")
        .map_or(address.as_str(), |(_, rest)| rest)
        .split('/')
        .next()
        .unwrap_or_default();
    authority
        .to_socket_addrs()
        .into_iter()
        .flatten()
        .any(|socket| TcpStream::connect_timeout(&socket, Duration::from_millis(300)).is_ok())
}

fn the_suite_checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_owned)
        .unwrap_or_default()
}

/// El checkout principal: el que guarda `checkout` en `.claude/worktrees/`, o él mismo.
fn the_primary_checkout_of(checkout: &Path) -> PathBuf {
    checkout
        .parent()
        .filter(|parent| parent.ends_with(".claude/worktrees"))
        .and_then(|worktrees| worktrees.parent()?.parent())
        .map_or_else(|| checkout.to_owned(), Path::to_owned)
}

/// El checkout que apuntó `just conformance-candidate` junto al binario, si no se ha vuelto a
/// compilar después.
fn the_checkout_that_built(binary: &Path) -> Option<PathBuf> {
    let note = binary.with_file_name("rfirma.checkout");
    let modified = |path: &Path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    };
    if modified(&note)? < modified(binary)? {
        return None;
    }
    let checkout = std::fs::read_to_string(note).ok()?;
    Some(PathBuf::from(checkout.trim()))
}

/// La biblioteca nativa de `checkout` o, si no la ha compilado, la del checkout principal.
fn the_library_dir_of(checkout: &Path, primary: &Path) -> PathBuf {
    let in_the_bridge = |root: &Path| root.join("rfirma-native-bridge/target/lib/rfirma");
    let own = in_the_bridge(checkout);
    if own
        .join(format!("{DLL_PREFIX}rfirma_crypto{DLL_SUFFIX}"))
        .is_file()
    {
        own
    } else {
        in_the_bridge(primary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_version_and_the_channel_from_the_version_line_of_rfirma() {
        let identity = the_identity_in(
            "rfirma 0.16.0 (nativo, linux x86_64) · compatible con AutoFirma 1.9.2 · lib: /usr/lib/rfirma/librfirma_crypto.so\n",
        );

        assert_eq!(
            identity,
            Some(Identity {
                version: Some("0.16.0".to_owned()),
                channel: Some("nativo".to_owned()),
            })
        );
    }

    #[test]
    fn a_version_line_that_is_not_of_rfirma_gives_no_identity() {
        assert_eq!(the_identity_in("AutoFirma 1.9.2\n"), None);
    }

    #[test]
    fn a_build_that_serves_its_own_frontend_carries_the_names_of_its_assets() {
        let mut built = b"\0\x01tauri\0/assets/main-etVtNAB6.js\0rest".as_slice();
        let mut dev = b"\0\x01tauri\0/assets/\0http://localhost:1420\0".as_slice();

        assert!(embeds_its_frontend(&mut built));
        assert!(!embeds_its_frontend(&mut dev));
    }

    #[test]
    fn an_asset_name_split_across_two_reads_is_still_found() {
        let mut bytes = vec![0u8; THE_CHUNK - 5];
        bytes.extend_from_slice(b"/assets/sede-D4YERRvW.js\0");

        assert!(embeds_its_frontend(&mut bytes.as_slice()));
    }

    #[test]
    fn a_worktree_finds_the_primary_checkout_that_holds_it() {
        let primary = the_primary_checkout_of(Path::new("/src/rfirma/.claude/worktrees/wf_1"));

        assert_eq!(primary, PathBuf::from("/src/rfirma"));
        assert_eq!(
            the_primary_checkout_of(Path::new("/src/rfirma")),
            PathBuf::from("/src/rfirma")
        );
    }
}
