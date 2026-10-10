use super::*;

#[test]
fn the_repair_only_waits_when_a_channel_is_serving() {
    assert_eq!(what_the_repair_leaves(true, true), Moment::Waiting);
    assert_eq!(
        what_the_repair_leaves(true, false),
        Moment::NoChannel(NoChannel::ChannelNotOpened)
    );
}

#[test]
fn the_repair_asks_for_the_local_ca_again_when_it_reached_no_store() {
    for serving in [true, false] {
        assert_eq!(
            what_the_repair_leaves(false, serving),
            Moment::NoChannel(NoChannel::LocalCaMissing)
        );
    }
}

mod while_a_channel_serves {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use super::*;
    use crate::site::application::tests::InMemoryCaSlots;
    use crate::site::domain::channel::{OpenChannel, Shutdown};
    use crate::site::domain::local_ca::{has_the_local_ca_subject, LocalCa};
    use crate::site::domain::trust_error::TrustError;

    const TRUSTED_SSL_CA: u32 = 0x38;

    #[derive(Default)]
    struct Stores {
        trusted: Mutex<Vec<(PathBuf, Vec<u8>)>>,
    }

    impl TrustStores for Stores {
        fn install(&self, profile: &Path, der: &[u8], _nickname: &str) -> Result<(), TrustError> {
            crate::lock(&self.trusted).push((profile.to_path_buf(), der.to_vec()));
            Ok(())
        }

        fn trust_of(&self, profile: &Path, der: &[u8]) -> Result<Option<u32>, TrustError> {
            Ok(crate::lock(&self.trusted)
                .iter()
                .any(|(where_, inside)| where_ == profile && inside == der)
                .then_some(TRUSTED_SSL_CA))
        }

        fn withdraw(&self, profile: &Path, der: &[u8]) -> Result<(), TrustError> {
            crate::lock(&self.trusted)
                .retain(|(where_, inside)| !(where_ == profile && inside == der));
            Ok(())
        }

        fn local_cas(&self, profile: &Path) -> Result<Vec<Vec<u8>>, TrustError> {
            Ok(crate::lock(&self.trusted)
                .iter()
                .filter(|(where_, der)| where_ == profile && has_the_local_ca_subject(der))
                .map(|(_, der)| der.clone())
                .collect())
        }
    }

    fn der_of(ca: &LocalCa) -> Vec<u8> {
        ca.certificate()
            .to_der()
            .expect("la CA de pruebas sale en DER")
    }

    fn an_unmarked_local_ca_trusted_only_in_the_first_profile() -> (LocalCaTrust, Vec<u8>) {
        let unmarked = LocalCa::unmarked_for_test().expect("deberia fabricarse");
        let profiles = vec![
            PathBuf::from("/perfiles/chrome"),
            PathBuf::from("/perfiles/firefox"),
        ];
        let stores = Stores::default();
        stores
            .install(&profiles[0], &der_of(&unmarked), "rFirma CA local")
            .expect("el doble admite escritura");
        let store = InMemoryCaSlots::default();
        store
            .write_serving(&unmarked)
            .expect("la ranura de pruebas admite escritura");
        (
            LocalCaTrust {
                store: Box::new(store),
                profiles,
                stores: Box::new(stores),
                mark: ChannelMark::Flatpak,
            },
            der_of(&unmarked),
        )
    }

    fn trusted_in(trust: &LocalCaTrust, profile: &Path, der: &[u8]) -> bool {
        trust
            .stores
            .trust_of(profile, der)
            .expect("el doble se lee")
            .is_some()
    }

    #[test]
    fn the_repair_installs_the_unmarked_local_ca_that_signed_the_serving_channel() {
        let (trust, signer) = an_unmarked_local_ca_trusted_only_in_the_first_profile();
        let held = HeldChannel::default();
        held.hold(OpenChannel::new(51_101, Shutdown::of(|| {})));

        let moment = repair_the_local_ca(&trust, &held, &LiveErrand::default());

        assert_eq!(moment, Moment::Waiting);
        let serving = trust
            .store
            .serving()
            .expect("se lee")
            .expect("sigue habiendo una vigente");
        assert!(
            der_of(&serving) == signer,
            "la CA que firma el canal sigue vigente"
        );
        for profile in &trust.profiles {
            assert!(
                trusted_in(&trust, profile, &signer),
                "{}",
                profile.display()
            );
        }
    }

    #[test]
    fn without_a_serving_channel_the_repair_replaces_the_unmarked_local_ca() {
        let (trust, unmarked) = an_unmarked_local_ca_trusted_only_in_the_first_profile();

        repair_the_local_ca(&trust, &HeldChannel::default(), &LiveErrand::default());

        let serving = trust
            .store
            .serving()
            .expect("se lee")
            .expect("hay una vigente");
        assert_eq!(serving.mark(), Some(ChannelMark::Flatpak));
        assert!(!trusted_in(&trust, &trust.profiles[0], &unmarked));
    }
}
