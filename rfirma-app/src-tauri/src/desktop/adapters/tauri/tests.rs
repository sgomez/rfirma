use super::*;

#[test]
fn the_nss_classes_keep_their_browser_brand() {
    assert_eq!(brand_of_class(StoreClass::Firefox), StoreBrand::Firefox);
    assert_eq!(brand_of_class(StoreClass::Chrome), StoreBrand::Chrome);
    assert_eq!(brand_of_class(StoreClass::Nssdb), StoreBrand::Nssdb);
}

#[test]
fn a_card_and_an_installed_p12_are_each_their_own_brand() {
    assert_eq!(brand_of_class(StoreClass::Card), StoreBrand::Card);
    assert_eq!(brand_of_class(StoreClass::Installed), StoreBrand::Installed);
}

#[test]
fn the_windows_personal_store_is_the_windows_brand() {
    assert_eq!(brand_of_class(StoreClass::Windows), StoreBrand::Windows);
}

#[test]
fn every_installation_outcome_crosses_as_its_own_view() {
    use crate::desktop::domain::installation::Installation;

    let crossed = [
        Installation::Installed,
        Installation::NoUpdate,
        Installation::NetworkFailure,
        Installation::InvalidSignature,
        Installation::NotAvailable,
    ]
    .map(InstallationView::from);

    assert_eq!(
        crossed,
        [
            InstallationView::Installed,
            InstallationView::NoUpdate,
            InstallationView::NetworkFailure,
            InstallationView::InvalidSignature,
            InstallationView::NotAvailable,
        ]
    );
}
