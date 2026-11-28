use crate::{adaptive,core,emotion,inference::Message,memory_core,style};
use serde::{Deserialize,Serialize};

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(default)]
pub struct Options {
 pub mode:String,pub traits:[u8;7],pub dynamic_style:bool,pub emotion_enabled:bool,pub mood:emotion::State,
}
impl Default for Options {
 fn default()->Self{Self{mode:"Companion".into(),traits:crate::personality::DEFAULT_TRAITS,dynamic_style:true,emotion_enabled:false,mood:Default::default()}}
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Memory {pub id:i64,pub content:String,pub importance:u8}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Input {
 pub history:Vec<Message>,
 #[serde(default)]pub options:Options,
 #[serde(default)]pub memories:Vec<Memory>,
 #[serde(default)]pub profile:adaptive::Profile,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Output {pub messages:Vec<Message>,pub mood:emotion::State,pub effective_traits:[u8;7]}
pub fn prepare(mut input:Input,now:u64)->Result<Output,String>{
 if input.history.is_empty()||input.history.len()>32||input.history.iter().any(|m|!matches!(m.role.as_str(),"user"|"assistant")){return Err("Invalid history".into());}
 let query=input.history.last().filter(|m|m.role=="user").ok_or("Missing user message")?.content.clone();
 if query.trim().is_empty()||query.chars().count()>12000{return Err("Invalid user message".into());}
 if !crate::personality::MODES.contains(&input.options.mode.as_str()){return Err("Invalid personality mode".into());}
 if input.memories.len()>100{return Err("Too many memories".into());}
 let traits=input.options.traits.map(|x|x.min(100));
 let mut mood=input.options.mood;
 mood.intensity=mood.intensity.min(100);
 mood.updated=mood.updated.min(now);
 if input.options.emotion_enabled{mood.observe(&query,now,traits[5]);}else{mood=Default::default();}
 let context_mood=input.options.emotion_enabled.then(||mood.clone());
 for field in [&mut input.profile.interests,&mut input.profile.communication,&mut input.profile.goals]{*field=field.chars().take(1000).collect();}
 let memories:Vec<_>=input.memories.into_iter().filter(|m|!m.content.trim().is_empty()&&m.content.chars().count()<=500).map(|m|(m.content,m.importance.min(100),m.id)).collect();
 let memory_query=memory_core::continuation_query(&query,&input.history);
 let relevant=memory_core::ranked(&memory_query,&memories);
 let effective_traits=if input.options.dynamic_style{style::plan(traits,&query,context_mood.as_ref(),&input.options.mode,&input.profile,&input.history).traits}else{traits};
 let req=core::Request{model:"mobile".into(),mode:input.options.mode,traits,query,history:input.history,memories,profile:input.profile,mood:context_mood,dynamic_style:input.options.dynamic_style,embedding_model:None};
 Ok(Output{messages:core::prepare(&req,&relevant),mood,effective_traits})
}
pub fn prepare_json(bytes:&[u8],now:u64)->Result<Output,String>{
 if bytes.len()>1024*1024{return Err("Context exceeds limit".into());}
 let input:Input=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
 prepare(input,now)
}
#[cfg(test)]mod tests{
 use super::*;
 fn input(query:&str)->Input{Input{history:vec![Message{role:"user".into(),content:query.into()}],options:Default::default(),memories:vec![],profile:Default::default()}}
 #[test]fn serious_style(){let out=prepare(input("серйозно"),100).unwrap();assert_eq!(out.effective_traits[6],0);}
 #[test]fn mood_opt_in(){let mut i=input("дякую");i.options.emotion_enabled=true;assert_eq!(prepare(i,100).unwrap().mood.mood,emotion::Mood::Joy);assert_eq!(prepare(input("дякую"),100).unwrap().mood.mood,emotion::Mood::Calm);}
 #[test]fn relevant_only(){let mut i=input("guitar");i.memories=vec![Memory{id:1,content:"guitar rock".into(),importance:80},Memory{id:2,content:"unrelated secret".into(),importance:100}];let o=prepare(i,100).unwrap();assert!(o.messages[0].content.contains("guitar rock"));assert!(!o.messages[0].content.contains("unrelated secret"));}
 #[test]fn adaptive_mobile_matches_prompt(){let mut i=input("далі");i.history.insert(0,Message{role:"user".into(),content:"виправ код Rust".into()});i.options.traits=[50;7];i.profile.communication="коротко".into();let o=prepare(i,100).unwrap();assert_eq!(o.effective_traits[0],85);assert!(o.messages[0].content.contains("Task context hint: Technical"));assert!(o.messages[0].content.contains("Response length: Concise"));}
 #[test]fn invalid_mode(){let mut i=input("hi");i.options.mode="invalid".into();assert!(prepare(i,100).is_err());}
 #[test]fn legacy_defaults(){let o=prepare_json(br#"{"history":[{"role":"user","content":"hello"}],"profile":{},"memories":[]}"#,100).unwrap();assert_eq!(o.messages.last().unwrap().content,"hello");assert_eq!(o.mood.intensity,0);}
}

#[cfg(test)]#[test]fn mobile_project_continuity(){
 let input=Input{history:vec![Message{role:"user".into(),content:"Hatoriel Rust".into()},Message{role:"assistant".into(),content:"unrelated secret".into()},Message{role:"user".into(),content:"далі".into()}],options:Default::default(),memories:vec![Memory{id:1,content:"Hatoriel project".into(),importance:90},Memory{id:2,content:"unrelated secret".into(),importance:100}],profile:Default::default()};
 let out=prepare(input,100).unwrap();assert!(out.messages[0].content.contains("Hatoriel project"));assert!(!out.messages[0].content.contains("unrelated secret"));
}
