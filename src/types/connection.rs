use raw_rabbitmq;
use error::Error;
use std::ffi::CString;
use std::time::Duration;
use libc::timeval;
use std::ptr::{self, Shared};
use util::duration_to_timeval;
use std::mem;

#[derive(Clone, Copy)]
pub struct Connection {
    ptr: Shared<raw_rabbitmq::amqp_connection_state_t_>,
}

impl Connection {
    pub fn new(hostname: &str, port: i32, timeout: Option<Duration>) -> Result<Connection, Error> {
        let raw_ptr = unsafe { raw_rabbitmq::amqp_new_connection() };

        let socket = unsafe { raw_rabbitmq::amqp_tcp_socket_new(raw_ptr) };

        if socket.is_null() {
            return Err(Error::TCPSocket);
        }
        // println!("socket {:?}", socket);

        let hostname = CString::new(hostname)?;


        let tv: *mut raw_rabbitmq::timeval = match timeout {
            Some(dur) => &duration_to_timeval(dur) as *const timeval as *mut raw_rabbitmq::timeval,
            None => ptr::null_mut(),
        };

        let status =
            unsafe { raw_rabbitmq::amqp_socket_open_noblock(socket, hostname.as_ptr(), port, tv) };

        // println!("status {:?}", status);

        if status != (raw_rabbitmq::amqp_status_enum__AMQP_STATUS_OK as i32) {
            return Err(Error::Status(status));
        }

        if let Some(ptr) = Shared::new(raw_ptr) {
            Ok(Connection { ptr: ptr })
        } else {
            Err(Error::TCPSocket)
        }
    }

    // pub fn amqp_login(state: amqp_connection_state_t,
    //                 vhost: *const ::std::os::raw::c_char,
    //                 channel_max: ::std::os::raw::c_int,
    //                 frame_max: ::std::os::raw::c_int,
    //                 heartbeat: ::std::os::raw::c_int,
    //                 sasl_method: amqp_sasl_method_enum, ...)
    //  -> amqp_rpc_reply_t;

    pub fn login(
        &self,
        vhost: &str,
        channel_max: i32,
        frame_max: i32,
        heartbeat: i32,
        login: &str,
        password: &str,
    ) -> Result<(), Error> {
        let vhost = CString::new(vhost)?;
        let login = CString::new(login)?;
        let password = CString::new(password)?;

        let login_reply = unsafe {
            raw_rabbitmq::amqp_login(
                self.raw_ptr(),
                vhost.as_ptr(),
                channel_max,
                frame_max,
                heartbeat,
                raw_rabbitmq::amqp_sasl_method_enum__AMQP_SASL_METHOD_PLAIN,
                login.as_ptr(),
                password.as_ptr(),
            )
        };

        match login_reply.reply_type {
            raw_rabbitmq::amqp_response_type_enum__AMQP_RESPONSE_NORMAL => Ok(()),
            _ => Err(Error::Reply),
        }
    }

    pub fn raw_ptr(&self) -> *mut raw_rabbitmq::amqp_connection_state_t_ {
        self.ptr.as_ptr()
    }

    pub fn close(&mut self) {
        unsafe {
            raw_rabbitmq::amqp_connection_close(
                self.ptr.as_ptr(),
                raw_rabbitmq::AMQP_REPLY_SUCCESS as i32,
            );
            raw_rabbitmq::amqp_destroy_connection(self.ptr.as_ptr());
        }
    }
}
