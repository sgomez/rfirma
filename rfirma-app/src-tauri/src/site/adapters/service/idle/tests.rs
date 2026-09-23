use std::time::Duration;

use tokio::time::{advance, timeout};

use super::*;

const A_MOMENT: Duration = Duration::from_millis(1);

async fn has_expired_after(clock: &IdleClock, wait: Duration) -> bool {
    timeout(wait, clock.expired()).await.is_ok()
}

#[tokio::test(start_paused = true)]
async fn a_clock_without_orders_expires_when_its_timeout_passes() {
    let clock = IdleClock::started(SOCKET_TIMEOUT);

    assert!(!has_expired_after(&clock, SOCKET_TIMEOUT - A_MOMENT).await);
    assert!(has_expired_after(&clock, A_MOMENT * 2).await);
}

#[tokio::test(start_paused = true)]
async fn an_order_in_flight_holds_the_clock_however_long_it_takes() {
    let clock = IdleClock::started(SOCKET_TIMEOUT);
    let in_flight = clock.order_arrived();

    assert!(!has_expired_after(&clock, SOCKET_TIMEOUT * 5).await);

    drop(in_flight);
    assert!(!has_expired_after(&clock, SOCKET_TIMEOUT - A_MOMENT).await);
    assert!(has_expired_after(&clock, A_MOMENT * 2).await);
}

#[tokio::test(start_paused = true)]
async fn an_answered_order_restarts_the_count_from_its_answer() {
    let clock = IdleClock::started(SOCKET_TIMEOUT);
    advance(Duration::from_secs(80)).await;

    drop(clock.order_arrived());

    assert!(!has_expired_after(&clock, Duration::from_secs(80)).await);
    assert!(has_expired_after(&clock, Duration::from_secs(11)).await);
}
