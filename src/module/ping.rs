use super::super::dto::Data;

pub fn ping(name: &str) -> Vec<Data> {
	vec![Data::string(format!("pong to {}", name))]
}