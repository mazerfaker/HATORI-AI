use crate::{adaptive,emotion,inference::{Backend,Message},memory_core,personality,semantic,style};
use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Request{
 pub model:String,pub mode:String,pub traits:[u8;7],pub query:String,
 pub history:Vec<Message>,pub memories:Vec<(String,u8,i64)>,
 pub profile:adaptive::Profile,pub mood:Option<emotion::State>,
 pub dynamic_style:bool,pub embedding_model:Option<String>,
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Event{Token(String),Done,Cancelled,Error(String)}
pub fn recent_context(history:&[Message],max_messages:usize,max_chars:usize)->Vec<Message>{
 let mut remaining=max_chars;let mut recent=Vec::new();
 for m in history.iter().rev().take(max_messages){
  if remaining==0{break;}
  if m.role!="user"&&m.role!="assistant"{continue;}
  let n=m.content.chars().count();
  if n>remaining&&!recent.is_empty(){break;}
  let mut item=m.clone();item.content=item.content.chars().take(remaining).collect();
  remaining=remaining.saturating_sub(n);recent.push(item);
 }
 recent.reverse();recent
}
pub fn prepare(req:&Request,memories:&[String])->Vec<Message>{
 let plan=req.dynamic_style.then(||style::plan(req.traits,&req.query,req.mood.as_ref(),&req.mode,&req.profile,&req.history));
 let traits=plan.map_or_else(||req.traits.map(|v|v.min(100)),|p|p.traits);
 let mut system=adaptive::context_prompt(personality::prompt(&req.mode,traits,memories),&req.profile);
 if let Some(p)=plan{system.push_str(&format!("\nResponse length: {}. Task context hint: {}. {} Explicit user requests take precedence over stylistic defaults. Task hints are fallible; continuation is not authorization for external actions.",p.length,p.task.label(),p.task.guidance()));}
 if let Some(mood)=&req.mood{system.push('\n');system.push_str(&mood.guidance());}
 let mut out=vec![Message{role:"system".into(),content:system}];
 out.extend(recent_context(&req.history,32,12000));out
}
/// Blocking API. Call from a worker, not a UI thread. Cancellation is cooperative.
pub fn run(backend:&dyn Backend,req:&Request,cancel:&dyn Fn()->bool,mut emit:impl FnMut(Event)->bool){
 run_with_multimodal(backend,req,None,0,crate::multimodal::DEFAULT_BUDGET,cancel,&mut emit);
}
/// Optional one-turn reference, never persisted in Request or memory selection.
pub fn run_with_reference(backend:&dyn Backend,req:&Request,reference:Option<&str>,cancel:&dyn Fn()->bool,mut emit:impl FnMut(Event)->bool){
 run_with_multimodal(backend,req,reference,0,crate::multimodal::DEFAULT_BUDGET,cancel,&mut emit);
}
pub fn run_with_multimodal(backend:&dyn Backend,req:&Request,reference:Option<&str>,images:usize,budget:usize,cancel:&dyn Fn()->bool,mut emit:impl FnMut(Event)->bool){
 if cancel(){emit(Event::Cancelled);return;}
 let memory_query=memory_core::continuation_query(&req.query,&req.history);
 let mut memories=memory_core::ranked(&memory_query,&req.memories);
 if let Some(model)=&req.embedding_model{
  if !memory_query.is_empty(){if let Ok(mut found)=semantic::retrieve(&memory_query,&req.memories,model){
   for m in memories{if !found.contains(&m){found.push(m);}}
   found.truncate(12);memories=found;
  }}
 }
 if cancel(){emit(Event::Cancelled);return;}
 let mut messages=prepare(req,&memories);
 if reference.is_some()||images>0{
  if req.query.chars().count()>crate::multimodal::MAX_QUERY{emit(Event::Error("Query exceeds 12000 characters".into()));return;}
  if let Err(e)=crate::multimodal::apply(&mut messages,reference,images,budget){emit(Event::Error(e));return;}
 }
 let mut disconnected=false;
 let mut output_bytes=0usize;let mut overflow=false;
 let result=backend.generate(&req.model,&messages,&mut |token|{
  if cancel(){return false;}output_bytes=output_bytes.saturating_add(token.len());if output_bytes>262144{overflow=true;return false;}let ok=emit(Event::Token(token));if !ok{disconnected=true;}ok
 });
 if disconnected{return;}
 emit(if cancel(){Event::Cancelled}else if overflow{Event::Error("Response exceeded 256 KiB limit".into())}else{match result{Ok(())=>Event::Done,Err(e)=>Event::Error(e)}});
}
#[cfg(test)]mod tests{
 use super::*;
 struct Mock;
 impl Backend for Mock{
  fn models(&self)->Result<Vec<String>,String>{Ok(vec!["mock".into()])}
  fn generate(&self,_:&str,m:&[Message],emit:&mut dyn FnMut(String)->bool)->Result<(),String>{assert_eq!(m[0].role,"system");if emit("ok".into()){Ok(())}else{Err("cancelled".into())}}
 }
 fn request()->Request{Request{model:"mock".into(),mode:"Companion".into(),traits:[50;7],query:"hi".into(),history:vec![Message{role:"user".into(),content:"hi".into()}],memories:vec![],profile:Default::default(),mood:None,dynamic_style:true,embedding_model:None}}
 #[test]fn events(){let mut out=vec![];run(&Mock,&request(),&||false,|e|{out.push(e);true});assert_eq!(out,vec![Event::Token("ok".into()),Event::Done]);}
 #[test]fn cancelled_before_start(){let mut out=vec![];run(&Mock,&request(),&||true,|e|{out.push(e);true});assert_eq!(out,vec![Event::Cancelled]);}
 #[test]fn oversized_response_is_rejected(){
  struct Huge;impl Backend for Huge{
   fn models(&self)->Result<Vec<String>,String>{Ok(vec![])}
   fn generate(&self,_:&str,_:&[Message],emit:&mut dyn FnMut(String)->bool)->Result<(),String>{let _=emit("x".repeat(262145));Ok(())}
  }
  let mut out=vec![];run(&Huge,&request(),&||false,|e|{out.push(e);true});assert_eq!(out,vec![Event::Error("Response exceeded 256 KiB limit".into())]);
 }
 #[test]fn reference_scope(){
  struct Inspect;impl Backend for Inspect{
   fn models(&self)->Result<Vec<String>,String>{Ok(vec![])}
   fn generate(&self,_:&str,m:&[Message],_:&mut dyn FnMut(String)->bool)->Result<(),String>{assert!(m[0].content.contains("untrusted"));assert!(m.last().unwrap().content.contains("secret excerpt"));Ok(())}
  }
  let req=request();let mut out=vec![];run_with_multimodal(&Inspect,&req,Some("secret excerpt"),1,24000,&||false,|e|{out.push(e);true});assert_eq!(out,vec![Event::Done]);assert!(!req.history[0].content.contains("secret excerpt"));
 }
 #[test]fn personality_shared_context(){let mut req=request();req.mode="Developer".into();req.traits=[0,80,90,90,90,80,70];req.query="серйозно".into();let m=prepare(&req,&[]);assert!(m[0].content.contains("sarcasm: 0"));assert!(m[0].content.contains("Avoid sarcasm"));assert!(m[0].content.contains("never invent build"));assert_eq!(req.traits[6],70);}
 #[test]fn adaptive_disabled_keeps_base(){let mut req=request();req.dynamic_style=false;req.traits=[90;7];req.query="error".into();let m=prepare(&req,&[]);assert!(m[0].content.contains("sarcasm: 90"));assert!(!m[0].content.contains("Task context hint:"));}
 #[test]fn unicode_budget(){let m=vec![Message{role:"user".into(),content:"абвг".into()}];assert_eq!(recent_context(&m,32,2)[0].content,"аб");}
}
