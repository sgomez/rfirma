use super::*;
use crate::trust::{nss::TRUSTED_SSL_CA, Notice, Situation};
use std::collections::HashMap;
use std::sync::Mutex;

type Registered = (Vec<u8>, String);

#[derive(Default)]
struct Doubled {
    contents: Mutex<HashMap<PathBuf, Vec<Registered>>>,
    refuse: Vec<PathBuf>,
}

impl Doubled {
    fn with_profiles(profiles: &[&Path]) -> Self {
        let mut contents = HashMap::new();
        for profile in profiles {
            contents.insert(profile.to_path_buf(), Vec::new());
        }
        Self {
            contents: Mutex::new(contents),
            refuse: Vec::new(),
        }
    }

    fn refusing(mut self, profile: &Path) -> Self {
        self.refuse.push(profile.to_path_buf());
        self
    }

    fn inside(&self, profile: &Path) -> Vec<Registered> {
        self.contents
            .lock()
            .expect("el doble no envenena su cerrojo")
            .get(profile)
            .cloned()
            .unwrap_or_default()
    }
}

impl TrustStores for Doubled {
    fn install(
        &self,
        profile: &Path,
        certificate_der: &[u8],
        nickname: &str,
    ) -> Result<(), TrustError> {
        if self.refuse.contains(&profile.to_path_buf()) {
            return Err(TrustError::new(
                Situation::StoreUnreachable,
                "el doble no deja escribir en este perfil",
            ));
        }
        self.contents
            .lock()
            .expect("el doble no envenena su cerrojo")
            .entry(profile.to_path_buf())
            .or_default()
            .push((certificate_der.to_vec(), nickname.to_owned()));
        Ok(())
    }

    fn trust_of(&self, profile: &Path, certificate_der: &[u8]) -> Result<Option<u32>, TrustError> {
        Ok(self
            .inside(profile)
            .iter()
            .any(|(der, _)| der == certificate_der)
            .then_some(TRUSTED_SSL_CA))
    }
}

fn a_store() -> (tempfile::TempDir, LocalCaStore) {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let store = LocalCaStore::of(&crate::paths::Paths::under(directory.path()));
    (directory, store)
}

fn der_of(ca: &LocalCa) -> Vec<u8> {
    ca.certificate().to_der().expect("deberia salir en DER")
}

fn profiles() -> [PathBuf; 2] {
    [
        PathBuf::from("/perfiles/firefox"),
        PathBuf::from("/perfiles/chrome"),
    ]
}

