use std::io;
use futures::future::Future;
use futures::stream::Stream;
use futures::{task, Async, Poll};
use types::connection::Connection;
use raw_rabbitmq::{self, amqp_frame_t, amqp_status_enum_};
use libc;
use error::Error;
use std::mem;
use libc::timeval;
use util::decode_raw_bytes;
use bytes::{BufMut, Bytes, BytesMut};

const AMQP_BASIC_DELIVER_METHOD: u32 = 0x003C003C;

type AmqpFrame = *mut raw_rabbitmq::amqp_frame_t;

pub struct RpcMessageFuture {
    conn: *mut raw_rabbitmq::amqp_connection_state_t_,
    polled: bool,
    method_delivered: bool,
    body_target: Option<usize>,
    body_received: usize,
    buf: BytesMut,
    timeout: timeval,
    frame: AmqpFrame,
    channel_id: u16,
    correlation_id: Bytes,
    reply_queue: raw_rabbitmq::amqp_bytes_t,
}

impl RpcMessageFuture {
    pub fn new(
        conn: *mut raw_rabbitmq::amqp_connection_state_t_,
        channel_id: u16,
        correlation_id: raw_rabbitmq::amqp_bytes_t,
        reply_queue: raw_rabbitmq::amqp_bytes_t,
    ) -> RpcMessageFuture {
        let timeout = timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let frame: *mut raw_rabbitmq::amqp_frame_t = unsafe {
            libc::malloc(mem::size_of::<raw_rabbitmq::amqp_frame_t>())
                as *mut raw_rabbitmq::amqp_frame_t
        };
        let buf = BytesMut::with_capacity(4096);

        RpcMessageFuture {
            conn: conn,
            polled: false,
            method_delivered: false,
            body_received: 0,
            body_target: None,
            buf: buf,
            timeout: timeout,
            frame: frame,
            channel_id: channel_id,
            reply_queue: reply_queue,
            correlation_id: decode_raw_bytes(correlation_id),
        }
    }

    pub fn read_frame_noblock(&mut self) -> amqp_status_enum_ {
        let timeout = &self.timeout as *const libc::timeval as *mut raw_rabbitmq::timeval;
        unsafe { raw_rabbitmq::amqp_simple_wait_frame_noblock(self.conn, self.frame, timeout) }
    }
}

impl Drop for RpcMessageFuture {
    fn drop(&mut self) {
        unsafe {
            raw_rabbitmq::amqp_bytes_free(self.reply_queue);
            libc::free(self.frame as *mut _);
        }
    }
}


impl Future for RpcMessageFuture {
    type Item = BytesMut;
    type Error = Error;

    fn poll(&mut self) -> Poll<BytesMut, Error> {
        let status = self.read_frame_noblock();
        // println!("poll!");

        if !self.polled {
            unsafe {
                raw_rabbitmq::amqp_maybe_release_buffers(self.conn);
                raw_rabbitmq::amqp_basic_consume(
                    self.conn,
                    self.channel_id,
                    self.reply_queue,
                    raw_rabbitmq::amqp_empty_bytes,
                    0,
                    1,
                    0,
                    raw_rabbitmq::amqp_empty_table,
                );
            }
            self.polled = true;
        }

        let status = match status {
            raw_rabbitmq::amqp_status_enum__AMQP_STATUS_OK => unsafe {
                if !self.method_delivered {
                    if (*self.frame).frame_type == (raw_rabbitmq::AMQP_FRAME_METHOD as u8)
                        && (*self.frame).payload.method.id == (AMQP_BASIC_DELIVER_METHOD)
                    {
                        self.method_delivered = true;
                    }
                    Ok(Async::NotReady)
                } else if let Some(body_target) = self.body_target {
                    if (*self.frame).frame_type != (raw_rabbitmq::AMQP_FRAME_BODY as u8) {
                        println!("Unexpected body!");
                        return Err(Error::Frame);
                    }
                    let props: *mut raw_rabbitmq::amqp_basic_properties_t =
                        mem::transmute((*self.frame).payload.properties.decoded);

                    let correlation_id = decode_raw_bytes((*props).correlation_id);
                    // println!("BODY props id {:?}", correlation_id);

                    if correlation_id != self.correlation_id {
                        Ok(Async::NotReady)
                    } else {
                        self.body_received += (*self.frame).payload.body_fragment.len;
                        let body_fragment = decode_raw_bytes((*self.frame).payload.body_fragment);
                        self.buf.put(body_fragment);
                        if self.body_received < body_target {
                            Ok(Async::NotReady)
                        } else {
                            Ok(Async::Ready(self.buf.take()))
                        }
                    }
                } else {
                    if (*self.frame).frame_type == (raw_rabbitmq::AMQP_FRAME_HEADER as u8) {
                        let props: *mut raw_rabbitmq::amqp_basic_properties_t
                            = mem::transmute((*self.frame).payload.properties.decoded);

                        let correlation_id = decode_raw_bytes((*props).correlation_id);
                        // println!("HEADER props id {:?}", correlation_id);

                        if correlation_id == self.correlation_id {
                            self.body_target =
                                Some((*self.frame).payload.properties.body_size as usize);
                        }
                    } else {
                        println!("Unexpected header!");
                        return Err(Error::Frame);
                    }
                    Ok(Async::NotReady)
                }
            },
            raw_rabbitmq::amqp_status_enum__AMQP_STATUS_TIMEOUT => Ok(Async::NotReady),
            _ => Err(Error::Frame),
        };

        if status == Ok(Async::NotReady) {
            task::current().notify();
        }
        status
    }
}
