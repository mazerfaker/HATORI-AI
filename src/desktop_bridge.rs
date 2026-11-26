use std::{io::Read,sync::{Arc,atomic::{AtomicBool,Ordering}},thread::{self,JoinHandle},time::Duration};
use tiny_http::{Server,Request,Response,Header,Method,StatusCode};
use serde_json::{Value,json};
const LIMIT:usize=256*1024;
#[derive(Default)]
pub struct Bridge{worker:Option<JoinHandle<()>>,stop:Option<Arc<AtomicBool>>,pub token:String}
impl Bridge{
 pub fn running(&self)->bool{self.worker.as_ref().is_some_and(|w|!w.is_finished())}
 pub fn start(&mut self,model:String)->Result<(),String>{
  if self.running(){return Err("Bridge already running or stopping".into());}
  if model.trim().is_empty(){return Err("Select an Ollama model".into());}
  let mut key=[0u8;32];getrandom::getrandom(&mut key).map_err(|e|e.to_string())?;
  let token=key.iter().map(|b|format!("{b:02x}")).collect::<String>();
  let server=Server::http("127.0.0.1:17845").map_err(|e|e.to_string())?;
  let client=reqwest::blocking::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).connect_timeout(Duration::from_secs(4)).timeout(Duration::from_secs(240)).build().map_err(|e|e.to_string())?;
  let stop=Arc::new(AtomicBool::new(false));let flag=stop.clone();let secret=token.clone();
  self.worker=Some(thread::spawn(move||{
   while !flag.load(Ordering::Relaxed){
    match server.recv_timeout(Duration::from_millis(200)){
     Ok(Some(req))=>handle(req,&client,&model,&secret,&flag),
     Ok(None)=>{},Err(_)=>break,
    }
   }
  }));self.stop=Some(stop);self.token=token;Ok(())
 }
 pub fn stop(&mut self){if let Some(s)=&self.stop{s.store(true,Ordering::Relaxed);}self.token.clear();}
}
impl Drop for Bridge{fn drop(&mut self){self.stop();}}
fn status_payload(model:&str)->Value{json!({"service":"hatoriel-desktop","protocol":1,"model":model,"backend":"ollama"})}
fn same(a:&str,b:&str)->bool{if a.len()!=b.len(){return false;}a.bytes().zip(b.bytes()).fold(0u8,|v,(x,y)|v|(x^y))==0}
fn reply(req:Request,status:u16,message:&str){let _=req.respond(Response::from_string(json!({"error":message}).to_string()).with_status_code(StatusCode(status)));}
struct LimitedStream{source:reqwest::blocking::Response,stop:Arc<AtomicBool>,remaining:usize}
impl Read for LimitedStream{fn read(&mut self,buf:&mut[u8])->std::io::Result<usize>{
 if self.stop.load(Ordering::Relaxed)||self.remaining==0{return Ok(0);}
 let cap=buf.len().min(self.remaining);let n=self.source.read(&mut buf[..cap])?;self.remaining-=n;Ok(n)
}}
fn handle(mut req:Request,client:&reqwest::blocking::Client,model:&str,secret:&str,stop:&Arc<AtomicBool>){
 if stop.load(Ordering::Relaxed){reply(req,503,"Bridge stopped");return;}
 let expected=format!("Bearer {secret}");
 let authorized=req.headers().iter().any(|h|h.field.equiv("Authorization")&&same(h.value.as_str(),&expected));
 if !authorized{reply(req,401,"Unauthorized");return;}
 if req.method()==&Method::Get&&req.url()=="/api/status"{
  let header=Header::from_bytes(&b"Content-Type"[..],&b"application/json"[..]).expect("static header");
  let cache=Header::from_bytes(&b"Cache-Control"[..],&b"no-store"[..]).expect("static header");
  let _=req.respond(Response::from_string(status_payload(model).to_string()).with_header(header).with_header(cache));return;
 }
 if req.method()!=&Method::Post||req.url()!="/api/chat"{reply(req,404,"Unsupported endpoint");return;}
 let Some(len)=req.body_length() else{reply(req,411,"Content-Length required");return;};
 if len==0||len>LIMIT{reply(req,413,"Request too large");return;}
 let mut bytes=Vec::new();
 if req.as_reader().take((LIMIT+1) as u64).read_to_end(&mut bytes).is_err()||bytes.len()!=len{reply(req,400,"Invalid body");return;}
 let Ok(v)=serde_json::from_slice::<Value>(&bytes) else{reply(req,400,"Invalid JSON");return;};
 if v["model"].as_str()!=Some(model){reply(req,400,"Select the same model as the desktop bridge");return;}
 let Some(messages)=v["messages"].as_array() else{reply(req,400,"Missing messages");return;};
 if messages.is_empty()||messages.len()>33||messages.iter().any(|m|!matches!(m["role"].as_str(),Some("system"|"user"|"assistant"))||m["content"].as_str().is_none()){
  reply(req,400,"Invalid messages");return;
 }
 if stop.load(Ordering::Relaxed){reply(req,503,"Bridge stopped");return;}
 let messages:Vec<_>=messages.iter().map(|m|json!({"role":m["role"],"content":m["content"]})).collect();
 let upstream=client.post("http://127.0.0.1:11434/api/chat").json(&json!({"model":model,"messages":messages,"stream":true,"options":{"num_predict":2048}})).send();
 match upstream{
  Ok(source) if source.status().is_success()=>{
   let header=Header::from_bytes(&b"Content-Type"[..],&b"application/x-ndjson"[..]).expect("static header");
   let reader=LimitedStream{source,stop:stop.clone(),remaining:2*1024*1024};
   let _=req.respond(Response::new(StatusCode(200),vec![header],reader,None,None));
  },_=>reply(req,502,"Local Ollama unavailable or rejected request")
 }
}
#[cfg(test)]mod tests{use super::*;#[test]fn auth(){assert!(same("abc","abc"));assert!(!same("abc","abd"));assert!(!same("abc","ab"));}#[test]fn status_exposes_only_protocol_and_model(){let v=status_payload("test-model");assert_eq!(v["model"],"test-model");assert_eq!(v["protocol"],1);assert_eq!(v.as_object().unwrap().len(),4);}}