#[test]
fn the_first_boot_makes_the_local_ca_and_leaves_it_trusted_everywhere() {
    let (_directory, store) = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);

    let outcome = refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("deberia poder fabricarse y registrarse");

    assert_eq!(outcome.stage, Stage::Absent);
    assert_eq!(outcome.work, Work::MakeOneAndInstallIt);
    assert_eq!(outcome.trusted, 2);
    assert!(outcome.missed.is_empty());
    assert!(!outcome.nowhere());
    assert_eq!(stores.inside(&profiles[0]).len(), 1);
}
#[test]
fn nothing_is_written_in_the_middle_of_an_errand() {
    let (_directory, store) = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);

    let outcome = refresh_local_ca_trust(&store, &profiles, &stores, Moment::MidErrand)
        .expect("no hacer nada no es un fallo");

    assert_eq!(outcome.work, Work::Nothing);
    assert!(stores.inside(&profiles[0]).is_empty());
    assert!(store.read().expect("deberia leerse").is_none());
    assert!(!outcome.looked(), "no se ha consultado ni un almacén");
    assert!(
        !outcome.nowhere(),
        "sin mirar no se puede decir que la CA no esté en ninguna parte"
    );
}
#[test]
fn the_second_boot_says_nothing_because_the_bits_are_already_there() {
    let (_directory, store) = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);

    let mut first = refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("deberia registrarse");
    let mut second = refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("deberia registrarse");

    assert_eq!(
        first.notice.when_the_errand_ends(),
        Some(Notice::RestartTheBrowser)
    );
    assert_eq!(second.notice.when_the_errand_ends(), None);
    assert_eq!(second.work, Work::InstallTheOneWeHave);
    assert_eq!(stores.inside(&profiles[0]).len(), 1);
}
#[test]
fn the_next_local_ca_goes_in_next_to_the_current_one() {
    let (_directory, store) = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let current = LocalCa::almost_expired_for_test().expect("deberia fabricarse");
    let current_der = der_of(&current);
    store.write(&current).expect("deberia guardarse");
    for profile in &profiles {
        stores
            .install(profile, &current_der, crate::tls::authority::COMMON_NAME)
            .expect("deberia registrarse la vigente");
    }

    let outcome = refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("deberia registrarse la siguiente");

    assert_eq!(outcome.stage, Stage::Overlapping);
    assert_eq!(outcome.work, Work::MakeTheNextAndInstallItToo);
    let inside = stores.inside(&profiles[0]);
    assert_eq!(inside.len(), 2, "se añade, nunca se sustituye");
    assert!(
        inside.iter().any(|(der, _)| der == &current_der),
        "la CA vigente sigue de confianza durante el solape"
    );
    assert!(
        inside
            .iter()
            .all(|(_, nickname)| nickname == crate::tls::authority::COMMON_NAME),
        "las dos comparten apodo: en NSS el apodo va con el sujeto"
    );
    assert_eq!(
        der_of(&store.read().unwrap().unwrap()),
        current_der,
        "la que firma el certificado del servidor local sigue siendo la vigente"
    );
    assert!(
        store.read_next().unwrap().is_some(),
        "la siguiente espera su turno en su propia ranura"
    );
}
#[test]
fn the_next_local_ca_is_not_remade_on_every_boot_of_the_overlap() {
    let (_directory, store) = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    store
        .write(&LocalCa::almost_expired_for_test().expect("deberia fabricarse"))
        .expect("deberia guardarse");

    refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("deberia empezar el solape");
    let next_der = der_of(&store.read_next().unwrap().unwrap());
    let mut second = refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("deberia repetirse sin fabricar nada");

    assert_eq!(second.work, Work::InstallBothOfThem);
    assert_eq!(stores.inside(&profiles[0]).len(), 2);
    assert_eq!(
        der_of(&store.read_next().unwrap().unwrap()),
        next_der,
        "la siguiente es la misma, no una tercera"
    );
    assert_eq!(second.notice.when_the_errand_ends(), None);
}
#[test]
fn the_waiting_local_ca_takes_over_without_asking_for_a_restart() {
    let (_directory, store) = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let expired = LocalCa::expired_for_test().expect("deberia fabricarse");
    let waiting = LocalCa::generate().expect("deberia fabricarse");
    store.write(&expired).expect("deberia guardarse");
    store.write_next(&waiting).expect("deberia guardarse");
    for profile in &profiles {
        for ca in [&expired, &waiting] {
            stores
                .install(profile, &der_of(ca), crate::tls::authority::COMMON_NAME)
                .expect("deberia registrarse");
        }
    }

    let mut outcome = refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("deberia poder relevarse");

    assert_eq!(outcome.stage, Stage::Expired);
    assert_eq!(outcome.work, Work::PromoteTheNextOne);
    assert_eq!(
        der_of(&store.read().unwrap().unwrap()),
        der_of(&waiting),
        "la que esperaba es ahora la que sirve"
    );
    assert!(store.read_next().unwrap().is_none());
    assert_eq!(
        outcome.notice.when_the_errand_ends(),
        None,
        "ya estaba instalada: nadie tiene que reiniciar nada"
    );
}
#[test]
fn an_expired_local_ca_with_no_successor_starts_again_from_scratch() {
    let (_directory, store) = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
    let expired = LocalCa::expired_for_test().expect("deberia fabricarse");
    store.write(&expired).expect("deberia guardarse");

    let mut outcome = refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("deberia fabricarse otra");

    assert_eq!(outcome.work, Work::MakeOneAndInstallIt);
    assert_ne!(der_of(&store.read().unwrap().unwrap()), der_of(&expired));
    assert_eq!(
        outcome.notice.when_the_errand_ends(),
        Some(Notice::RestartTheBrowser)
    );
}
#[test]
fn a_profile_that_refuses_does_not_leave_the_others_without_the_local_ca() {
    let (_directory, store) = a_store();
    let profiles = profiles();
    let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]).refusing(&profiles[1]);

    let outcome = refresh_local_ca_trust(&store, &profiles, &stores, Moment::Startup)
        .expect("un perfil que falla no es un fallo del material");

    assert_eq!(outcome.trusted, 1);
    assert_eq!(outcome.missed.len(), 1);
    assert_eq!(outcome.missed[0].0, profiles[1]);
    assert_eq!(outcome.missed[0].1.situation(), Situation::StoreUnreachable);
}
#[test]
fn a_machine_without_nss_profiles_ends_up_with_the_ca_nowhere() {
    let (_directory, store) = a_store();
    let stores = Doubled::default();

    let outcome = refresh_local_ca_trust(&store, &[], &stores, Moment::Startup)
        .expect("no haber perfiles no es un fallo del material");

    assert!(outcome.nowhere());
    assert!(!outcome.notice.is_pending());
}

