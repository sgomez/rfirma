//! **La confianza**: cómo entra la CA local en los almacenes NSS de la persona
//! y cuándo toca renovarla (ADR-0005, ID-221, ID-224, ID-227, ID-228).
//!
//! [`crate::tls`] fabrica el material y no registra nada; aquí se registra, y
//! **no se fabrica nada**. Son dos mitades del mismo ADR con dos motivos
//! distintos para cambiar.
//!
//! | Pieza | Qué es |
//! |---|---|
//! | [`Stage`] | En qué punto de su vida está la CA local guardada |
//! | [`Moment`] | Si estamos arrancando o **a mitad de un trámite** (ID-224) |
//! | [`PendingNotice`] | El aviso de reiniciar el navegador, que solo sale al terminar |
//! | [`TrustStores`] | El puerto que escribe en un almacén NSS, doblable en pruebas |
//! | [`nss::NssTrustStores`] | Su única implementación de verdad, por FFI |
//!
//! # Las tres reglas que este módulo sostiene
//!
//! - **El solape** (ID-224): instalar la CA siguiente **no retira la vigente**.
//!   [`TrustStores::install`] solo añade; no hay ninguna llamada que borre.
//!   Dos certificados de confianza con el mismo sujeto conviven en Firefox y en
//!   Chrome, en cualquier orden, y por eso comparten apodo: en NSS el apodo va
//!   con el sujeto, no con el certificado.
//! - **No se repara en caliente** (ID-224): [`work_at`] devuelve
//!   [`Work::Nothing`] siempre que el momento sea [`Moment::MidErrand`], sea
//!   cual sea la etapa. Chrome no relee su `nssdb` y Firefox envenena su caché
//!   de confianza tras haber fallado, así que «reparar y continuar» no existe.
//! - **El aviso llega al final** (ID-224): [`PendingNotice`] no lo suelta a
//!   mitad de nada; hay que preguntárselo con
//!   [`PendingNotice::when_the_errand_ends`].
//!
//! Comprobar que la confianza está puesta es **leer los bits**
//! ([`TrustStores::trust_of`]), nunca verificar una cadena: el veredicto de
//! `vfychain` sale invertido respecto a Firefox (ID-227, TD-60).

pub mod error;
pub mod nss;

use std::path::Path;

pub use error::{Situation, TrustError};
pub use nss::NssTrustStores;

/// Cuánto antes de caducar se instala la CA local siguiente.
///
/// Es el **solape**: durante estos días hay dos CA locales de confianza en el
/// almacén, la que sirve y la que servirá. Cuatro meses son de sobra para que
/// alguien que firma dos o tres veces al año se encuentre la siguiente ya
/// instalada, que es justo el caso que hace raro el camino de reparar.
pub const OVERLAP_DAYS: i64 = 120;

/// En qué punto de su vida está la CA local que hay guardada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// No hay ninguna: primer arranque, o `$HOME` limpio.
    Absent,
    /// La hay y le sobra vida: nada que hacer.
    Serving,
    /// Le quedan menos de [`OVERLAP_DAYS`]: toca fabricar la siguiente y
    /// **añadirla** sin retirar esta.
    Overlapping,
    /// Ya caducó. El navegador dejó de confiar solo, que es exactamente lo que
    /// la caducidad existe para conseguir.
    Expired,
}

impl Stage {
    /// La etapa a partir de los días que le queden al certificado guardado.
    ///
    /// `None` es «no hay CA local guardada», que **no es un fallo**: es el
    /// primer arranque.
    pub fn of(days_left: Option<i64>) -> Self {
        match days_left {
            None => Stage::Absent,
            Some(days) if days <= 0 => Stage::Expired,
            Some(days) if days < OVERLAP_DAYS => Stage::Overlapping,
            Some(_) => Stage::Serving,
        }
    }
}

