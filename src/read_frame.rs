use std::io;
use futures::future::Future;
use futures::stream::Stream;
use futures::{Async, Poll};
use types::connection::Connection;
use raw_rabbitmq::{self, amqp_frame_t, amqp_status_enum_};
use libc;
use error::Error;
use std::mem;
use libc::timeval;

pub struct Frame {
    pub ptr: *mut raw_rabbitmq::amqp_frame_t,
}

impl Default for Frame {
    fn default() -> Frame {
        let frame: *mut amqp_frame_t =
            unsafe { libc::malloc(mem::size_of::<amqp_frame_t>()) as *mut amqp_frame_t };

        Frame { ptr: frame }
    }
}

pub struct ReadFrame {
    conn: *mut raw_rabbitmq::amqp_connection_state_t_,
    timeout: timeval,
    frame: Frame,
}

impl ReadFrame {
    pub fn new(conn: *mut raw_rabbitmq::amqp_connection_state_t_) -> ReadFrame {
        let timeout = timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        ReadFrame {
            conn: conn,
            timeout: timeout,
            frame: Frame::default(),
        }
    }

    pub fn read_frame_noblock(&mut self) -> amqp_status_enum_ {
        let timeout = unsafe { mem::transmute(&self.timeout) };
        unsafe { raw_rabbitmq::amqp_simple_wait_frame_noblock(self.conn, self.frame.ptr, timeout) }
    }
}


impl Future for ReadFrame {
    type Item = Frame;
    type Error = Error;

    fn poll(&mut self) -> Poll<Frame, Error> {
        let status = self.read_frame_noblock();

        match status {
            amqp_status_enum__AMQP_STATUS_OK => {
                let frame = Frame { ptr: self.frame.ptr };
                Ok(Async::Ready(frame))
            },
            amqp_status_enum__AMQP_STATUS_TIMEOUT => Ok(Async::NotReady),
            _ => Err(Error::Frame),
        }
    }
}
