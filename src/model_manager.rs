use std::{io::{BufRead,BufReader},time::Duration};
pub fn progress(v:&serde_json::Value)->Option<f32>{
 let total=v["total"].as_u64()?; let done=v["completed"].as_u64()?;
 if total==0{return None;} Some((done.min(total) as f64/total as f64) as f32)
}
pub fn pull(model:&str,mut notify:impl FnMut(String,Option<f32>))->Result<(),Box<dyn std::error::Error>>{
 if model.trim().is_empty(){return Err("Empty model name".into());}
 let client=reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(7200)).build()?;
 let response=client.post("http://127.0.0.1:11434/api/pull").json(&serde_json::json!({"model":model,"stream":true})).send()?.error_for_status()?;
 for line in BufReader::new(response).lines(){
 let line=line?; if line.trim().is_empty(){continue;}
 let v:serde_json::Value=serde_json::from_str(&line)?;
 if let Some(e)=v["error"].as_str(){return Err(e.to_owned().into());}
 let status=v["status"].as_str().unwrap_or("Downloading");
 notify(status.to_owned(),progress(&v));
 if status=="success"{return Ok(());}
 }
 Err("Download stream ended before success".into())
}
#[cfg(test)] mod tests{
 use super::*;
 #[test] fn progress_bounds(){assert_eq!(progress(&serde_json::json!({"total":100,"completed":50})),Some(0.5));assert_eq!(progress(&serde_json::json!({"total":0,"completed":1})),None);assert_eq!(progress(&serde_json::json!({"total":100,"completed":200})),Some(1.0));}
}
