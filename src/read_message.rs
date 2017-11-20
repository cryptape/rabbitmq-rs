use std::io;
use futures::future::Future;
use futures::stream::Stream;
use futures::{Async, Poll, task};
use types::connection::Connection;
use raw_rabbitmq::{self, amqp_frame_t, amqp_status_enum_};
use libc;
use error::Error;
use std::mem;
use libc::timeval;
use util::decode_raw_bytes;
use bytes::{BufMut, BytesMut};

const AMQP_BASIC_DELIVER_METHOD: u32 = 0x003C003C;

type AmqpFrame = *mut raw_rabbitmq::amqp_frame_t;

pub struct ReadMessage {
    conn: *mut raw_rabbitmq::amqp_connection_state_t_,
    method_delivered: bool,
    body_target: Option<usize>,
    body_received: usize,
    buf: BytesMut,
    timeout: timeval,
    frame: AmqpFrame,
}

impl ReadMessage {
    pub fn new(conn: *mut raw_rabbitmq::amqp_connection_state_t_) -> ReadMessage {
        let timeout = timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let frame: *mut raw_rabbitmq::amqp_frame_t = unsafe {
            libc::malloc(mem::size_of::<raw_rabbitmq::amqp_frame_t>())
                as *mut raw_rabbitmq::amqp_frame_t
        };
        let buf = BytesMut::new();

        ReadMessage {
            conn: conn,
            method_delivered: false,
            body_received: 0,
            body_target: None,
            buf: buf,
            timeout: timeout,
            frame: frame,
        }
    }

    pub fn read_frame_noblock(&mut self) -> amqp_status_enum_ {
        let timeout = unsafe { mem::transmute(&self.timeout) };
        unsafe { raw_rabbitmq::amqp_simple_wait_frame_noblock(self.conn, self.frame, timeout) }
    }
}

impl Drop for ReadMessage {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.frame as *mut _);
        }
    }
}


impl Future for ReadMessage {
    type Item = BytesMut;
    type Error = Error;

    fn poll(&mut self) -> Poll<BytesMut, Error> {
        let status = self.read_frame_noblock();
        // println!("poll!");

        let status = match status {
            amqp_status_enum__AMQP_STATUS_OK => unsafe {
                if !self.method_delivered {
                    if (*self.frame).frame_type == (raw_rabbitmq::AMQP_FRAME_METHOD as u8)
                        && (*self.frame).payload.method.id == (AMQP_BASIC_DELIVER_METHOD)
                    {
                        self.method_delivered = true;
                    }
                    Ok(Async::NotReady)
                } else {
                    if let Some(body_target) = self.body_target {
                        if (*self.frame).frame_type != (raw_rabbitmq::AMQP_FRAME_BODY as u8) {
                            println!("Unexpected body!");
                            return Err(Error::Frame);
                        }
                        self.body_received += (*self.frame).payload.body_fragment.len;
                        let body_fragment = decode_raw_bytes((*self.frame).payload.body_fragment);
                        self.buf.put(body_fragment);
                        if self.body_received < body_target {
                            Ok(Async::NotReady)
                        } else {
                            Ok(Async::Ready(self.buf.take()))
                        }
                    } else {
                        if (*self.frame).frame_type == (raw_rabbitmq::AMQP_FRAME_HEADER as u8) {
                            self.body_target =
                                Some((*self.frame).payload.properties.body_size as usize);
                        } else {
                            println!("Unexpected header!");
                            return Err(Error::Frame);
                        }
                        Ok(Async::NotReady)
                    }
                }
            },
            amqp_status_enum__AMQP_STATUS_TIMEOUT => Ok(Async::NotReady),
            _ => Err(Error::Frame),
        };

        if status == Ok(Async::NotReady) {
            task::current().notify();
        }
        status
    }
}
