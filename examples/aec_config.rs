use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use structopt::StructOpt;
use webrtc_audio_processing::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub num_capture_channels: usize,
    pub num_render_channels: usize,
    pub config: Config,
    #[cfg(feature = "experimental-aec3-config")]
    pub aec3: experimental::EchoCanceller3Config,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            num_capture_channels: 1,
            num_render_channels: 1,
            config: Config::default(),
            #[cfg(feature = "experimental-aec3-config")]
            aec3: experimental::EchoCanceller3Config::default(),
        }
    }
}

impl AppConfig {
    pub fn load(path: Option<PathBuf>) -> Result<Self, Error> {
        match path {
            Some(path) => {
                let content = fs::read_to_string(path)?;
                let value: serde_json::Value = json5::from_str(&content)?;

                // Use serde_ignored to warn about extra fields.
                let config: Self = serde_ignored::deserialize(value, |path| {
                    eprintln!("Warning: unused configuration field: {}", path);
                })?;

                Ok(config)
            },
            None => Ok(Self::default()),
        }
    }

    pub fn dump() -> Result<(), Error> {
        println!("{}", serde_json::to_string_pretty(&Self::default())?);
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    #[derive(Debug, StructOpt)]
    struct Args {
        #[structopt(short, long)]
        config_file: Option<PathBuf>,
    }
    let args = Args::from_args();

    let config = AppConfig::load(args.config_file)?;
    println!("{}", serde_json::to_string_pretty(&config)?);

    Ok(())
}

#[cfg(all(test, feature = "experimental-aec3-config"))]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_matches_file() {
        let file_path = PathBuf::from("examples/aec-configs/defaults.json5");
        let file_contents = fs::read_to_string(file_path).expect("Failed to load defaults.json5");

        let default_config = AppConfig::default();
        let default_json = serde_json::to_string_pretty(&default_config).unwrap();

        assert_eq!(
            file_contents, default_json,
            "The library's default config does not match examples/aec-configs/defaults.json5.\n\
             Update the file by running: cargo run --example aec_config --features \"derive_serde experimental-aec3-config\" > examples/aec-configs/defaults.json5"
        );
    }
}
