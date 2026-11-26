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
