use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Debug, Clone)]
pub struct Data {
    #[serde(rename = "type")]
    pub type_: String,
    pub data: Value,
}

impl Data {
    pub fn string(text: String) -> Data {
        Data {
            type_: "text".to_string(),
            data: Value::Object(json!({ "text": text }).as_object().unwrap().clone()),
        }
    }
    pub fn at(qq: u64) -> Data {
        Data {
            type_: "at".to_string(),
            data: Value::Object(json!({ "qq": qq.to_string() }) .as_object().unwrap().clone()),
        }
    }
    pub fn reply(msg: u64) -> Data {
        Data {
            type_: "reply".to_string(),
            data: Value::Object(json!({ "id": msg }).as_object().unwrap().clone()),
        }
    }
}


#[derive(Serialize, Debug, Clone)]
pub struct RetMessage {
    pub action: String,
    pub params: Value,
}
#[derive(Deserialize, Debug, Clone)]
pub struct ImgData {
    pub summary: String,
    pub file: String,
    pub url: String,
    pub file_size: String,
}



#[derive(Debug, Serialize)]
pub struct ChatData {
    pub task_uid: String,
    pub bot_uid: String,
    pub data: ConversationData,
    pub language: &'static str,
    pub locale: &'static str,
    pub task_type: &'static str,
    pub tool_data: ToolData,
    pub ai_resp_language: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ConversationData {
    pub conversation_id: String,
    pub items: Vec<ConversationItem>,
    pub pre_generated_reply_id: String,
    pub pre_parent_item_id: String,
    pub origin: &'static str,
    pub origin_page_title: &'static str,
    pub trigger_by: &'static str,
    pub use_model: String,
    pub is_incognito: bool,
    pub use_new_memory: bool,
}

#[derive(Debug, Serialize)]
pub struct ConversationItem {
    pub item_id: String,
    pub conversation_id: String,
    pub item_type: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_item_id: Option<String>,
    pub data: ItemData,
}

#[derive(Debug, Serialize)]
pub struct ImageItem {
	pub use_full_text: bool,
	pub file_name: String,
	pub file_type: String,
	pub file_ext: String,
	pub file_size: u64,
	pub file_url: String,
	pub file_uid: String,
	pub file_chunks: u64,
	pub file_tokens: u64,
}

#[derive(Debug, Serialize)]
pub struct ItemData {
    #[serde(rename = "type")]
    pub data_type: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_token: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_incognito: Option<bool>,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    pub file_infos: Option<Vec<ImageItem>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolData {
    pub sys_skill_list: Vec<String>,
}