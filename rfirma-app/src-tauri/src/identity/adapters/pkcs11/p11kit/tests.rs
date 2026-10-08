use super::*;
use crate::desktop::domain::channel::Channel;

const P11_KIT_TRUST: &str = "\
# This is a module config for the 'included' p11-kit trust module
module: p11-kit-trust.so
priority: 1
trust-policy: yes
x-trust-lookup: pkcs11:library-description=PKCS%2311%20Kit%20Trust%20Module
disable-in: p11-kit-proxy
x-init-reserved:
";

const FLATPAK_TRUST_FORWARDING: &str = "\
# This overrides the runtime p11-kit-trusted module with a client one talking to the trust module on the host
module: p11-kit-client.so
";

const GNOME_KEYRING: &str = "\
# The file is installed/loaded from the default module p11-kit directory
module: gnome-keyring-pkcs11.so
enable-in: geary, midori
x-trust-store: pkcs11:library-manufacturer=GNOME%20Keyring;serial=1:XDG:DEFAULT
";

const OPENSC: &str = "\
# This file describes how to load the opensc module
module: opensc-pkcs11.so
";

struct Installation {
    root: tempfile::TempDir,
}

impl Installation {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("deberia poder crearse un directorio temporal"),
        }
    }

    fn usr(&self) -> PathBuf {
        self.root.path().join("usr")
    }

    fn system(&self) -> PathBuf {
        self.root.path().join("usr/share/p11-kit/modules")
    }

    fn etc(&self) -> PathBuf {
        self.root.path().join("etc/pkcs11/modules")
    }

    fn user(&self) -> PathBuf {
        self.root.path().join("home/.config/pkcs11/modules")
    }

    fn library(&self, relative_to_usr: &str) -> PathBuf {
        let path = self.usr().join(relative_to_usr);
        std::fs::create_dir_all(path.parent().expect("tiene padre"))
            .expect("deberia poder crearse el directorio de la biblioteca");
        std::fs::write(&path, b"").expect("deberia poder escribirse la biblioteca");
        path
    }

    fn registers(&self, directory: &Path, file: &str, text: &str) {
        std::fs::create_dir_all(directory).expect("deberia poder crearse el directorio");
        std::fs::write(directory.join(file), text).expect("deberia poder escribirse el .module");
    }

    fn app(&self) -> PathBuf {
        self.root.path().join("app")
    }

    fn app_registry(&self) -> PathBuf {
        self.app().join("share/p11-kit/modules")
    }

    fn app_library(&self, relative_to_app: &str) -> PathBuf {
        let path = self.app().join(relative_to_app);
        std::fs::create_dir_all(path.parent().expect("tiene padre"))
            .expect("deberia poder crearse el directorio de la biblioteca");
        std::fs::write(&path, b"").expect("deberia poder escribirse la biblioteca");
        path
    }

    fn registered(&self) -> Vec<PathBuf> {
        registered_modules(&[self.system(), self.user()], &self.usr(), None)
    }

    fn registered_under_app(&self) -> Vec<PathBuf> {
        let app = self.app();
        registered_modules(&[self.app_registry()], &self.usr(), Some(&app))
    }
}

#[test]
fn a_module_file_names_its_library() {
    assert_eq!(
        module_for_rfirma(OPENSC).as_deref(),
        Some("opensc-pkcs11.so")
    );
}

#[test]
fn a_trust_policy_module_is_a_ca_store_and_not_a_key_store() {
    assert_eq!(module_for_rfirma(P11_KIT_TRUST), None);
}

#[test]
fn the_trust_forwarding_client_named_by_its_absolute_path_is_not_a_card() {
    assert_eq!(
        module_for_rfirma("module: /usr/lib/x86_64-linux-gnu/pkcs11/p11-kit-client.so\n"),
        None
    );
}

#[test]
fn a_module_enabled_only_in_other_programs_stays_out() {
    assert_eq!(module_for_rfirma(GNOME_KEYRING), None);
}

#[test]
fn a_module_enabled_in_rfirma_comes_in() {
    assert_eq!(
        module_for_rfirma("module: a.so\nenable-in: geary,rfirma midori\n").as_deref(),
        Some("a.so")
    );
}

#[test]
fn a_module_disabled_in_rfirma_stays_out() {
    assert_eq!(
        module_for_rfirma("module: a.so\ndisable-in: rfirma\n"),
        None
    );
}

#[test]
fn a_module_disabled_only_in_other_programs_comes_in() {
    assert_eq!(
        module_for_rfirma("module: a.so\ndisable-in: p11-kit-proxy\n").as_deref(),
        Some("a.so")
    );
}

#[test]
fn a_program_name_that_only_contains_rfirma_is_another_program() {
    assert_eq!(
        module_for_rfirma("module: a.so\nenable-in: rfirma-helper\n"),
        None
    );
}

#[test]
fn a_file_without_module_registers_nothing() {
    assert_eq!(module_for_rfirma("# nada\npriority: 3\n"), None);
}

#[test]
fn opensc_resolves_under_the_multiarch_pkcs11_directory() {
    let installation = Installation::new();
    let opensc = installation.library("lib/x86_64-linux-gnu/pkcs11/opensc-pkcs11.so");
    installation.registers(&installation.system(), "opensc-pkcs11.module", OPENSC);

    assert_eq!(installation.registered(), vec![opensc]);
}

