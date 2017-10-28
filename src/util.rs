use libc::timeval;
use std::time::Duration;


pub fn duration_to_timeval(t: Duration) -> timeval {
    timeval {
        tv_sec: t.as_secs() as i64,
        tv_usec: (t.subsec_nanos() / 1_000_000) as i64,
    }
}
