use std::sync::Arc;
use redis::{Client, Commands};
use uuid::Uuid;
use crate::constants::{*};
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use tokio::sync::Mutex;
use once_cell::sync::Lazy;

use crate::dto::{*};

// Add global lock to ensure a single main conversation process at a time.
static MAIN_CONVO_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub fn clear_record(gid: u64, db:Arc<Client>, b: &str) -> Result<(), crate::handler::DynErr> {
	let mut conn = db.get_connection()?;
	let bot;

	if b == "main" {
		let main_model: String = conn.get(format!("ai:{}:model",gid)).unwrap_or_else(|_| AI_DEFAULT_MODEL.read().unwrap().clone());
		bot = main_model.replace("-", "_").replace(".", "_");
	}else{
		bot = b.to_string();
	}
	let _: () = conn.set(format!("ai:{}:{}:conv", gid, bot), Uuid::new_v4().to_string())?;
	let _: () = conn.set(format!("ai:{}:{}:prev", gid, bot), Uuid::new_v4().to_string())?;
	let _: () = conn.set(format!("ai:{}:{}:now", gid, bot), Uuid::new_v4().to_string())?;
	let _: () = conn.set(format!("ai:{}:{}:count", gid, bot), 0)?;

	Ok(())
}

pub fn set_model(gid: u64, db:Arc<Client>, model: &str) -> Result<Vec<Data>, crate::handler::DynErr> {

	let mut conn = db.get_connection()?;
	let _: () = conn.set(format!("ai:model:{}", gid), model.to_string())?;

	clear_record(gid, db, "main")?;
	Ok(vec![Data::string("Model set".to_string())])
}

pub fn check_join(gid: u64, db:Arc<Client>) -> Result<bool, crate::handler::DynErr> {
	let mut conn = db.get_connection()?;
	Ok(conn.exists(format!("ai:{}:JOIN", gid))?)
}

pub fn set_join(gid: u64, db:Arc<Client>) -> Result<(), crate::handler::DynErr> {
	let mut conn = db.get_connection()?;
	let key = format!("ai:{}:JOIN", gid);
	if conn.exists(&key)? {
		let _:() = conn.expire(key, AI_ENGAGE_TIME.read().unwrap().clone())?;
	}else{
		let _:() = conn.set(&key, 1)?;
		let _:() = conn.expire(key, AI_ENGAGE_TIME.read().unwrap().clone())?;
	}
	Ok(())
}


pub async fn main_conversation(gid: Option<u64>, db:Arc<Client>, msg: &str) -> Result<Vec<Data>, crate::handler::DynErr>{
    // Ensure only one main conversation runs concurrently.
    let _lock = MAIN_CONVO_LOCK.lock().await;
	let gid = match gid {
		Some(gid) => gid,
		None => 0,
	};

	let mut conn = db.get_connection()?;

	let main_model: String = conn.get(format!("ai:{}:model",gid)).unwrap_or_else(|_| AI_DEFAULT_MODEL.read().unwrap().clone());
	let main_bot = main_model.clone().replace("-", "_").replace(".", "_");

	let resp;
	let mut next_msg = msg.to_owned();


	loop {
		println!("AI request: {:?}", next_msg);
		let main_resp = conversation(gid, &main_model, &main_bot, db.clone(), &next_msg, false).await?;
		println!("AI response: {:?}", main_resp);
		if main_resp.starts_with("!#[") {
			let tool_resp = use_tool(db.clone(), &main_resp).await?;
			next_msg = tool_resp;
		} else {
			resp = main_resp;
			break;
		}
	}

	Ok(vec![Data::string(resp)])
}