fn an_outcome(
    trusted: usize,
    missed: Vec<(PathBuf, TrustError)>,
    notice: PendingNotice,
) -> TrustOutcome {
    TrustOutcome {
        stage: Stage::Absent,
        work: Work::MakeOneAndInstallIt,
        trusted,
        missed,
        notice,
    }
}
#[test]
fn nowhere_with_no_profiles_says_there_is_nowhere_to_install() {
    let outcome = an_outcome(0, Vec::new(), PendingNotice::none());

    let lines = narrate_startup_outcome(outcome, &[]);

    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("no se ha encontrado ningún almacén"));
}
#[test]
fn nowhere_with_profiles_that_all_refused_names_the_profiles_instead() {
    let profile = PathBuf::from("/home/persona/.mozilla/firefox/perfil");
    let missed = vec![(
        profile.clone(),
        TrustError::new(Situation::StoreUnreachable, "el perfil está bloqueado"),
    )];
    let outcome = an_outcome(0, missed, PendingNotice::none());

    let lines = narrate_startup_outcome(outcome, std::slice::from_ref(&profile));

    assert_eq!(lines.len(), 2);
    assert!(!lines[0].contains("no se ha encontrado ningún almacén"));
    assert!(lines[0].contains("no ha entrado en ninguno"));
    assert!(lines[1].contains("1 almacén(es) NSS"));
}
#[test]
fn an_outcome_that_was_already_trusted_says_nothing() {
    let outcome = an_outcome(1, Vec::new(), PendingNotice::none());

    let lines = narrate_startup_outcome(outcome, &[PathBuf::from("/home/persona/perfil")]);

    assert!(lines.is_empty());
}
#[test]
fn installing_asks_to_restart_the_browser() {
    let outcome = an_outcome(1, Vec::new(), PendingNotice::after_installing());

    let lines = narrate_startup_outcome(outcome, &[PathBuf::from("/home/persona/perfil")]);

    assert_eq!(
        lines,
        vec!["rfirma: se ha instalado la CA local; reinicia el navegador"]
    );
}
#[test]
fn one_missed_profile_among_others_only_reports_the_miss() {
    let profile = PathBuf::from("/home/persona/perfil-b");
    let missed = vec![(
        profile.clone(),
        TrustError::new(Situation::StoreUnreachable, "sin permiso de escritura"),
    )];
    let outcome = an_outcome(1, missed, PendingNotice::after_installing());

    let lines =
        narrate_startup_outcome(outcome, &[PathBuf::from("/home/persona/perfil-a"), profile]);

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("reinicia el navegador"));
    assert!(lines[1].contains("1 almacén(es) NSS"));
}
