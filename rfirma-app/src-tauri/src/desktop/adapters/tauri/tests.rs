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
