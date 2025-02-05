use super::super::dto::Data;

use std::process::Command;

pub fn exec(msg: &str) -> Result<Vec<Data>, crate::handler::DynErr> {
	
	let output = if cfg!(target_os = "windows") {
		Command::new("cmd")
			.args(["/C", msg])
			.output()
			.expect("failed to execute process")
	} else {
		Command::new("sh")
			.arg("-c")
			.arg(msg)
			.output()
			.expect("failed to execute process")
	};

	let ret = vec![Data::string(output.status.to_string()),
		Data::string(String::from_utf8_lossy(&output.stdout).to_string()),
		Data::string(String::from_utf8_lossy(&output.stderr).to_string())];

	Ok(ret)
}