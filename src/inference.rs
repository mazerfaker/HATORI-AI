use serde::{Serialize,Deserialize};
use std::{io::{BufRead,BufReader},time::Duration};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Message { pub role:String,pub content:String }
pub trait Backend: Send+Sync {
 fn generate(&self,model:&str,messages:&[Message],emit:&mut dyn FnMut(String)->bool)->Result<(),String>;
 fn models(&self)->Result<Vec<String>,String>;
}
pub struct Ollama;
impl Backend for Ollama {
 fn generate(&self,model:&str,messages:&[Message],emit:&mut dyn FnMut(String)->bool)->Result<(),String>{
  (||->Result<(),Box<dyn std::error::Error>>{
   let client=reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(4)).timeout(Duration::from_secs(300)).build()?;
   let response=client.post("http://127.0.0.1:11434/api/chat").json(&serde_json::json!({"model":model,"messages":messages,"stream":true})).send()?.error_for_status()?;
   decode(BufReader::new(response),emit)
  })().map_err(|e|e.to_string())
 }
 fn models(&self)->Result<Vec<String>,String>{
  (||->Result<Vec<String>,Box<dyn std::error::Error>>{
   let v:serde_json::Value=reqwest::blocking::Client::builder().timeout(Duration::from_secs(5)).build()?.get("http://127.0.0.1:11434/api/tags").send()?.error_for_status()?.json()?;
   Ok(v["models"].as_array().into_iter().flatten().filter_map(|m|m["name"].as_str().map(str::to_owned)).collect())
  })().map_err(|e|e.to_string())
 }
}
fn decode(reader:impl BufRead,emit:&mut dyn FnMut(String)->bool)->Result<(),Box<dyn std::error::Error>>{
 for line in reader.lines(){let line=line?;if line.trim().is_empty(){continue;}let v:serde_json::Value=serde_json::from_str(&line)?;
 if let Some(e)=v["error"].as_str(){return Err(e.to_owned().into());}
 if let Some(s)=v["message"]["content"].as_str(){if !emit(s.into()){return Err("Generation cancelled".into());}}
 if v["done"].as_bool()==Some(true){return Ok(());}
 } Err("Stream ended prematurely".into())
}
#[cfg(test)] mod tests{
 use super::*;
 #[test] fn stream(){let mut out=String::new();decode(std::io::Cursor::new(b"{\"message\":{\"content\":\"Hi\"}}\n{\"done\":true}\n"),&mut |s|{out.push_str(&s);true}).unwrap();assert_eq!(out,"Hi");}
 #[test] fn incomplete(){assert!(decode(std::io::Cursor::new(b"{}\n"),&mut |_|true).is_err());}
}

#[cfg(test)]
#[test]fn cancellation_stops_stream(){let mut calls=0;let result=decode(std::io::Cursor::new("{\"message\":{\"content\":\"Hi\"}}\n{\"done\":true}\n"),&mut |_|{calls+=1;false});assert!(result.is_err());assert_eq!(calls,1);}
