use std::path::Path;
use std::sync::Mutex;

use super::UnavailableReaderWatch;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::readers::{follow_the_readers, CardListing, LastListing};
use crate::identity::application::tests::{NoMemory, NoToken};
use crate::identity::domain::store::Store;

#[test]
fn with_no_watcher_nothing_is_announced_and_the_listing_stays_as_it_was() {
    let (listed, installed_copies, last) = (
        ListedCertificates::new(),
        ListedCertificates::new(),
        LastListing::default(),
    );
    let handle = listed.mint(
        crate::identity::application::tests::a_certificate("FIRMA", &[])
            .reference()
            .clone(),
    );
    let heard = Mutex::new(0);

    follow_the_readers(
        &mut UnavailableReaderWatch,
        &CardListing {
            token: &NoToken,
            stores: vec![Store::module("/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so")],
            installed_dir: Path::new("/casa/ada/certificados"),
            listed: &listed,
            installed_copies: &installed_copies,
            memory: &NoMemory,
            last: &last,
        },
        &|_| *heard.lock().unwrap() += 1,
    );

    assert_eq!(*heard.lock().unwrap(), 0);
    assert!(listed.get(&handle).is_some());
}
