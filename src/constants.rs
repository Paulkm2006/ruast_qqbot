use lazy_static::lazy_static;
use std::sync::RwLock;

lazy_static! {
    pub static ref OWNER_ID: RwLock<u64> = RwLock::new(0);
    pub static ref AI_TOKEN: RwLock<String> = RwLock::new(String::new());
    pub static ref AI_ENDPOINT: RwLock<String> = RwLock::new(String::from("https://api.monica.im/api/custom_bot/chat"));
    pub static ref AI_DEFAULT_MODEL: RwLock<String> = RwLock::new(String::from("openai-o-3-mini"));
    pub static ref AI_ENGAGE_TIME: RwLock<i64> = RwLock::new(60);
    pub static ref INIT_PROMPT: RwLock<String> = RwLock::new(String::from(""));
    pub static ref AI_AUTO_JOIN: RwLock<bool> = RwLock::new(false);
}
pub fn set_owner_id(id: u64) {
    *OWNER_ID.write().unwrap() = id;
}


pub fn set_ai_token(token: String) {
    *AI_TOKEN.write().unwrap() = token;

}

pub fn set_ai_endpoint(endpoint: String) {
    *AI_ENDPOINT.write().unwrap() = endpoint;
}

pub fn set_ai_default_model(model: String) {
    *AI_DEFAULT_MODEL.write().unwrap() = model;
}

pub fn set_ai_init_prompt(prompt: String) {
    *INIT_PROMPT.write().unwrap() = prompt;
}

pub fn set_ai_engage_time(time: i64) {
    *AI_ENGAGE_TIME.write().unwrap() = time;
}
pub fn set_ai_auto_join(auto_join: bool) {
    *AI_AUTO_JOIN.write().unwrap() = auto_join;
}