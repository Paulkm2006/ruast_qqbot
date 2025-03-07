use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};
use crate::module::ai_img::process_image;

use super::super::dto::{*};
use super::DynErr;
use super::super::constants::{OWNER_ID, AI_AUTO_JOIN};
use redis::Client;
use log::{info,error};

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

async fn process_command(msg_id: u64, msg: &str, sender: &Map<String, Value>, db: Arc<Client>, gid: u64) -> Result<Vec<Data>, DynErr> {
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
                crate::module::ai::clear_record(gid, db.clone(), "main").await?;
                vec![Data::string("Record cleared".to_string())]
            } else if args.get(0) == Some(&"!model") {
                allow!(sender, Identity::Owner); // Require owner for model
                crate::module::ai::set_model(gid, db, args.get(1).unwrap_or(&"")).await?
            } else {
                crate::module::ai::main_conversation(Some(gid), db, &args.join(" ")).await?
            }
        }
        _ => {
            vec![Data::string("Unknown command".to_string())]
        }
    };

    info!("[{msg_id} <=cmd] {}", ret[0].data["text"]);

    Ok(ret)
}

async fn default_handler(msg_id: u64, nick: String, uid: u64, msg: &str, img: &Vec<ImgData>, db:Arc<Client>, gid: u64, reply: Option<u64>) -> Result<Vec<Data>, DynErr> {
    let mut prompt = nick.clone();
    prompt += "发送了以下内容：\n";
    if !msg.is_empty() {
        prompt += "文字：";
        prompt += msg;
        prompt += "\n";
    }
    if !img.is_empty() {
        for i in img{
            prompt += &format!("图片：{} {}\n", i.summary, process_image(i).await?);
        }
    }
    let mut ret = crate::module::ai::main_conversation(Some(gid), db, &prompt).await?;
    info!("[{msg_id} <=ai_reply] {}", ret[0].data["text"]);
    if let Some(id) = reply {
        ret.insert(0,Data::reply(id));
        ret.insert(0, Data::at(uid));
    }
    Ok(ret)
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
    let s_id = s["user_id"].as_u64().unwrap();
    let s_nick = s["nickname"].as_str().unwrap();
    let m = msg["message"].as_array().unwrap();
    let self_id = msg["self_id"].as_u64().unwrap();
    let msg_id = msg["message_id"].as_u64().unwrap();

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
                    info!("[{msg_id} {gid} {s_nick}] <=image] {}", img_data.file);
                    in_img.push(img_data);
                }
            }
            else if let Some(img_summary) = segment["data"]["summary"].as_str() {
                info!("[{msg_id} {gid} {s_nick}] <=sticker] {}", img_summary);
                in_img.push(ImgData{
                    file: "".to_string(),
                    url: "".to_string(),
                    file_size: "".to_string(),
                    summary: img_summary.to_string(),
                });
            }
        }
    }


    if at {
        
        let v = if in_msg.starts_with(" ~") {
            info!("[{msg_id} {gid} {s_nick}] >=cmd] {}", in_msg);
            process_command(msg_id, &in_msg, &s, db, gid).await
        } else {
            crate::module::ai::set_join(gid, db.clone()).await?;
            info!("[{msg_id} {gid} {s_nick}] >=ai_at] {}", in_msg);
            default_handler(msg_id, s_nick.to_owned(),s_id, &in_msg, &in_img, db, gid, Some(msg_id)).await
        };

        let r = v.unwrap_or_else(|e| {
            error!("[{msg_id}] <=at] {}", e);
            vec![Data::string(format!("Error: {:?}", e))] });

        Ok(Some(resp(r, gid)))
    }else{
        if *AI_AUTO_JOIN.read().unwrap() && crate::module::ai::check_join(gid, db.clone()).await? {
            info!("[{msg_id} {gid} {s_nick}] =>ai_auto] {}", in_msg);
            let v = default_handler(msg_id, s_nick.to_owned(),s_id, &in_msg, &in_img, db, gid, None).await;
            let r = v.unwrap_or_else(|e| {
                error!("[{msg_id} <=ai_auto>] {:?}", e);
                vec![Data::string(format!("Error: {:?}", e))] });
            Ok(Some(resp(r, gid)))
        } else {
            Ok(None)
        }
    }
}

