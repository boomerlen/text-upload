// config.rs
//
// Includes configuration file grabbing etc
//
// File created 12/1/24 by HS

use serde::Deserialize;

use std::io::Read;
use std::path::Path;
use std::fs::File;

// Use git2 errors throughout just makes life a lot easier
use git2::Error;

#[derive(Deserialize)]
pub struct Config {
    pub url: String,
    pub local_dir: String,
    pub branch: String,
    pub buffer_dir_rel: String,
    pub ssh_file: String,
}

pub const DFT_CONF_PATH: &str = "conf.toml";

pub fn get_config(conf_file: &Path) -> Result<Config, git2::Error> {
    // Open file
    let mut file = match File::open(conf_file) {
        Ok(f) => f,
        Err(why) => return Err(Error::from_str(format!("Could not open conf file with error: {}.", why.to_string()).as_str())),
    };

    let mut conf_text = String::new();
    match file.read_to_string(&mut conf_text) {
        Ok(_) => (),
        Err(why) => return Err(Error::from_str(format!("Could not read config file with error: {}.", why.to_string()).as_str())),
    }

    // Parse
    let conf: Config = match toml::from_str(conf_text.as_str()) {
        Ok(c) => c,
        Err(why) => panic!("Could not parse file with error: {}", why),
    };

    Ok(conf)
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn test_parse_config() {
        let config: Config = toml::from_str(r#"
        url = 'test-url'
        local_dir = 'test-local-dir'
        branch = 'test-branch'
        buffer_dir_rel = 'test-buffer-dir-rel'
        ssh_file = 'test-ssh-file' 
        "#).unwrap();

        assert_eq!(config.url, "test-url");
        assert_eq!(config.local_dir, "test-local-dir");
        assert_eq!(config.branch, "test-branch");
        assert_eq!(config.buffer_dir_rel, "test-buffer-dir-rel");
        assert_eq!(config.ssh_file, "test-ssh-file");
    }

    #[test]
    fn test_parse_config_file() {
       let config_text = r#"
        url = 'test-url'
        local_dir = 'test-local-dir'
        branch = 'test-branch'
        buffer_dir_rel = 'test-buffer-dir-rel'
        ssh_file = 'test-ssh-file' 
        "#;

        let config_path = Path::new("/tmp/test_config.toml");
        let mut file: File = File::create(&config_path).unwrap();
        
        file.write_all(config_text.as_bytes()).unwrap();
    }
}