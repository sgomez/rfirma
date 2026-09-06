use super::*;

#[test]
fn what_is_answered_is_what_the_other_end_receives() {
    let (sender, mut receiver) = oneshot::channel();

    ReplyHandle::of(sender).answer("OK".to_owned());

    assert_eq!(receiver.try_recv(), Ok("OK".to_owned()));
}

#[test]
fn answering_a_connection_that_is_gone_is_not_a_failure() {
    let (sender, receiver) = oneshot::channel();
    drop(receiver);

    ReplyHandle::of(sender).answer("CANCEL".to_owned());
}
