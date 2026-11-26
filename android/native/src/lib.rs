use hatori_ai::{core, inference::Message};
use jni::{JNIEnv, objects::{JClass,JString},sys::jstring};
use std::ptr::null_mut;

#[no_mangle]
pub extern "system" fn Java_com_hatoriel_app_NativeCore_version(env:JNIEnv,_class:JClass)->jstring{
 env.new_string(env!("CARGO_PKG_VERSION")).map(|s|s.into_raw()).unwrap_or(null_mut())
}
#[no_mangle]
pub extern "system" fn Java_com_hatoriel_app_NativeCore_prepare(mut env:JNIEnv,_class:JClass,input:JString)->jstring{
 let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{
 let query:String=env.get_string(&input).map_err(|e|e.to_string())?.into();
 if query.trim().is_empty()||query.chars().count()>12000{return Err("Enter 1–12000 characters".to_string());}
 let req=core::Request{model:"preview".into(),mode:"Companion".into(),traits:[90,80,90,90,90,80,70],query:query.clone(),history:vec![Message{role:"user".into(),content:query}],memories:vec![],profile:Default::default(),mood:None,dynamic_style:true,embedding_model:None};
 serde_json::to_string(&core::prepare(&req,&[])).map_err(|e|e.to_string())
 }));
 let value=match result{Ok(Ok(s))=>s,Ok(Err(e))=>serde_json::json!({"error":e}).to_string(),Err(_)=>"{\"error\":\"Native core failure\"}".into()};
 env.new_string(value).map(|s|s.into_raw()).unwrap_or(null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_hatoriel_app_NativeCore_prepareHistory(mut env:JNIEnv,_class:JClass,input:JString)->jstring{
 let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<String,String>{
 let json:String=env.get_string(&input).map_err(|e|e.to_string())?.into();
 if json.len()>262144{return Err("History exceeds limit".into());}
 let history:Vec<Message>=serde_json::from_str(&json).map_err(|e|e.to_string())?;
 if history.len()>32{return Err("Too many messages".into());}
 let query=history.last().filter(|m|m.role=="user").ok_or("Missing user message")?.content.clone();
 if query.trim().is_empty()||query.chars().count()>12000{return Err("Invalid user message".into());}
 let req=core::Request{model:"mobile".into(),mode:"Companion".into(),traits:[90,80,90,90,90,80,70],query,history,memories:vec![],profile:Default::default(),mood:None,dynamic_style:true,embedding_model:None};
 serde_json::to_string(&core::prepare(&req,&[])).map_err(|e|e.to_string())
 }));
 let value=match result{Ok(Ok(s))=>s,Ok(Err(e))=>serde_json::json!({"error":e}).to_string(),Err(_)=>"{\"error\":\"Native core failure\"}".into()};
 env.new_string(value).map(|s|s.into_raw()).unwrap_or(null_mut())
}

fn prepare_memory_json(json:&str)->Result<String,String>{
 let output=hatori_ai::mobile_context::prepare_json(json.as_bytes(),hatori_ai::emotion::now())?;
 serde_json::to_string(&output.messages).map_err(|e|e.to_string())
}
#[no_mangle]
pub extern "system" fn Java_com_hatoriel_app_NativeCore_prepareMemory(mut env:JNIEnv,_class:JClass,input:JString)->jstring{
 let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<String,String>{let json:String=env.get_string(&input).map_err(|e|e.to_string())?.into();prepare_memory_json(&json)}));
 let value=match result{Ok(Ok(s))=>s,Ok(Err(e))=>serde_json::json!({"error":e}).to_string(),Err(_)=>serde_json::json!({"error":"Native core failure"}).to_string()};
 env.new_string(value).map(|s|s.into_raw()).unwrap_or(null_mut())
}
#[no_mangle]
pub extern "system" fn Java_com_hatoriel_app_NativeCore_suggestMemory(mut env:JNIEnv,_class:JClass,input:JString)->jstring{
 let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<String,String>{let query:String=env.get_string(&input).map_err(|e|e.to_string())?.into();Ok(match hatori_ai::memory_suggest::suggest(&query){Some(m)=>serde_json::json!({"content":m.content,"category":m.category,"importance":m.importance}).to_string(),None=>"null".into()})}));
 let value=match result{Ok(Ok(s))=>s,_=>"null".into()};env.new_string(value).map(|s|s.into_raw()).unwrap_or(null_mut())
}
#[cfg(test)]mod memory_tests{use super::*;#[test]fn selects_relevant(){let input=serde_json::json!({"history":[{"role":"user","content":"guitar"}],"memories":[{"id":1,"content":"guitar rock","importance":90},{"id":2,"content":"unrelated","importance":100}],"profile":{}});let s=prepare_memory_json(&input.to_string()).unwrap();assert!(s.contains("guitar rock"));assert!(!s.contains("unrelated"));}}

#[no_mangle]
pub extern "system" fn Java_com_hatoriel_app_NativeCore_sync(mut env:JNIEnv,_class:JClass,input:JString)->jstring{
 let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<String,String>{
 let json:String=env.get_string(&input).map_err(|e|e.to_string())?.into();
 hatori_ai::sync::command(&json)
 }));
 let value=match result{Ok(Ok(s))=>s,Ok(Err(e))=>serde_json::json!({"error":e}).to_string(),Err(_)=>serde_json::json!({"error":"Sync core failure"}).to_string()};
 env.new_string(value).map(|s|s.into_raw()).unwrap_or(null_mut())
}
