//! Pruebas del certificado pegajoso de una seleccion.


use crate::site::application::errand::*;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::ports::CertificateMemory;
use crate::signing::application::tests::a_memory;
use super::support::*;

#[test]
fn a_sticky_selection_opens_the_window_with_the_desk_remembered_row_preselected_and_answers_nothing(
) {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[0].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);

    let engine = AnEngine::answering(&[&[0]]);
    let request = requested(&an_operation("&sticky=true"));
    let step = consent_for(
        &engine,
        &request,
        ours.clone(),
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    let ErrandStep::AskingForConsent {
        certificates: rows,
        sticky,
        ..
    } = step
    else {
        panic!("sin ventana no hay certificado: {step:?}");
    };
    assert!(sticky);
    assert!(
        rows[0].remembered,
        "la fila recordada llega preseleccionada"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "la sede no recibe nada hasta que la persona consiente"
    );
}

#[test]
fn a_sticky_selection_sticks_the_chosen_one_to_its_session_and_not_to_the_desk() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA"), a_usable_certificate("OTRO")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[0].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();

    let engine = AnEngine::answering(&[&[0, 1], &[0], &[0, 1]]);
    let request = requested(&an_operation("&sticky=true"));
    let neighbours = a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory);
    let ErrandStep::AskingForConsent {
        certificates: rows,
        sticky,
        ..
    } = consent_for(&engine, &request, ours.clone(), &neighbours, &live)
    else {
        panic!("'sticky' pregunta siempre");
    };

    let reply = identity_handed_over(
        &engine,
        request.filter(),
        sticky,
        &ours,
        &rows[1].id,
        &neighbours,
        &live,
    );
    assert!(matches!(reply, SiteOutcome::Certificate(_)));
    assert!(
        memory
            .remembered_certificate()
            .is_some_and(|one| one.is_the_same_as(ours[0].reference())),
        "la sede no escribe en la memoria del escritorio"
    );

    let ErrandStep::AskingForConsent {
        certificates: again,
        ..
    } = consent_for(&engine, &request, ours.clone(), &neighbours, &live)
    else {
        panic!("'sticky' tampoco contesta con el fijado en la sesion");
    };
    let preselected: Vec<bool> = again.iter().map(|row| row.remembered).collect();
    assert_eq!(
        preselected,
        vec![false, true],
        "el fijado en la sesion manda sobre el recordado del escritorio"
    );
}

#[test]
fn two_site_sessions_do_not_share_the_sticky_certificate() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA"), a_usable_certificate("OTRO")];
    let (listed, _) = listed_from(&ours);
    let first = a_live();
    let second = a_live();

    let engine = AnEngine::answering(&[&[0, 1], &[0], &[0, 1]]);
    let request = requested(&an_operation("&sticky=true"));
    let neighbours = a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory);
    let ErrandStep::AskingForConsent {
        certificates: rows,
        sticky,
        ..
    } = consent_for(&engine, &request, ours.clone(), &neighbours, &first)
    else {
        panic!("'sticky' pregunta siempre");
    };
    identity_handed_over(
        &engine,
        request.filter(),
        sticky,
        &ours,
        &rows[1].id,
        &neighbours,
        &first,
    );
    assert!(
        first.the_stuck().is_some(),
        "la primera sesion fijo el suyo"
    );

    let ErrandStep::AskingForConsent {
        certificates: elsewhere,
        ..
    } = consent_for(&engine, &request, ours.clone(), &neighbours, &second)
    else {
        panic!("'sticky' pregunta siempre");
    };
    assert!(
        elsewhere.iter().all(|row| !row.remembered),
        "otra sesion no ve el certificado que fijo la primera"
    );
    assert_eq!(second.the_stuck(), None);
}

#[test]
fn resetsticky_forgets_the_session_one_and_leaves_the_desk_remembered_certificate_alone() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[0].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();
    live.stick(ours[0].reference());

    let engine = AnEngine::answering(&[&[0]]);
    let request = requested(&an_operation("&sticky=true&resetsticky=true"));
    let step = consent_for(
        &engine,
        &request,
        ours.clone(),
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    assert!(
        matches!(step, ErrandStep::AskingForConsent { .. }),
        "se pregunta: {step:?}"
    );
    assert_eq!(
        live.the_stuck(),
        None,
        "'resetsticky' olvida el de su sesion"
    );
    assert!(
        memory
            .remembered_certificate()
            .is_some_and(|one| one.is_the_same_as(ours[0].reference())),
        "el ultimo certificado usado del escritorio sigue donde estaba"
    );
}

#[test]
fn without_sticky_the_remembered_certificate_changes_nothing() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[0].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();

    let engine = AnEngine::answering(&[&[0]]);
    let request = requested(&an_operation(""));
    let neighbours = a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory);
    let step = consent_for(&engine, &request, ours.clone(), &neighbours, &live);

    let ErrandStep::AskingForConsent {
        certificates: rows,
        sticky,
        ..
    } = step
    else {
        panic!("sin 'sticky' se pregunta siempre");
    };
    assert!(!sticky, "sin 'sticky' no hay nada que recordar");
    assert!(
        memory.remembered_certificate().is_some(),
        "el recordado sigue donde estaba"
    );

    let reply = identity_handed_over(
        &AnEngine::answering(&[&[0]]),
        request.filter(),
        sticky,
        &ours,
        &rows[0].id,
        &neighbours,
        &live,
    );
    assert!(matches!(reply, SiteOutcome::Certificate(_)));
    assert_eq!(live.the_stuck(), None, "sin 'sticky' no se fija nada");
}