pub async fn conversation(gid: u64, model: &str, bot: &str, db:Arc<Client>, msg: &str, tool: bool) -> Result<String, crate::handler::DynErr> {



	let (conv, prev, now, count) = if !tool {
		let mut conn = db.get_connection()?;
		let conv: String = conn.get(format!("ai:{}:{}:conv", gid, bot))?;
		let prev: String = conn.get(format!("ai:{}:{}:prev", gid, bot))?;
		let now: String = conn.get(format!("ai:{}:{}:now", gid, bot))?;
		let count: i32 = conn.get(format!("ai:{}:{}:count", gid, bot))?;
		(conv, prev, now, count)
	} else {
		(Uuid::new_v4().to_string(), Uuid::new_v4().to_string(), Uuid::new_v4().to_string(), 0)
	};
	

	let next_msg = Uuid::new_v4().to_string();

	let mut items = Vec::new();

	let mut msg = msg.to_owned();
	if count == 0 {
		items.push(ConversationItem {
			item_id: "msg:".to_owned()+&prev,
			conversation_id: "conv:".to_owned()+&conv,
			item_type: "reply".to_string(),
			summary: "__RENDER_BOT_WELCOME_MSG__".to_string(),
			parent_item_id: None,
			data: ItemData {
				data_type: "text".to_string(),
				content: "__RENDER_BOT_WELCOME_MSG__".to_string(),
				quote_content: None,
				max_token: None,
				is_incognito: None,
				file_infos: None,
			},
		});
		if !tool {
			msg = INIT_PROMPT.read().unwrap().clone() + &msg;
		}
	}

	items.push(ConversationItem {
		item_id: "msg:".to_owned()+&now,
		conversation_id: "conv:".to_owned()+&conv,
		item_type: "question".to_string(),
		summary: msg.to_string(),
		parent_item_id: Some("msg:".to_owned()+&prev),
		data: ItemData {
			data_type: "text".to_string(),
			content: msg.to_string(),
			quote_content: Some("".to_string()),
			max_token: Some(0),
			is_incognito: Some(tool),
			file_infos: None,
		},
	});

	let req = ChatData {
		task_uid: "task:".to_owned()+Uuid::new_v4().to_string().as_str(),
		bot_uid: bot.to_owned(),
		data: ConversationData {
			conversation_id: "conv:".to_owned()+&conv,
			items,
			pre_generated_reply_id: "msg:".to_owned()+&next_msg,
			pre_parent_item_id: "msg:".to_owned()+&now,
			origin: "https://monica.im/home/chat/DeepSeek%20V3/deepseek_chat",
			origin_page_title: "o3-mini - Monica 智能体",
			trigger_by: "auto",
			use_model: model.to_owned(),
			is_incognito: tool,
			use_new_memory: true,
		},
		language: "auto",
		locale: "zh_CN",
		task_type: "chat",
		tool_data: ToolData {
			sys_skill_list: vec![],
		},
		ai_resp_language: "Chinese (Simplified)",
	};

	let resp = send_request(&req).await?;

	if !tool {
		let mut conn = db.get_connection()?;
		let _: () = conn.set(format!("ai:{}:{}:prev", gid, bot), next_msg)?;
		let _: () = conn.set(format!("ai:{}:{}:now", gid, bot), Uuid::new_v4().to_string())?;
		let _: () = conn.incr(format!("ai:{}:{}:count", gid, bot), 1)?;
	}

    Ok(resp)
}

async fn use_tool(db: Arc<Client>, query: &str) -> Result<String, crate::handler::DynErr> {
	let catch = regex::Regex::new(r"!#\[(\w+)\(").unwrap();
	let tool = if let Some(caps) = catch.captures(query) {
		if let Some(m) = caps.get(1) {
			m.as_str()
		} else {
			return Ok("不合法的调用".to_owned());
		}
	} else {
		return Ok("不合法的调用".to_owned());
	};
	match tool{
		"search" | "getLastRate" | "getCryptoInformation" | "getIpInfo" | "searchNews" => {
			conversation(0, "gpt-4o", "jv6tFQ5q", db, query, true).await
		},
		"readURL" | "getTopNews" | "getCurrentTime" | "getWeather" => {
			conversation(0, "gpt-4o", "zzWzZzSg", db, query, true).await
		},
		_ => Ok("不合法的调用".to_owned()),
	}
}

pub async fn send_request(req: &ChatData) -> Result<String, crate::handler::DynErr> {

    // Setup cookie jar
    let jar = Arc::new(reqwest::cookie::Jar::default());
    jar.add_cookie_str(
        format!("session_id={}", AI_TOKEN.read().unwrap().as_str()).as_str(),
        &reqwest::Url::parse("https://api.monica.im").unwrap()
    );

    let client = reqwest::Client::builder()
        .cookie_provider(Arc::clone(&jar))
        .build()?;

    let request_url = reqwest::Url::parse(AI_ENDPOINT.read().unwrap().as_str())?;
    let request = client.post(request_url)
        .json(&req);


    // Use server-sent events to receive response
    let mut event_source = EventSource::new(request)?;
    let mut resp = String::new();

    while let Some(event) = event_source.next().await {
        match event {
            Ok(Event::Open) => { /* connection opened */ },
            Ok(Event::Message(message)) => {
                let v: serde_json::Value = serde_json::from_str(&message.data)?;
                resp += v["text"].as_str().unwrap();
				
				if let Some(_) = v["finished"].as_bool() {
					break;
				}
            },
			Err(e) => {
				return Err(crate::handler::DynErr::from(e));
			},
        }
    }
    Ok(resp)
}