#[test]
fn a_relative_module_resolves_under_lib64_and_under_lib_too() {
    let installation = Installation::new();
    let fedora = installation.library("lib64/pkcs11/a.so");
    let arch = installation.library("lib/pkcs11/b.so");
    installation.registers(&installation.system(), "a.module", "module: a.so\n");
    installation.registers(&installation.system(), "b.module", "module: b.so\n");

    assert_eq!(installation.registered(), vec![fedora, arch]);
}

#[test]
fn a_relative_module_that_is_not_installed_is_left_out() {
    let installation = Installation::new();
    installation.registers(&installation.system(), "opensc-pkcs11.module", OPENSC);

    assert!(installation.registered().is_empty());
}

#[test]
fn an_absolute_module_is_kept_as_it_comes() {
    let installation = Installation::new();
    let softhsm = installation.library("lib/x86_64-linux-gnu/softhsm/libsofthsm2.so");
    installation.registers(
        &installation.system(),
        "softhsm2.module",
        &format!("module: {}\n", softhsm.display()),
    );

    assert_eq!(installation.registered(), vec![softhsm]);
}

#[test]
fn a_user_module_file_overrides_the_system_one_with_the_same_name() {
    let installation = Installation::new();
    let system = installation.library("lib/pkcs11/system.so");
    let user = installation.library("lib/pkcs11/user.so");
    installation.registers(
        &installation.system(),
        "token.module",
        "module: system.so\n",
    );
    installation.registers(&installation.user(), "token.module", "module: user.so\n");

    let registered = installation.registered();

    assert_eq!(registered, vec![user]);
    assert!(!registered.contains(&system));
}

#[test]
fn a_user_module_file_can_take_a_system_module_out() {
    let installation = Installation::new();
    installation.library("lib/pkcs11/opensc-pkcs11.so");
    installation.registers(&installation.system(), "opensc-pkcs11.module", OPENSC);
    installation.registers(
        &installation.user(),
        "opensc-pkcs11.module",
        "module: opensc-pkcs11.so\ndisable-in: rfirma\n",
    );

    assert!(installation.registered().is_empty());
}

#[test]
fn the_system_ca_store_and_the_keyring_of_p11_kit_never_reach_the_listing() {
    let installation = Installation::new();
    installation.library("lib/x86_64-linux-gnu/pkcs11/p11-kit-trust.so");
    installation.library("lib/x86_64-linux-gnu/pkcs11/gnome-keyring-pkcs11.so");
    let opensc = installation.library("lib/x86_64-linux-gnu/pkcs11/opensc-pkcs11.so");
    installation.registers(
        &installation.system(),
        "p11-kit-trust.module",
        P11_KIT_TRUST,
    );
    installation.registers(
        &installation.system(),
        "gnome-keyring.module",
        GNOME_KEYRING,
    );
    installation.registers(&installation.system(), "opensc-pkcs11.module", OPENSC);

    assert_eq!(installation.registered(), vec![opensc]);
}

#[test]
fn the_flatpak_trust_forwarding_that_overrides_the_runtime_one_is_not_a_card() {
    let installation = Installation::new();
    installation.library("lib/x86_64-linux-gnu/pkcs11/p11-kit-trust.so");
    installation.library("lib/x86_64-linux-gnu/pkcs11/p11-kit-client.so");
    installation.registers(
        &installation.system(),
        "p11-kit-trust.module",
        P11_KIT_TRUST,
    );
    installation.registers(
        &installation.etc(),
        "p11-kit-trust.module",
        FLATPAK_TRUST_FORWARDING,
    );

    let registered = registered_modules(
        &[installation.system(), installation.etc()],
        &installation.usr(),
        None,
    );

    assert!(registered.is_empty());
}

#[test]
fn a_file_that_does_not_end_in_module_is_not_read() {
    let installation = Installation::new();
    installation.library("lib/pkcs11/opensc-pkcs11.so");
    installation.registers(&installation.system(), "opensc-pkcs11.module.bak", OPENSC);

    assert!(installation.registered().is_empty());
}

#[test]
fn the_user_directory_is_read_after_the_two_of_the_system() {
    let home = Path::new("/home/alguien");

    assert_eq!(
        configuration_directories(home, Channel::Native),
        vec![
            PathBuf::from("/usr/share/p11-kit/modules"),
            PathBuf::from("/etc/pkcs11/modules"),
            PathBuf::from("/home/alguien/.config/pkcs11/modules"),
        ]
    );
}

#[test]
fn in_the_flatpak_a_relative_module_under_app_resolves_to_the_app_library() {
    let installation = Installation::new();
    let library = installation.app_library("lib/pkcs11/opensc-pkcs11.so");
    installation.registers(&installation.app_registry(), "opensc.module", OPENSC);

    assert_eq!(installation.registered_under_app(), vec![library]);
}

#[test]
fn outside_the_flatpak_the_same_tree_adds_no_module() {
    let installation = Installation::new();
    installation.app_library("lib/pkcs11/opensc-pkcs11.so");
    installation.registers(&installation.app_registry(), "opensc.module", OPENSC);

    let registered = registered_modules(&[installation.app_registry()], &installation.usr(), None);

    assert!(registered.is_empty());
}

#[test]
fn only_the_flatpak_searches_the_app_registry_and_root() {
    let home = Path::new("/home/alguien");

    assert!(configuration_directories(home, Channel::Flatpak)
        .contains(&PathBuf::from("/app/share/p11-kit/modules")));
    assert!(!configuration_directories(home, Channel::Native)
        .contains(&PathBuf::from("/app/share/p11-kit/modules")));
    assert_eq!(app_root(Channel::Flatpak), Some(Path::new("/app")));
    assert_eq!(app_root(Channel::Native), None);
}
