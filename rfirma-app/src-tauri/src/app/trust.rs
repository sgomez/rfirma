//! **La CA local en los almacenes NSS de la persona**: instalarla, solaparla y
//! renovarla (ADR-0005, ID-221, ID-224, ID-227, ID-228).
//!
//! Es el caso de uso que junta las tres piezas que ninguna sabe de las otras:
//! el almacén de dos ficheros de [`crate::tls::LocalCaStore`], los perfiles que
//! encuentra [`crate::pkcs11::stores::nss_profiles`] y el registro de
//! [`crate::trust::TrustStores`].
//!
//! # Lo que decide, en una frase cada uno
//!
//! - **Cuándo**: al arrancar y nunca a mitad de un trámite (ID-224). Lo dice
//!   [`crate::trust::work_at`] y aquí se obedece.
//! - **Qué CA**: la que hay si le sobra vida, una nueva si no había o caducó, y
//!   la siguiente —**junto a** la vigente— dentro del solape.
//! - **Dónde**: en todos los perfiles, y **un perfil que falle no deja sin CA a
//!   los demás**, igual que un almacén que no carga no deja sin certificados al
//!   listado (ID-03).
//! - **Qué se dice**: nada mientras se hace, y «reinicia el navegador» **al
//!   terminar**, y solo si de verdad ha entrado en algún almacén.
//!
//! # Un almacén que ya la tiene no se toca
//!
//! Antes de escribir se **leen los bits** (ID-227, TD-60): un perfil que ya
//! tiene la CA local marcada como CA de confianza para TLS no se reescribe y no
//! cuenta para el aviso. Sin eso, cada arranque pediría reiniciar el navegador.

use std::path::{Path, PathBuf};

use crate::tls::{LocalCa, LocalCaStore, TlsError};
use crate::trust::{
    self, nss::is_trusted_ssl_ca, Moment, PendingNotice, Stage, TrustError, TrustStores, Work,
};

/// Cómo quedó el registro de la CA local, sin nada que interrumpa.
#[derive(Debug)]
pub struct TrustOutcome {
    /// En qué punto de su vida estaba la CA local guardada.
    pub stage: Stage,
    /// Qué se decidió hacer.
    pub work: Work,
    /// Almacenes en los que la CA local **está** de confianza al terminar,
    /// tanto si ya lo estaba como si ha entrado ahora.
    pub trusted: usize,
    /// Los almacenes que se han quedado sin ella, con el motivo de cada uno.
    /// No para nada: es lo que se cuenta al final.
    pub missed: Vec<(PathBuf, TrustError)>,
    /// El aviso, que **solo sale al terminar el trámite** (ID-224).
    pub notice: PendingNotice,
}

impl TrustOutcome {
    /// Ni un almacén con la CA local: la sede no va a poder abrir el canal.
    pub fn nowhere(&self) -> bool {
        self.trusted == 0
    }
}

/// **Deja la CA local instalada y de confianza en los perfiles NSS.**
///
/// Devuelve el parte de cómo quedó. **No falla por un perfil**: el único error
/// que sale por el `Result` es el del material —no poder leer o escribir la CA
/// local—, porque sin CA no hay nada que registrar en ninguna parte.
pub fn refresh_local_ca_trust(
    store: &LocalCaStore,
    profiles: &[PathBuf],
    stores: &dyn TrustStores,
    moment: Moment,
) -> Result<TrustOutcome, TlsError> {
    let saved = store.read()?;
    let days_left = saved.as_ref().map(LocalCa::days_left).transpose()?;
    let stage = Stage::of(days_left);
    let work = trust::work_at(moment, stage);

    let certificate = match work {
        Work::Nothing => {
            return Ok(TrustOutcome {
                stage,
                work,
                trusted: 0,
                missed: Vec::new(),
                notice: PendingNotice::none(),
            })
        }
        // La que ya hay. `saved` no puede ser `None` aquí: sin CA guardada la
        // etapa es `Absent` y el trabajo, fabricar una.
        Work::InstallTheOneWeHave => saved.expect("la etapa Serving sale de una CA guardada"),
        // Fabricar. En el solape la vigente **se queda donde está** dentro de
        // los almacenes NSS: aquí solo se sustituye la que sirve.
        Work::MakeOneAndInstallIt | Work::MakeTheNextAndInstallItToo => {
            let fresh = LocalCa::generate()?;
            store.write(&fresh)?;
            fresh
        }
    };

    let der = certificate.certificate().to_der().map_err(|error| {
        TlsError::new(
            crate::tls::Situation::MaterialDamaged,
            format!("el certificado de la CA local no sale en DER: {error}"),
        )
    })?;

    let mut trusted = 0;
    let mut installed = 0;
    let mut missed = Vec::new();

    for profile in profiles {
        match settle(stores, profile, &der) {
            Ok(Settled::AlreadyThere) => trusted += 1,
            Ok(Settled::JustInstalled) => {
                trusted += 1;
                installed += 1;
            }
            Err(error) => missed.push((profile.clone(), error)),
        }
    }

    Ok(TrustOutcome {
        stage,
        work,
        trusted,
        missed,
        notice: if installed > 0 {
            PendingNotice::after_installing()
        } else {
            PendingNotice::none()
        },
    })
}