/// Cuándo se está mirando la confianza.
///
/// No es un detalle de registro: es lo único que separa instalar de no tocar
/// nada (ID-224).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    /// Al arrancar rfirma, que es el único momento en el que pedir que se
    /// reinicie el navegador no le cuesta nada a nadie.
    Startup,
    /// Con un trámite de una sede en marcha.
    MidErrand,
}

/// Lo que hay que hacer con los almacenes NSS ahora mismo.
///
/// Las tres formas de trabajo **instalan**; lo que las separa es si hace falta
/// fabricar una CA local nueva antes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work {
    /// Nada. Es lo que sale **siempre** a mitad de un trámite.
    Nothing,
    /// Registrar la que ya hay, allí donde no esté. Es lo que cubre el perfil
    /// de Firefox creado después de la instalación de rfirma, y en un almacén
    /// que ya la tiene no hace nada.
    InstallTheOneWeHave,
    /// Fabricar una CA local y registrarla: no había ninguna, o la que había ya
    /// caducó.
    MakeOneAndInstallIt,
    /// Fabricar la siguiente y **añadirla** junto a la vigente, que se queda
    /// donde está. Es el solape del ID-224.
    MakeTheNextAndInstallItToo,
}

/// Qué toca hacer, dado el momento y la etapa.
///
/// A mitad de un trámite la respuesta es [`Work::Nothing`] **siempre**, incluso
/// con la CA caducada: instalarla ahí obligaría a parar el trámite para pedir
/// que se reinicie el navegador y volver a empezar (ID-224).
pub fn work_at(moment: Moment, stage: Stage) -> Work {
    match (moment, stage) {
        (Moment::MidErrand, _) => Work::Nothing,
        (Moment::Startup, Stage::Serving) => Work::InstallTheOneWeHave,
        (Moment::Startup, Stage::Absent | Stage::Expired) => Work::MakeOneAndInstallIt,
        (Moment::Startup, Stage::Overlapping) => Work::MakeTheNextAndInstallItToo,
    }
}

/// Lo que se le dice a la persona cuando se ha tocado un almacén NSS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    /// «Se ha instalado la CA local: reinicia el navegador.» Ni Chrome relee su
    /// `nssdb` en caliente ni Firefox olvida un fallo de confianza anterior.
    RestartTheBrowser,
}

/// Un aviso que espera a que termine el trámite (ID-224).
///
/// No hay forma de sacarlo a mitad: [`PendingNotice::mid_errand`] devuelve
/// `None` siempre, y el aviso solo sale por
/// [`PendingNotice::when_the_errand_ends`], que además **se lo lleva**, para
/// que no se repita en el trámite siguiente.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingNotice(Option<Notice>);

impl PendingNotice {
    /// No hay nada que avisar.
    pub fn none() -> Self {
        Self(None)
    }

    /// Se ha instalado la CA local en al menos un almacén.
    pub fn after_installing() -> Self {
        Self(Some(Notice::RestartTheBrowser))
    }

    /// Lo que se enseña a mitad de un trámite: **nada, nunca**.
    pub fn mid_errand(&self) -> Option<Notice> {
        None
    }

    /// El aviso, ya al terminar el trámite. Se lo lleva.
    pub fn when_the_errand_ends(&mut self) -> Option<Notice> {
        self.0.take()
    }

    /// Si queda algo por avisar.
    pub fn is_pending(&self) -> bool {
        self.0.is_some()
    }
}

/// **El puerto que escribe en los almacenes NSS de la persona.**
///
/// Solo **añade** confianza: no hay ninguna operación que retire, y eso es el
/// solape del ID-224 escrito en el tipo. La retirada explícita —que tiene que
/// llevarse las dos CA vivas y borrar **por huella, nunca por apodo**— es de
/// Preferencias y no de este puerto.
pub trait TrustStores {
    /// Mete el certificado en el almacén de `profile` y le pone los bits de CA
    /// de confianza para TLS. Idempotente: repetirlo no duplica nada.
    fn install(
        &self,
        profile: &Path,
        certificate_der: &[u8],
        nickname: &str,
    ) -> Result<(), TrustError>;

