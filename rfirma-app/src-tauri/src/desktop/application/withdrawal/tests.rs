use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

fn a_profile(brand: StoreBrand) -> (PathBuf, StoreBrand) {
    (PathBuf::from(format!("/profiles/{brand:?}")), brand)
}

fn a_report(handler: Withdrawal, stores: Vec<StoreWithdrawal>) -> WithdrawalReport {
    WithdrawalReport { handler, stores }
}

#[test]
fn the_first_attempt_retries_the_handler_and_every_profile() {
    assert!(handler_needs_retry(None));

    let profiles = vec![
        a_profile(StoreBrand::Firefox),
        a_profile(StoreBrand::Chrome),
    ];
    assert_eq!(
        profiles_to_retry(&profiles, None),
        vec![profiles[0].0.clone(), profiles[1].0.clone()]
    );
}

#[test]
fn a_failed_handler_is_retried_but_a_withdrawn_one_is_not() {
    let failed = a_report(Withdrawal::Failed("perfil en uso".to_owned()), Vec::new());
    let withdrawn = a_report(Withdrawal::Withdrawn, Vec::new());

    assert!(handler_needs_retry(Some(&failed)));
    assert!(!handler_needs_retry(Some(&withdrawn)));
}

#[test]
fn retrying_only_touches_the_stores_that_failed() {
    let firefox = a_profile(StoreBrand::Firefox);
    let chrome = a_profile(StoreBrand::Chrome);
    let previous = a_report(
        Withdrawal::Withdrawn,
        vec![
            StoreWithdrawal {
                brand: StoreBrand::Firefox,
                outcome: Withdrawal::Withdrawn,
            },
            StoreWithdrawal {
                brand: StoreBrand::Chrome,
                outcome: Withdrawal::Failed("perfil en uso".to_owned()),
            },
        ],
    );

    let retry = profiles_to_retry(&[firefox.clone(), chrome.clone()], Some(&previous));

    assert_eq!(retry, vec![chrome.0.clone()]);
}

#[test]
fn same_brand_profiles_are_correlated_by_position_not_by_brand() {
    let working = (PathBuf::from("/profiles/nssdb-working"), StoreBrand::Nssdb);
    let failing = (PathBuf::from("/profiles/nssdb-failing"), StoreBrand::Nssdb);
    let previous = a_report(
        Withdrawal::Withdrawn,
        vec![
            StoreWithdrawal {
                brand: StoreBrand::Nssdb,
                outcome: Withdrawal::Withdrawn,
            },
            StoreWithdrawal {
                brand: StoreBrand::Nssdb,
                outcome: Withdrawal::Failed("perfil en uso".to_owned()),
            },
        ],
    );

    let retry = profiles_to_retry(&[working.clone(), failing.clone()], Some(&previous));
    assert_eq!(retry, vec![failing.0.clone()]);

    let retried = HashMap::from([(failing.0.clone(), Withdrawal::Withdrawn)]);
    let report = merged_report(
        Withdrawal::Withdrawn,
        &[working.clone(), failing.clone()],
        retried,
        Some(&previous),
    );

    assert_eq!(
        report,
        a_report(
            Withdrawal::Withdrawn,
            vec![
                StoreWithdrawal {
                    brand: StoreBrand::Nssdb,
                    outcome: Withdrawal::Withdrawn,
                },
                StoreWithdrawal {
                    brand: StoreBrand::Nssdb,
                    outcome: Withdrawal::Withdrawn,
                },
            ]
        )
    );
}

#[test]
fn merging_keeps_what_was_not_retried_and_replaces_what_was() {
    let firefox = a_profile(StoreBrand::Firefox);
    let chrome = a_profile(StoreBrand::Chrome);
    let previous = a_report(
        Withdrawal::Withdrawn,
        vec![
            StoreWithdrawal {
                brand: StoreBrand::Firefox,
                outcome: Withdrawal::Withdrawn,
            },
            StoreWithdrawal {
                brand: StoreBrand::Chrome,
                outcome: Withdrawal::Failed("perfil en uso".to_owned()),
            },
        ],
    );
    let retried = HashMap::from([(chrome.0.clone(), Withdrawal::Withdrawn)]);

    let report = merged_report(
        Withdrawal::Withdrawn,
        &[firefox.clone(), chrome.clone()],
        retried,
        Some(&previous),
    );

    assert_eq!(
        report,
        a_report(
            Withdrawal::Withdrawn,
            vec![
                StoreWithdrawal {
                    brand: StoreBrand::Firefox,
                    outcome: Withdrawal::Withdrawn,
                },
                StoreWithdrawal {
                    brand: StoreBrand::Chrome,
                    outcome: Withdrawal::Withdrawn,
                },
            ]
        )
    );
}