/// Cómo acabó un almacén concreto.
enum Settled {
    /// Ya tenía la CA local con los bits puestos.
    AlreadyThere,
    /// Ha entrado ahora.
    JustInstalled,
}

/// **El éxito de la escritura no es la señal**: se vuelve a leer.
///
/// El ADR-0005 lo deja escrito y tiene un caso concreto detrás: los bits de
/// confianza son atributos autenticados, y en un perfil con contraseña maestra
/// el certificado puede entrar con confianza `,,` **sin que nada falle**. Un
/// éxito parcial silencioso contado como almacén instalado sería peor que un
/// error: la sede fallaría después, y el parte diría que todo fue bien.
fn settle(stores: &dyn TrustStores, profile: &Path, der: &[u8]) -> Result<Settled, TrustError> {
    if stores
        .trust_of(profile, der)?
        .is_some_and(is_trusted_ssl_ca)
    {
        return Ok(Settled::AlreadyThere);
    }
    stores.install(profile, der, crate::tls::authority::COMMON_NAME)?;
    if !stores
        .trust_of(profile, der)?
        .is_some_and(is_trusted_ssl_ca)
    {
        return Err(TrustError::new(
            crate::trust::Situation::TrustNotWritten,
            format!(
                "la CA local ha entrado en «{}» pero sin los bits de confianza \
                 (¿contraseña maestra en el perfil?)",
                profile.display()
            ),
        ));
    }
    Ok(Settled::JustInstalled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::{nss::TRUSTED_SSL_CA, Notice, Situation};
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Un certificado registrado en el doble: su DER y con qué apodo entró.
    type Registered = (Vec<u8>, String);

    /// Un doble de los almacenes NSS: recuerda qué le han metido a cada perfil
    /// y **nunca borra nada**, que es la propiedad del solape.
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

        fn trust_of(
            &self,
            profile: &Path,
            certificate_der: &[u8],
        ) -> Result<Option<u32>, TrustError> {
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

    /// **ID-224.** Con un trámite en marcha no se toca ni un almacén, ni
    /// siquiera cuando no hay CA local ninguna.
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
    }

    /// El aviso no se repite: el segundo arranque encuentra los bits puestos,
    /// no escribe y no dice nada.
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

    /// **El solape.** La CA siguiente **se añade**; la vigente sigue dentro del
    /// almacén, con el mismo apodo y sin que nadie la haya retirado.
    #[test]
    fn the_next_local_ca_goes_in_next_to_the_current_one() {
        let (_directory, store) = a_store();
        let profiles = profiles();
        let stores = Doubled::with_profiles(&[&profiles[0], &profiles[1]]);
        // La CA vigente, ya registrada e instalada, a la que le queda un día:
        // es lo que dispara el solape.
        let current = LocalCa::almost_expired_for_test().expect("deberia fabricarse");
        let current_der = current
            .certificate()
            .to_der()
            .expect("deberia salir en DER");
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
    }

    /// **ID-03 otra vez.** Un perfil que no se deja escribir no deja sin CA a
    /// los demás, y lo que no ha entrado se cuenta al final.
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

    /// Sin ningún perfil no hay dónde instalar, y eso se dice: es el caso en el
    /// que la sede no va a poder abrir el canal.
    #[test]
    fn a_machine_without_nss_profiles_ends_up_with_the_ca_nowhere() {
        let (_directory, store) = a_store();
        let stores = Doubled::default();

        let outcome = refresh_local_ca_trust(&store, &[], &stores, Moment::Startup)
            .expect("no haber perfiles no es un fallo del material");

        assert!(outcome.nowhere());
        assert!(!outcome.notice.is_pending());
    }
}
