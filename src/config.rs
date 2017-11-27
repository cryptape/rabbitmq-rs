use configlib::{ConfigError, File, FileFormat};
pub use configlib::Config as ConfiBuilder;

// pub type ConfiBuilder = configlib::Config;

static DEFAULT_CONFIG: &str = include_str!("res/default.toml");

#[derive(Debug, Deserialize)]
pub struct Connection {
    pub hostname: String,
    pub port: i32,
}


#[derive(Debug, Deserialize)]
pub struct Login {
    pub vhost: String,
    pub channel_max: i32,
    pub frame_max: i32,
    pub heartbeat: i32,
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub connection: Connection,
    pub login: Login,
}


impl Config {
    pub fn new() -> Result<ConfiBuilder, ConfigError> {
        let mut s = ConfiBuilder::new();
        s.merge(File::from_str(DEFAULT_CONFIG, FileFormat::Toml))?;
        Ok(s)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn basic() {
    //     let config: ConfiBuilder = Config::new().unwrap();
    // }
}