    /// Los bits de confianza TLS que tiene ese certificado en ese almacén, o
    /// `None` si el certificado no está.
    ///
    /// Es la comprobación del ID-227: se **leen los bits**, no se verifica una
    /// cadena.
    fn trust_of(&self, profile: &Path, certificate_der: &[u8]) -> Result<Option<u32>, TrustError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_home_without_a_local_ca_is_the_first_boot_and_not_a_failure() {
        assert_eq!(Stage::of(None), Stage::Absent);
    }

    #[test]
    fn a_local_ca_with_years_left_is_simply_serving() {
        assert_eq!(Stage::of(Some(700)), Stage::Serving);
    }

    /// El solape empieza [`OVERLAP_DAYS`] antes, no el día de la caducidad.
    #[test]
    fn the_next_local_ca_goes_in_months_before_the_current_one_expires() {
        assert_eq!(Stage::of(Some(OVERLAP_DAYS)), Stage::Serving);
        assert_eq!(Stage::of(Some(OVERLAP_DAYS - 1)), Stage::Overlapping);
        assert_eq!(Stage::of(Some(1)), Stage::Overlapping);
    }

    #[test]
    fn a_local_ca_that_ran_out_is_expired_and_not_overlapping() {
        assert_eq!(Stage::of(Some(0)), Stage::Expired);
        assert_eq!(Stage::of(Some(-40)), Stage::Expired);
    }

    /// **ID-224.** «Reparar y continuar» no existe: ni con la CA caducada, ni
    /// sin CA ninguna, ni en el solape.
    #[test]
    fn nothing_is_ever_repaired_in_the_middle_of_an_errand() {
        for stage in [
            Stage::Absent,
            Stage::Serving,
            Stage::Overlapping,
            Stage::Expired,
        ] {
            assert_eq!(work_at(Moment::MidErrand, stage), Work::Nothing);
        }
    }

    #[test]
    fn the_first_boot_makes_a_local_ca_and_so_does_an_expired_one() {
        assert_eq!(
            work_at(Moment::Startup, Stage::Absent),
            Work::MakeOneAndInstallIt
        );
        assert_eq!(
            work_at(Moment::Startup, Stage::Expired),
            Work::MakeOneAndInstallIt
        );
    }

    /// Una CA local con vida de sobra **no se rehace**, pero sí se registra:
    /// el perfil de Firefox creado ayer no la tiene y nadie más va a ponérsela.
    #[test]
    fn a_local_ca_that_still_serves_is_installed_but_never_remade() {
        assert_eq!(
            work_at(Moment::Startup, Stage::Serving),
            Work::InstallTheOneWeHave
        );
    }

    #[test]
    fn the_overlap_installs_the_next_one_without_asking_for_the_current_one_back() {
        assert_eq!(
            work_at(Moment::Startup, Stage::Overlapping),
            Work::MakeTheNextAndInstallItToo
        );
    }

    /// El aviso **no** sale a mitad del trámite, ni siquiera preguntándole
    /// directamente (ID-224).
    #[test]
    fn the_notice_never_shows_up_in_the_middle_of_an_errand() {
        let pending = PendingNotice::after_installing();

        assert_eq!(pending.mid_errand(), None);
        assert!(pending.is_pending());
    }

    #[test]
    fn the_notice_comes_out_once_when_the_errand_ends() {
        let mut pending = PendingNotice::after_installing();

        assert_eq!(
            pending.when_the_errand_ends(),
            Some(Notice::RestartTheBrowser)
        );
        assert_eq!(pending.when_the_errand_ends(), None);
    }

    #[test]
    fn nothing_installed_means_nothing_to_say() {
        let mut pending = PendingNotice::none();

        assert!(!pending.is_pending());
        assert_eq!(pending.when_the_errand_ends(), None);
    }
}
