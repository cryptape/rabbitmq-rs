use configlib::{Config as ConfiBuilder, ConfigError, File, FileFormat};

static DEFAULT_CONFIG: &str = include_str!("res/default.toml");

#[derive(Debug, Deserialize)]
struct Connection {
    hostname: String,
    port: i32,
}


#[derive(Debug, Deserialize)]
struct Login {
    vhost: String,
    channel_max: i32,
    frame_max: i32,
    heartbeat: i32,
    login: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    connection: Connection,
    login: Login,
}


impl Config {
    pub fn new() -> Result<Config, ConfigError> {
        let mut s = ConfiBuilder::new();
        s.merge(File::from_str(DEFAULT_CONFIG, FileFormat::Toml))?;
        s.try_into()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let config: Config = Config::new().unwrap();
    }
}
