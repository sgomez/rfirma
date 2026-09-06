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
