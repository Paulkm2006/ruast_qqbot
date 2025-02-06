use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};
use super::super::dto::{*};
use super::DynErr;
use super::super::constants::OWNER_ID;
use redis::Client;

#[derive(PartialEq)]
enum Identity {
    Owner,
    User,
}

fn get_identity(sender: &Map<String, Value>) -> Identity {
    let user_id = sender["user_id"].as_u64().unwrap();
    if user_id == *OWNER_ID.read().unwrap() {
        Identity::Owner
    } else {
        Identity::User
    }
}

#[macro_export]
macro_rules! allow {
    ($sender:expr, $required:expr) => {
        let identity = get_identity($sender);
        match $required {
            Identity::Owner if identity != Identity::Owner => {
                return Ok(vec![Data::string("Permission denied: Owner required".to_string())]);
            }
            _ => {}
        }
    };
}

async fn process_command(msg: &str, sender: &Map<String, Value>, db: Arc<Client>, gid: u64) -> Result<Vec<Data>, DynErr> {
    let mut msg = msg.split_whitespace();
    let cmd = msg.next().unwrap();
    let args = msg.collect::<Vec<&str>>();

    let ret = match &cmd[1..] {
        "echo" => {
            vec![Data::string(args.join(" "))]
        }
        "ping" => {
            crate::module::ping::ping(&sender["nickname"].as_str().unwrap())
        }
        "exec" => {
            allow!(sender, Identity::Owner); // Require owner for exec
            crate::module::exec::exec(&args.join(" "))?
        }
        "ai" => {
            if args.get(0) == Some(&"!clear") {
                allow!(sender, Identity::Owner); // Require owner for clear
                crate::module::ai::clear_record(gid, db.clone(), "main")?;
                if args.get(1) == Some(&"all") {
                    let bots = vec!["gemini_2_0".to_string(), "jv6tFQ5q".to_string(), "zzWzZzSg".to_string()];
                    for bot in bots {
                        crate::module::ai::clear_record(0, db.clone(), &bot)?
                    }
                }
                vec![Data::string("Record cleared".to_string())]
            } else if args.get(0) == Some(&"!model") {
                allow!(sender, Identity::Owner); // Require owner for model
                crate::module::ai::set_model(gid, db, args.get(1).unwrap_or(&""))?
            } else {
                crate::module::ai::main_conversation(Some(gid), db, &args.join(" ")).await?
            }
        }
        _ => {
            vec![Data::string("Unknown command".to_string())]
        }
    };

    Ok(ret)
}

async fn default_handler(sender: &str, msg: &str, img: &Vec<ImgData>, _sender: &Map<String, Value>, db:Arc<Client>, gid: u64) -> Result<Vec<Data>, DynErr> {
    
    
    let mut prompt = sender.to_owned() + "说：\n";
    if !img.is_empty() {
        for i in img {
            let desc = crate::module::ai_img::process_image(&i).await?;
            prompt += &format!("收到一张图片：{}\n{}\n", i.summary, desc);
        }
    }
    prompt += msg;
    Ok(crate::module::ai::main_conversation(Some(gid), db, &prompt).await?)
}

#[derive(Serialize, Debug, Clone)]
struct GroupMessageParams {
	group_id: String,
	message: Vec<Data>,
}

fn resp(r: Vec<Data>, gid: u64) -> RetMessage {

    let v = serde_json::to_value(GroupMessageParams {
        group_id: gid.to_string(),
        message: r,
    }).unwrap();

    RetMessage {
        action: "send_group_msg".to_string(),
        params: v,
    }
}

pub async fn handle(msg: &Value, db: Arc<Client>) -> Result<Option<RetMessage>, DynErr> 
{
    let s = msg["sender"].as_object().unwrap();
    let m = msg["message"].as_array().unwrap();
    let self_id = msg["self_id"].as_u64().unwrap();

    let mut at = false;
    let mut in_msg = String::new();
    let mut in_img = vec![];

    let gid = msg["group_id"].as_u64().unwrap();

    for segment in m {
        if segment["type"] == "at" && segment["data"]["qq"] == self_id.to_string() {
            at = true;
        }
        if segment["type"] == "text" {
            let txt = segment["data"]["text"].as_str().unwrap();
            in_msg += txt;
        }
        if segment["type"] == "image" {
            if let Ok(img_data) = serde_json::from_value::<ImgData>(segment["data"].clone()){
                if img_data.file_size.parse::<u64>().unwrap() > 1024 {
                    in_img.push(img_data);
                }
            }
        }
    }

    if at {
        if in_msg.starts_with(" ~") {
            let v = process_command(&in_msg, &s, db, gid).await?;
            Ok(Some(resp(v, gid)))
        } else {
            crate::module::ai::set_join(gid, db.clone())?;
            let nickname = s["nickname"].as_str().unwrap_or_else(|| "Unknown");
            let v = default_handler(nickname, &in_msg, &in_img, &s, db, gid).await?;
            Ok(Some(resp(v, gid)))
        }
    }else{
        if crate::module::ai::check_join(gid, db.clone())? {
            let nickname = s["nickname"].as_str().unwrap_or_else(|| "Unknown");
            let v = default_handler(nickname, &in_msg, &in_img, &s, db, gid).await?;
            Ok(Some(resp(v, gid)))
        } else {
            Ok(None)
        }
    }
}

