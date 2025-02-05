use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref OWNER_ID: Mutex<u64> = Mutex::new(0);
    pub static ref AI_TOKEN: Mutex<String> = Mutex::new(String::new());
    pub static ref AI_ENDPOINT: Mutex<String> = Mutex::new(String::from("https://api.monica.im/api/custom_bot/chat"));
    pub static ref AI_DEFAULT_MODEL: Mutex<String> = Mutex::new(String::from("openai-o-3-mini"));
    pub static ref INIT_PROMPT: Mutex<String> = Mutex::new(String::from("你是一个群聊中的聊天机器人。请友善地和所有成员互动，回答他们的问题并提供帮助，以营造一个积极的氛围。确保及时回应并适时插入幽默元素，让聊天更加轻松愉快。请不要使用markdown、json等人类难以理解的语言对话。请控制自己的发言在50个字符以下。现在，请加入以下对话："));
}

pub fn set_owner_id(id: u64) {
    if let Ok(mut owner_id) = OWNER_ID.lock() {
        *owner_id = id;
    }
}


pub fn set_ai_token(token: String) {
    if let Ok(mut ai_token) = AI_TOKEN.lock() {
        *ai_token = token;
    }
}

pub fn set_ai_endpoint(endpoint: String) {
    if let Ok(mut ai_endpoint) = AI_ENDPOINT.lock() {
        *ai_endpoint = endpoint;
    }
}

pub fn set_ai_default_model(model: String) {
    if let Ok(mut ai_default_model) = AI_DEFAULT_MODEL.lock() {
        *ai_default_model = model;
    }
}

pub fn set_ai_init_prompt(prompt: String) {
    if let Ok(mut init_prompt) = INIT_PROMPT.lock() {
        *init_prompt = prompt;
    }
}
