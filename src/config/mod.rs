
use config_file::FromConfigFile;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
	pub api: Api,
	pub bot: Bot,
    pub redis: Redis,
	pub ai: Ai,
}

#[derive(Deserialize, Clone)]
pub struct Api {
    pub url: String,
	pub access_token: String,
}

#[derive(Deserialize, Clone)]
pub struct Bot{
	pub owner: u64,
}

#[derive(Deserialize, Clone)]
pub struct Redis {
    pub url: String,
}

#[derive(Deserialize, Clone)]
pub struct Ai {
    pub token: String,
    pub endpoint: String,
    pub default_model: String,
	pub init_prompt: String,
	pub engage_time: i64,
	pub auto_join: bool,
}


pub fn init_config_from_file(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    match Config::from_config_file(path){
		Ok(config) => Ok(config),
		Err(e) => Err(Box::new(e)),
	}
}




pub async fn init_config() -> Config {
	match init_config_from_file("config/config.toml") {
		Ok(config) => config,
		Err(e) => {
			panic!("Failed to load config from file: {}", e);
		},
	}
}