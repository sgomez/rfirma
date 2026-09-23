use super::*;
use crate::site::domain::channel::ChannelLocation;

fn an_occupied_port() -> (TcpListener, u16) {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .expect("el sistema deberia dar un puerto efimero");
    let port = listener
        .local_addr()
        .expect("un escuchador atado tiene direccion")
        .port();
    (listener, port)
}

fn ipv6_loopback_is_available() -> bool {
    TcpListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, 0))).is_ok()
}

#[test]
fn the_first_free_of_the_drawn_ports_is_the_one_that_is_bound() {
    let (occupied, taken) = an_occupied_port();
    let (free, available) = an_occupied_port();
    drop(free);

    let listener = bind_first_free(&ChannelLocation::Drawn(vec![taken, available]))
        .expect("el segundo estaba libre");

    assert_eq!(
        listener.port().expect("atado"),
        available,
        "se prueban en el orden en que la sede los mando"
    );
    drop(occupied);
}

#[test]
fn the_channel_only_listens_on_the_loopback() {
    let (free, available) = an_occupied_port();
    drop(free);

    let listener = bind_first_free(&ChannelLocation::Drawn(vec![available])).expect("estaba libre");

    let addresses = listener.addresses();
    assert!(
        addresses.iter().all(|address| address.ip().is_loopback()),
        "un canal atado a la comodin estaria abierto a la red local: {addresses:?}"
    );
}

#[test]
fn the_channel_listens_on_both_loopbacks_at_the_same_port() {
    if !ipv6_loopback_is_available() {
        return;
    }
    let (free, available) = an_occupied_port();
    drop(free);

    let listener = bind_first_free(&ChannelLocation::Drawn(vec![available])).expect("estaba libre");

    assert_eq!(
        listener.addresses(),
        vec![
            SocketAddr::from((Ipv4Addr::LOCALHOST, available)),
            SocketAddr::from((Ipv6Addr::LOCALHOST, available)),
        ]
    );
}

#[test]
fn a_port_taken_on_the_ipv6_loopback_is_skipped_like_any_taken_port() {
    if !ipv6_loopback_is_available() {
        return;
    }
    let occupied = TcpListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, 0)))
        .expect("el sistema deberia dar un puerto efimero");
    let taken = occupied.local_addr().expect("atado").port();
    let (free, available) = an_occupied_port();
    drop(free);

    let listener = bind_first_free(&ChannelLocation::Drawn(vec![taken, available]))
        .expect("el segundo estaba libre en los dos bucles locales");

    assert_eq!(listener.port().expect("atado"), available);
    drop(occupied);
}

#[test]
fn with_every_drawn_port_taken_there_is_no_channel() {
    let (occupied, taken) = an_occupied_port();

    let error = bind_first_free(&ChannelLocation::Drawn(vec![taken]))
        .expect_err("el unico puerto estaba ocupado");

    assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
    assert!(error.detail().contains(&taken.to_string()));
    drop(occupied);
}

#[test]
fn the_port_of_the_third_protocol_is_never_bound_when_drawn() {
    let error = bind_first_free(&ChannelLocation::Drawn(vec![
        THE_PORT_OF_THE_THIRD_PROTOCOL,
    ]))
    .expect_err("ese puerto no se ata jamas cuando se sorteo");

    assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
    assert!(
        !error.detail().contains("63117"),
        "no es que estuviera ocupado: es que ni se intenta"
    );
}

#[test]
fn a_launch_without_drawn_ports_binds_nothing() {
    let error =
        bind_first_free(&ChannelLocation::Drawn(vec![])).expect_err("sin puertos no hay canal");

    assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
}

#[test]
fn a_fixed_port_is_bound_even_if_it_is_the_port_of_the_third_protocol() {
    let listener = bind_first_free(&ChannelLocation::Fixed(THE_PORT_OF_THE_THIRD_PROTOCOL))
        .expect("un puerto fijo se ata tal cual, aunque sea el 63117");

    assert_eq!(
        listener.port().expect("atado"),
        THE_PORT_OF_THE_THIRD_PROTOCOL
    );
}

#[test]
fn a_fixed_port_already_taken_is_a_bind_failure() {
    let (occupied, taken) = an_occupied_port();

    let error =
        bind_first_free(&ChannelLocation::Fixed(taken)).expect_err("el puerto fijo estaba ocupado");

    assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
    assert!(error.detail().contains(&taken.to_string()));
    drop(occupied);
}
