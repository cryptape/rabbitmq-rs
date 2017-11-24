use raw_rabbitmq;
use std::mem;
use libc;
use std::ptr;

pub struct BasicProperties {
    pub raw: *mut raw_rabbitmq::amqp_basic_properties_t,
}

impl BasicProperties {
    pub fn new() -> BasicProperties {
        let raw: *mut raw_rabbitmq::amqp_basic_properties_t = unsafe {
            libc::malloc(mem::size_of::<raw_rabbitmq::amqp_basic_properties_t>())
                as *mut raw_rabbitmq::amqp_basic_properties_t
        };

        BasicProperties { raw: raw }
    }

    pub fn null() -> BasicProperties {
        let raw: *mut raw_rabbitmq::amqp_basic_properties_t = ptr::null_mut();

        BasicProperties { raw: raw }
    }
}


impl Drop for BasicProperties {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.raw as *mut _);
        }
    }
}
