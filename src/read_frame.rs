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

type AmqpFrame = *mut raw_rabbitmq::amqp_frame_t;

pub struct ReadFrame {
    conn: *mut raw_rabbitmq::amqp_connection_state_t_,
    timeout: timeval,
    frame: AmqpFrame,
}

impl ReadFrame {
    pub fn new(conn: *mut raw_rabbitmq::amqp_connection_state_t_, frame: AmqpFrame) -> ReadFrame {
        let timeout = timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        ReadFrame {
            conn: conn,
            timeout: timeout,
            frame: frame,
        }
    }

    pub fn read_frame_noblock(&mut self) -> amqp_status_enum_ {
        let timeout = unsafe { mem::transmute(&self.timeout) };
        unsafe { raw_rabbitmq::amqp_simple_wait_frame_noblock(self.conn, self.frame, timeout) }
    }
}


impl Future for ReadFrame {
    type Item = AmqpFrame;
    type Error = Error;

    fn poll(&mut self) -> Poll<AmqpFrame, Error> {
        let status = self.read_frame_noblock();

        match status {
            amqp_status_enum__AMQP_STATUS_OK => {
                Ok(Async::Ready(self.frame))
            },
            amqp_status_enum__AMQP_STATUS_TIMEOUT => Ok(Async::NotReady),
            _ => Err(Error::Frame),
        }
    }
}
