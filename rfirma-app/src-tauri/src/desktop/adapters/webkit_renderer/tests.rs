use super::*;

fn environment<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    }
}

#[test]
fn an_x11_session_turns_compositing_off() {
    assert!(compositing_must_be_turned_off(environment(&[(
        "DISPLAY", ":0"
    )])));
}

#[test]
fn a_wayland_session_keeps_compositing() {
    assert!(!compositing_must_be_turned_off(environment(&[
        ("WAYLAND_DISPLAY", "wayland-0"),
        ("DISPLAY", ":0"),
    ])));
}

#[test]
fn a_wayland_session_forced_to_x11_turns_compositing_off() {
    assert!(compositing_must_be_turned_off(environment(&[
        ("WAYLAND_DISPLAY", "wayland-0"),
        ("GDK_BACKEND", "x11"),
    ])));
}

#[test]
fn the_first_backend_of_the_list_is_the_one_that_counts() {
    assert!(!compositing_must_be_turned_off(environment(&[(
        "GDK_BACKEND",
        "wayland,x11"
    )])));
    assert!(compositing_must_be_turned_off(environment(&[
        ("WAYLAND_DISPLAY", "wayland-0"),
        ("GDK_BACKEND", "x11,wayland"),
    ])));
}

#[test]
fn a_wildcard_backend_falls_back_to_the_session() {
    assert!(!compositing_must_be_turned_off(environment(&[
        ("WAYLAND_DISPLAY", "wayland-0"),
        ("GDK_BACKEND", "*"),
    ])));
}

#[test]
fn a_renderer_choice_already_in_the_environment_is_respected() {
    for name in RENDERER_CHOICES {
        assert!(
            !compositing_must_be_turned_off(environment(&[(name, "0")])),
            "{name}"
        );
    }
}
