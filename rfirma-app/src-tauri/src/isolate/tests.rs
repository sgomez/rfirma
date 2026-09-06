use super::Isolate;
use crate::ffi::BridgeError;

fn a_failing_isolate() -> Isolate {
    Isolate::start_with(|| Err(BridgeError::Failed("no hay librería".to_owned())))
}

#[test]
fn a_failure_to_open_is_told_to_every_caller_and_not_only_to_the_first() {
    let isolate = a_failing_isolate();

    for _ in 0..3 {
        let answer = isolate.run(|_| ()).expect("el hilo sigue vivo");
        assert!(answer.is_err(), "la librería sigue sin estar");
    }
}

#[test]
fn the_library_is_opened_lazily_and_only_once() {
    let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&attempts);
    let isolate = Isolate::start_with(move || {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(BridgeError::Failed("no hay librería".to_owned()))
    });

    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 0);
    for _ in 0..3 {
        let _ = isolate.run(|_| ());
    }
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn the_handle_can_be_shared_between_threads() {
    let isolate = a_failing_isolate();
    let elsewhere = isolate.clone();

    let joined = std::thread::spawn(move || elsewhere.run(|_| ()).is_ok())
        .join()
        .expect("el hilo de prueba no entra en pánico");

    assert!(joined);
}
