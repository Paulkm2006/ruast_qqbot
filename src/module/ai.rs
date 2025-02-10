use std::sync::Arc;
use redis::{Client, AsyncCommands};
use uuid::Uuid;
use crate::constants::{*};
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use tokio::time::{timeout, Duration}; // add timeout

use crate::dto::{*};

// Add global lock to ensure a single main conversation process at a time.
static MAIN_CONVO_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub async fn clear_record(gid: u64, db:Arc<Client>, b: &str) -> Result<(), crate::handler::DynErr> {
	let mut conn = db.get_multiplexed_async_connection().await?;
	let bot;

	if b == "main" {
		let main_model: String = conn.get(format!("ai:{}:model",gid)).await.unwrap_or_else(|_| AI_DEFAULT_MODEL.read().unwrap().clone());
		bot = main_model.replace("-", "_").replace(".", "_");
	}else{
		bot = b.to_string();
	}
	let _: () = conn.set(format!("ai:{}:{}:conv", gid, bot), Uuid::new_v4().to_string()).await?;
	let _: () = conn.set(format!("ai:{}:{}:prev", gid, bot), Uuid::new_v4().to_string()).await?;
	let _: () = conn.set(format!("ai:{}:{}:now", gid, bot), Uuid::new_v4().to_string()).await?;
	let _: () = conn.set(format!("ai:{}:{}:count", gid, bot), 0).await?;

	Ok(())
}

pub async fn set_model(gid: u64, db:Arc<Client>, model: &str) -> Result<Vec<Data>, crate::handler::DynErr> {

	let mut conn = db.get_multiplexed_async_connection().await?;
	let _: () = conn.set(format!("ai:model:{}", gid), model.to_string()).await?;

	clear_record(gid, db, "main").await?;
	Ok(vec![Data::string("Model set".to_string())])
}

pub async fn check_join(gid: u64, db:Arc<Client>) -> Result<bool, crate::handler::DynErr> {
	let mut conn = db.get_multiplexed_async_connection().await?;
	Ok(conn.exists(format!("ai:{}:JOIN", gid)).await?)
}

pub async fn set_join(gid: u64, db:Arc<Client>) -> Result<(), crate::handler::DynErr> {
	let mut conn = db.get_multiplexed_async_connection().await?;
	let key = format!("ai:{}:JOIN", gid);
	let t = AI_ENGAGE_TIME.read().unwrap().clone();
	if conn.exists(&key).await? {
		let _:() = conn.expire(key, t).await?;
	}else{
		let _:() = conn.set(&key, 1).await?;
		let _:() = conn.expire(key, t).await?;
	}
	Ok(())
}


pub async fn main_conversation(gid: Option<u64>, db:Arc<Client>, msg: &str) -> Result<Vec<Data>, crate::handler::DynErr>{
    // Use a timeout when acquiring the lock to avoid deadlocks.
    let _lock = tokio::time::timeout(Duration::from_secs(120), MAIN_CONVO_LOCK.lock())
        .await
        .map_err(|_| "Timeout waiting for conversation lock")?;
	let gid = match gid {
		Some(gid) => gid,
		None => 0,
	};

	let mut conn = db.get_multiplexed_async_connection().await?;


	let main_model: String = conn.get(format!("ai:{}:model",gid)).await.unwrap_or_else(|_| AI_DEFAULT_MODEL.read().unwrap().clone());
	let main_bot = main_model.clone().replace("-", "_").replace(".", "_");


	println!("AI request: {:?}", msg);
	let main_resp = conversation(gid, &main_model, &main_bot, db.clone(), &msg).await?;
	println!("AI response: {:?}", main_resp);

	Ok(vec![Data::string(main_resp)])
}

pub async fn conversation(gid: u64, model: &str, bot: &str, db:Arc<Client>, msg: &str) -> Result<String, crate::handler::DynErr> {

	let mut conn = db.get_multiplexed_async_connection().await?;
	let conv: String = conn.get(format!("ai:{}:{}:conv", gid, bot)).await?;
	let prev: String = conn.get(format!("ai:{}:{}:prev", gid, bot)).await?;
	let now: String = conn.get(format!("ai:{}:{}:now", gid, bot)).await?;
	let count: i32 = conn.get(format!("ai:{}:{}:count", gid, bot)).await?;

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
		msg = INIT_PROMPT.read().unwrap().clone() + &msg;
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
			is_incognito: Some(false),
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
			is_incognito: false,
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

	let mut conn = db.get_multiplexed_async_connection().await?;
	let _: () = conn.set(format!("ai:{}:{}:prev", gid, bot), next_msg).await?;
	let _: () = conn.set(format!("ai:{}:{}:now", gid, bot), Uuid::new_v4().to_string()).await?;
	let _: () = conn.incr(format!("ai:{}:{}:count", gid, bot), 1).await?;

    Ok(resp)
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

    loop {
        match timeout(Duration::from_secs(30), event_source.next()).await {
            Ok(Some(event)) => {
                match event {
                    Ok(Event::Open) => { /* connection opened */ },
                    Ok(Event::Message(message)) => {
                        let v: serde_json::Value = serde_json::from_str(&message.data)?;
                        resp += v["text"].as_str().unwrap();
                        if v.get("finished").and_then(|b| b.as_bool()).unwrap_or(false) {
                            break;
                        }
                    },
                    Err(e) => return Err(crate::handler::DynErr::from(e)),
                }
            },
            Ok(None) => break,
            Err(_) => return Err("Timeout during event streaming".into()),
        }
    }
    Ok(resp)
}