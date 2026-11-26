use crate::inference::{Backend,Message};
use std::{io::{BufRead,BufReader},time::Duration};
pub struct Llama;
impl Backend for Llama {
 fn generate(&self,model:&str,messages:&[Message],emit:&mut dyn FnMut(String))->Result<(),String>{
 (||->Result<(),Box<dyn std::error::Error>>{
 let client=reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(4)).timeout(Duration::from_secs(300)).build()?;
 let response=client.post("http://127.0.0.1:8080/v1/chat/completions").json(&serde_json::json!({"model":model,"messages":messages,"stream":true})).send()?.error_for_status()?;
 decode(BufReader::new(response),emit)
 })().map_err(|e|e.to_string())
 }
 fn models(&self)->Result<Vec<String>,String>{
 (||->Result<Vec<String>,Box<dyn std::error::Error>>{
 let v:serde_json::Value=reqwest::blocking::Client::builder().timeout(Duration::from_secs(5)).build()?.get("http://127.0.0.1:8080/v1/models").send()?.error_for_status()?.json()?;
 Ok(v["data"].as_array().into_iter().flatten().filter_map(|m|m["id"].as_str().map(str::to_owned)).collect())
 })().map_err(|e|e.to_string())
 }
}
fn decode(reader:impl BufRead,emit:&mut dyn FnMut(String))->Result<(),Box<dyn std::error::Error>>{
 for line in reader.lines(){let line=line?;let Some(data)=line.strip_prefix("data:") else{continue;};let data=data.trim();if data=="[DONE]"{return Ok(());}
 let v:serde_json::Value=serde_json::from_str(data)?;
 if !v["error"].is_null(){return Err(v["error"].to_string().into());}
 if let Some(s)=v["choices"][0]["delta"]["content"].as_str(){emit(s.to_owned());}
 } Err("Incomplete llama-server stream".into())
}
#[cfg(test)]mod tests{use super::*;#[test]fn stream(){let mut out=String::new();decode(std::io::Cursor::new(b"data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\ndata: [DONE]\n"),&mut|s|out.push_str(&s)).unwrap();assert_eq!(out,"Hi");}#[test]fn truncated(){assert!(decode(std::io::Cursor::new(b""),&mut |_|{}).is_err());}}
