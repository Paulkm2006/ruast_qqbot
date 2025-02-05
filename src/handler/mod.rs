pub mod group;
pub mod private;

use std::sync::Arc;
use crate::dto::RetMessage;

use futures::{stream::SplitSink, SinkExt as _};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use serde_json::Value;
use redis::Client;

pub type Sender = Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>;
pub type DynErr = Box<dyn std::error::Error + Send + Sync>;

async fn send(response: RetMessage, sender: Sender) -> Result<(), DynErr> {
	let j = serde_json::to_string(&response).unwrap();
	let message = Message::Text(j.into());
	sender.lock().await.send(message).await?;
	Ok(())
}

pub async fn recv(msg: &str, sender: Sender, db: Arc<Client>) -> Result<(), DynErr>
{
	let msg = msg.to_string();
	let msg: Value = serde_json::from_str(&msg).unwrap();

	if let Some(status) = msg["status"].as_str(){
		if status != "ok"{
			return Err(format!("Received a message with status not ok\n{:?}\n", msg).into());
		}else{
			return Ok(());
		}
	}

	let resp = match msg["post_type"].as_str().unwrap(){
		"message" => {
			match msg["message_type"].as_str().unwrap(){
				"group" => {
					group::handle(&msg, db).await
				}
				"private" => {
					private::handle(&msg, db).await
				}
				_ => {
					Ok(None)
				}
			}
		}
		_ => {
			Ok(None)
		}
	};
	if let Some(resp) = resp? {
		send(resp, sender).await?;
	}
	Ok(())
}