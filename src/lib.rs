extern crate libc;
extern crate librabbitmq_sys as raw_rabbitmq;

mod error;
mod types;

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {
        use super::*;
        let conn = types::connection::Connection::new("localhost", 5672);

        assert!(conn.is_ok());
        let conn = conn.unwrap();
        let login = conn.login("/", 0, 131072, 0, "guest", "guest");
        assert!(login.is_ok());

        let channel = types::channel::Channel::new(&conn, 1);
    }
}
