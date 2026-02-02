use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use structopt::StructOpt;
#[cfg(feature = "experimental-aec3-config")]
use webrtc_audio_processing::experimental;
use webrtc_audio_processing_config::Config;

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
    pub fn multichannel_default() -> Self {
        Self {
            num_capture_channels: 2,
            num_render_channels: 2,
            config: Config::default(),
            #[cfg(feature = "experimental-aec3-config")]
            aec3: experimental::EchoCanceller3Config::multichannel_default(),
        }
    }

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
    enum Args {
        ReadConfigFile { config_file: PathBuf },
        DefaultConfig,
        DefaultMultichannelConfig,
    }
    let args = Args::from_args();

    let config = match args {
        Args::ReadConfigFile { config_file } => AppConfig::load(Some(config_file))?,
        Args::DefaultConfig => AppConfig::default(),
        Args::DefaultMultichannelConfig => AppConfig::multichannel_default(),
    };
    println!("{}", serde_json::to_string_pretty(&config)?);

    Ok(())
}

#[cfg(all(test, feature = "experimental-aec3-config"))]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_matches_file() {
        test_config_matches_file("examples/aec-configs/defaults.json5", AppConfig::default());
    }

    #[test]
    fn test_multichannel_default_config_matches_file() {
        test_config_matches_file(
            "examples/aec-configs/multichannel-defaults.json5",
            AppConfig::multichannel_default(),
        );
    }

    fn test_config_matches_file(filepath: &str, config: AppConfig) {
        let file_path = PathBuf::from(filepath);
        let file_contents = fs::read_to_string(&file_path)
            .unwrap_or_else(|e| panic!("Failed to load {:?}: {:#}.", file_path, e));

        let json = serde_json::to_string_pretty(&config).unwrap();

        assert_eq!(
            file_contents.trim(), json.trim(),
            "The passed config does not match {filepath}.\n\
             Update the file by running: cargo run --example aec_config --features serde,experimental-aec3-config -- default-* > {filepath}"
        );
    }
}
