use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};
use super::super::dto::{Data, RetMessage};
use super::DynErr;
use super::super::constants::OWNER_ID;
use redis::Client;

async fn process_command(msg: &str, sender: &Map<String, Value>, db: Arc<Client>) -> Result<Vec<Data>, DynErr> {
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
            if sender["user_id"].as_u64().unwrap() != *OWNER_ID.read().unwrap() {
				return Ok(vec![Data::string("Permission denied: Owner required".to_string())]);
			}
            crate::module::exec::exec(&args.join(" "))?
        }
        "ai" => {
            if args.get(0) == Some(&"!clear") {
                crate::module::ai::clear_record(0, db.clone(), "main").await?;
                if args.get(1) == Some(&"all") {
                    let bots = vec!["gemini_2_0".to_string(), "jv6tFQ5q".to_string(), "zzWzZzSg".to_string()];
                    for bot in bots {
                        crate::module::ai::clear_record(0, db.clone(), &bot).await?
                    }
                }
                vec![Data::string("Record cleared".to_string())]
            } else if args.get(0) == Some(&"!model") {
                crate::module::ai::set_model(0, db, args.get(1).unwrap_or(&"")).await?
            } else {
				crate::module::ai::main_conversation(None, db, &args.join(" ")).await?
            }
        }
        _ => {
            vec![Data::string("Unknown command".to_string())]
        }
    };

    Ok(ret)
}

fn _default_handler(_msg: &str, _sender: &Map<String, Value>) -> Result<Vec<Data>, DynErr> {
    Ok(vec![])
}

#[derive(Serialize, Debug, Clone)]
struct PrivateMessageParams {
	user_id: String,
	message: Vec<Data>,
}

fn resp(r: Result<Vec<Data>, DynErr>, uid: u64) -> RetMessage {

    let re = r.unwrap_or_else(|e| vec![Data::string(format!("Error: {:?}", e))]);
    let v = serde_json::to_value(PrivateMessageParams {
        user_id: uid.to_string(),
        message: re,
    }).unwrap();

    RetMessage {
        action: "send_private_msg".to_string(),
        params: v,
    }
}

pub async fn handle(msg: &Value, db: Arc<Client>) -> Result<Option<RetMessage>, DynErr> 
{
    let s = msg["sender"].as_object().unwrap();
    let m = msg["message"].as_array().unwrap();

    let mut in_msg = String::new();

    for segment in m {
        if segment["type"] == "text" {
            let txt = segment["data"]["text"].as_str().unwrap();
            in_msg += txt;
        }
    }

    let v = if in_msg.starts_with("~") {
        process_command(&in_msg, &s, db).await
    } else {
        return Ok(None);
    };
    Ok(Some(resp(v, msg["target_id"].as_u64().unwrap())))
}

