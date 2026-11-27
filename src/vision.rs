//! Vision transport for explicitly selected local Ollama models.
use crate::inference::{Backend,Message,Ollama};
use base64::{Engine,engine::general_purpose::STANDARD};
use serde_json::{json,Value};
use std::{io::{BufReader,Read},time::Duration};
pub const MAX_IMAGE:usize=4*1024*1024;
pub const MAX_TOTAL:usize=12*1024*1024;
pub const MAX_COUNT:usize=4;
pub struct Vision{images:Vec<Vec<u8>>}
impl Vision{
 pub fn new(images:Vec<Vec<u8>>)->Result<Self,String>{
  if images.is_empty()||images.len()>MAX_COUNT||images.iter().any(|i|i.is_empty()||i.len()>MAX_IMAGE||!i.starts_with(&[0xff,0xd8,0xff]))||images.iter().map(Vec::len).sum::<usize>()>MAX_TOTAL{return Err("Invalid normalized image payload".into());}
  Ok(Self{images})
 }
 fn body(&self,model:&str,messages:&[Message])->Result<Value,String>{
  if !valid_model(model){return Err("Select an installed local vision model (not a cloud model)".into());}
  if messages.last().map(|m|m.role.as_str())!=Some("user"){return Err("Image request must end with a user message".into());}
  let mut rows:Vec<Value>=messages.iter().map(|m|json!({"role":m.role,"content":m.content})).collect();
  rows.last_mut().unwrap()["images"]=json!(self.images.iter().map(|b|STANDARD.encode(b)).collect::<Vec<_>>());
  Ok(json!({"model":model,"messages":rows,"stream":true,"options":{"num_predict":2048}}))
 }
}
fn valid_model(model:&str)->bool{
 let s=model.trim();
 !s.is_empty()&&s.len()<=200&&!s.contains(char::is_whitespace)&&!s.contains("://")&&!s.to_ascii_lowercase().contains("cloud")
}
fn supports_vision(v:&Value)->bool{
 v["capabilities"].as_array().is_some_and(|a|a.iter().any(|c|c.as_str()==Some("vision")))
 && ["remote_host","remote_model"].iter().all(|key|v[*key].is_null()||v[*key].as_str()==Some(""))
}
impl Backend for Vision{
 fn models(&self)->Result<Vec<String>,String>{Ollama.models()}
 fn generate(&self,model:&str,messages:&[Message],emit:&mut dyn FnMut(String)->bool)->Result<(),String>{
  let payload=self.body(model,messages)?;
  let client=reqwest::blocking::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).connect_timeout(Duration::from_secs(4)).timeout(Duration::from_secs(300)).build().map_err(|e|e.to_string())?;
  let response=client.post("http://127.0.0.1:11434/api/show").json(&json!({"model":model})).send().and_then(|r|r.error_for_status()).map_err(|e|format!("Vision model check: {e}"))?;
  let mut data=Vec::new();response.take(2*1024*1024+1).read_to_end(&mut data).map_err(|e|e.to_string())?;
  if data.len()>2*1024*1024{return Err("Vision capability response too large".into());}
  let info:Value=serde_json::from_slice(&data).map_err(|e|e.to_string())?;
  if !supports_vision(&info){return Err("Model does not advertise local vision support. Install a vision model in Ollama.".into());}
  let response=client.post("http://127.0.0.1:11434/api/chat").json(&payload).send().and_then(|r|r.error_for_status()).map_err(|e|format!("Vision: {e}"))?;
  crate::inference::decode(BufReader::new(response.take(4*1024*1024)),emit).map_err(|e|e.to_string())
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn image_only_on_last_user(){let b=Vision::new(vec![vec![255,216,255,1]]).unwrap();let rows=vec![Message{role:"system".into(),content:"s".into()},Message{role:"user".into(),content:"old".into()},Message{role:"user".into(),content:"new".into()}];let body=b.body("local-model",&rows).unwrap();assert!(body["messages"][0]["images"].is_null());assert!(body["messages"][1]["images"].is_null());assert_eq!(body["messages"][2]["images"][0].as_str(),Some("/9j/AQ=="));assert!(b.body("model:cloud",&rows).is_err());}
 #[test]fn rejects_unsupported(){assert!(!supports_vision(&json!({"capabilities":["completion"]})));assert!(!supports_vision(&json!({"capabilities":["vision"],"remote_host":"example"})));assert!(supports_vision(&json!({"capabilities":["vision"]})));assert!(Vision::new(vec![b"text".to_vec()]).is_err());assert!(Vision::new(vec![]).is_err());}
}
