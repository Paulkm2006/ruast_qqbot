use std::{sync::Arc, thread::sleep, time::Duration};

use rand::prelude::*;
use uuid::Uuid;

use crate::constants::{*};
use crate::dto::{*};

use super::ai::send_request;



pub async fn process_image(data: &ImgData) -> Result<String, crate::handler::DynErr> {
	let filename = data.file.clone();
	println!("Processing image: {}", filename);
	let file_size = data.file_size.parse::<u64>().unwrap();
	let url = data.url.clone();
	let item = upload_image(&filename, file_size, &url).await?;
	explain_image(item).await
}

pub async fn explain_image(img: ImageItem) -> Result<String, crate::handler::DynErr> {
	let mut items = Vec::new();

	let conv = Uuid::new_v4().to_string();

	let start_id = Uuid::new_v4().to_string();

	items.push(ConversationItem {
		item_id: "msg:".to_owned()+&start_id,
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

	let msg_id = Uuid::new_v4().to_string();

	items.push(ConversationItem {
		item_id: "msg:".to_owned()+&msg_id,
		conversation_id: "conv:".to_owned()+&conv,
		item_type: "question".to_string(),
		summary: "解释这张图片 ".to_string(),
		parent_item_id: Some("msg:".to_owned()+&start_id),
		data: ItemData {
			data_type: "file_with_text".to_string(),
			content: "解释这张图片 ".to_string(),
			quote_content: None,
			max_token: Some(0),
			is_incognito: Some(true),
			file_infos: Some(vec![img]),
		},
	});

	let cdata = ConversationData {
		conversation_id: conv.clone(),
		items,
		pre_generated_reply_id: "msg:".to_owned()+&Uuid::new_v4().to_string(),
		pre_parent_item_id: "msg:".to_owned()+&msg_id,
		origin: "https://monica.im/home/chat/Gemini%202.0%20Flash/gemini_2_0",
		origin_page_title: "Gemini 2.0 Flash - Monica Bots",
		trigger_by: "auto",
		use_model: "gemini-2.0".to_string(),
		is_incognito: true,
		use_new_memory: true,
	};

	let req = ChatData {
		task_uid: "task:".to_owned()+Uuid::new_v4().to_string().as_str(),
		bot_uid: "gemini_2_0".to_string(),
		data: cdata,
		language: "auto",
		locale: "zh_CN",
		task_type: "chat",
		tool_data: ToolData {
			sys_skill_list: vec![],
		},
		ai_resp_language: "Chinese (Simplified)",
	};

	let resp = send_request(&req).await?;

	println!("Picture description: {}", resp);
	
	Ok(resp)
}


pub async fn download_image(url: &str) -> Result<Vec<u8>, crate::handler::DynErr> {
	let client = reqwest::Client::new();
	let resp = client.get(url).send().await?;
	let bytes = resp.bytes().await?;
	Ok(bytes.to_vec())
}

pub async fn upload_image(filename: &str, file_size: u64, url: &str) -> Result<ImageItem, crate::handler::DynErr> {
	let bytes = download_image(url).await?;

	let jar = Arc::new(reqwest::cookie::Jar::default());
    jar.add_cookie_str(
        format!("session_id={}", AI_TOKEN.lock().unwrap().as_str()).as_str(),
        &reqwest::Url::parse("https://api.monica.im").unwrap()
    );

    let client = reqwest::Client::builder()
        .cookie_provider(Arc::clone(&jar))
        .build()?;

	let pre_url = "https://api.monica.im/api/file_object/pre_sign_list_by_module";
	let obj_id: String = rand::rng().sample_iter(&rand::distr::Alphanumeric).take(22).map(char::from).collect();


	let resp = client.post(pre_url)
		.json(&serde_json::json!({
			"filename_list": vec![filename.to_string()],
			"location": "files".to_string(),
			"module": "chat_bot".to_string(),
			"obj_id": obj_id,
		}))
		.send()
		.await?;
	let pre_json = resp.json::<serde_json::Value>().await?;


	let upload_url = pre_json["data"]["pre_sign_url_list"][0].as_str().unwrap();
	let object_url = pre_json["data"]["object_url_list"][0].as_str().unwrap();
	let _ = client.put(upload_url)
		.body(bytes)
		.send()
		.await?;


	let create_url = "https://api.monica.im/api/files/batch_create_llm_file";
	let create_resp = client.post(create_url)
		.json(&serde_json::json!({
			"data":[
				{"url":"","parse":true,
				"file_name":filename.to_string(),
				"file_size":file_size,
				"file_type":filename.split('.').last().unwrap(),
				"object_url":object_url,
				"embedding":false}
				]}))
		.send()
		.await?;
	let create_json = create_resp.json::<serde_json::Value>().await?;
	let file_uid = create_json["data"]["items"][0]["file_uid"].as_str().unwrap();

	let mut t = 0;
	let file_tokens;
	let file_chunks;
	loop {
		let check_url = "https://api.monica.im/api/files/batch_get_file";
		let check_resp = client.post(check_url)
			.json(&serde_json::json!({
				"file_uids":[
					file_uid
					]}))
			.send()
			.await?;
		let check_json = check_resp.json::<serde_json::Value>().await?;
		match check_json["data"]["items"][0]["index_state"].as_u64().unwrap() {
			3 => {
				file_tokens = check_json["data"]["items"][0]["file_tokens"].as_u64().unwrap();
				file_chunks = check_json["data"]["items"][0]["file_chunks"].as_u64().unwrap();
				break;
			}
			2 => {
				return Err(("Upload processing error:\n".to_owned()+check_json["data"]["items"][0]["error_message"].as_str().unwrap()).into());
			}
			_ => {}
		}
		sleep(Duration::from_millis(1000));
		t += 1;
		if t > 5 {
			return Err("Upload failed".to_string().into());
		}
	}

	Ok(ImageItem {
		use_full_text: true,
		file_name: filename.to_string(),
		file_type: filename.split('.').last().unwrap().to_string(),
		file_ext: filename.split('.').last().unwrap().to_string(),
		file_size: file_size as u64,
		file_url: object_url.to_string(),
		file_uid: file_uid.to_string(),
		file_chunks,
		file_tokens
	})
}