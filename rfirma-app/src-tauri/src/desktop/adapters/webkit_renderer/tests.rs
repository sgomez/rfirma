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
    assert_eq!(
        the_switch_for_this_session(environment(&[("DISPLAY", ":0")])),
        Some(COMPOSITING_SWITCH)
    );
}

#[test]
fn a_wayland_session_turns_the_dmabuf_renderer_off() {
    assert_eq!(
        the_switch_for_this_session(environment(&[
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("DISPLAY", ":0"),
        ])),
        Some(DMABUF_SWITCH)
    );
}

#[test]
fn a_wayland_session_forced_to_x11_turns_compositing_off() {
    assert_eq!(
        the_switch_for_this_session(environment(&[
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("GDK_BACKEND", "x11"),
        ])),
        Some(COMPOSITING_SWITCH)
    );
}

#[test]
fn the_first_backend_of_the_list_is_the_one_that_counts() {
    assert_eq!(
        the_switch_for_this_session(environment(&[("GDK_BACKEND", "wayland,x11")])),
        Some(DMABUF_SWITCH)
    );
    assert_eq!(
        the_switch_for_this_session(environment(&[
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("GDK_BACKEND", "x11,wayland"),
        ])),
        Some(COMPOSITING_SWITCH)
    );
}

#[test]
fn a_wildcard_backend_falls_back_to_the_session() {
    assert_eq!(
        the_switch_for_this_session(environment(&[
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("GDK_BACKEND", "*"),
        ])),
        Some(DMABUF_SWITCH)
    );
}

#[test]
fn a_renderer_choice_already_in_the_environment_is_respected() {
    for name in RENDERER_CHOICES {
        assert_eq!(
            the_switch_for_this_session(environment(&[(name, "0")])),
            None,
            "{name}"
        );
    }
}
