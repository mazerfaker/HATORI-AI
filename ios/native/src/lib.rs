use hatori_ai::{core, inference::Message};
use std::{ffi::CString,os::raw::c_char,ptr};

fn prepare(bytes:&[u8])->Result<String,String>{
 if bytes.len()>262144{return Err("Input exceeds limit".into());}
 let history:Vec<Message>=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
 if history.is_empty()||history.len()>32||history.iter().any(|m|!matches!(m.role.as_str(),"user"|"assistant")){return Err("Invalid history".into());}
 let query=history.last().filter(|m|m.role=="user").ok_or("Missing user message")?.content.clone();
 if query.trim().is_empty()||query.chars().count()>12000{return Err("Invalid user message".into());}
 let req=core::Request{model:"ios-preview".into(),mode:"Companion".into(),traits:[90,80,90,90,90,80,70],query,history,memories:vec![],profile:Default::default(),mood:None,dynamic_style:true,embedding_model:None};
 serde_json::to_string(&core::prepare(&req,&[])).map_err(|e|e.to_string())
}
fn owned(s:String)->*mut c_char{CString::new(s).map(CString::into_raw).unwrap_or(ptr::null_mut())}
#[no_mangle]
pub extern "C" fn hatoriel_version()->*mut c_char{owned(env!("CARGO_PKG_VERSION").into())}
/// Caller must supply a readable buffer of len bytes for this call.
#[no_mangle]
pub unsafe extern "C" fn hatoriel_prepare(data:*const u8,len:usize)->*mut c_char{
 let result=std::panic::catch_unwind(||{
 if data.is_null()||len==0||len>262144{return Err("Invalid input buffer".into());}
 prepare(unsafe{std::slice::from_raw_parts(data,len)})
 });
 owned(match result{Ok(Ok(s))=>s,Ok(Err(e))=>serde_json::json!({"error":e}).to_string(),Err(_)=>serde_json::json!({"error":"Native core failure"}).to_string()})
}
/// Pointer must be null or returned by this library, freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn hatoriel_free(value:*mut c_char){if !value.is_null(){drop(unsafe{CString::from_raw(value)});}}
#[cfg(test)]mod tests{use super::*;
 #[test]fn unicode(){let s=prepare("[{\"role\":\"user\",\"content\":\"Привіт 🌹\"}]".as_bytes()).unwrap();let v:Vec<Message>=serde_json::from_str(&s).unwrap();assert_eq!(v[0].role,"system");assert_eq!(v.last().unwrap().content,"Привіт 🌹");}
 #[test]fn invalid(){assert!(prepare(b"[]").is_err());assert!(prepare(b"[{\"role\":\"system\",\"content\":\"x\"}]").is_err());}
 #[test]fn ownership(){unsafe{let p=hatoriel_version();assert!(!p.is_null());assert_eq!(std::ffi::CStr::from_ptr(p).to_str().unwrap(),env!("CARGO_PKG_VERSION"));hatoriel_free(p);hatoriel_free(ptr::null_mut());}}
}

#[no_mangle]
pub unsafe extern "C" fn hatoriel_prepare_context(data:*const u8,len:usize)->*mut c_char{
 let result=std::panic::catch_unwind(||->Result<String,String>{
 if data.is_null()||len==0||len>1024*1024{return Err("Invalid context buffer".into());}
 let output=hatori_ai::mobile_context::prepare_json(unsafe{std::slice::from_raw_parts(data,len)},hatori_ai::emotion::now())?;
 serde_json::to_string(&output).map_err(|e|e.to_string())
 });
 owned(match result{Ok(Ok(s))=>s,Ok(Err(e))=>serde_json::json!({"error":e}).to_string(),Err(_)=>serde_json::json!({"error":"Native core failure"}).to_string()})
}

#[no_mangle]
pub unsafe extern "C" fn hatoriel_suggest_memory(data:*const u8,len:usize)->*mut c_char{
 let result=std::panic::catch_unwind(||->Result<String,String>{
 if data.is_null()||len==0||len>2000{return Ok("null".into());}
 let input=std::str::from_utf8(unsafe{std::slice::from_raw_parts(data,len)}).map_err(|e|e.to_string())?;
 Ok(match hatori_ai::memory_suggest::suggest(input){Some(m)=>serde_json::json!({"content":m.content,"category":m.category,"importance":m.importance}).to_string(),None=>"null".into()})
 });
 owned(match result{Ok(Ok(s))=>s,_=>"null".into()})
}
#[cfg(test)]mod suggestion_tests{use super::*;
 #[test]fn suggestion_utf8(){let s="Я люблю гітару";unsafe{let p=hatoriel_suggest_memory(s.as_ptr(),s.len());let v:serde_json::Value=serde_json::from_str(std::ffi::CStr::from_ptr(p).to_str().unwrap()).unwrap();assert_eq!(v["category"],"Interests");hatoriel_free(p);}}
 #[test]fn no_suggestion(){let s="Привіт";unsafe{let p=hatoriel_suggest_memory(s.as_ptr(),s.len());assert_eq!(std::ffi::CStr::from_ptr(p).to_str().unwrap(),"null");hatoriel_free(p);}}
}

#[no_mangle]
pub unsafe extern "C" fn hatoriel_sync(data:*const u8,len:usize)->*mut c_char{
 let result=std::panic::catch_unwind(||->Result<String,String>{
 if data.is_null()||len==0||len>3*1024*1024{return Err("Invalid sync buffer".into());}
 let input=std::str::from_utf8(unsafe{std::slice::from_raw_parts(data,len)}).map_err(|_|"Invalid UTF-8")?;
 hatori_ai::sync::command(input)
 });
 owned(match result{Ok(Ok(s))=>s,Ok(Err(e))=>serde_json::json!({"error":e}).to_string(),Err(_)=>serde_json::json!({"error":"Sync core failure"}).to_string()})
}

#[no_mangle]
pub unsafe extern "C" fn hatoriel_suggest_summary(data:*const u8,len:usize)->*mut c_char{
 let result=std::panic::catch_unwind(||->Result<String,String>{
 if data.is_null()||len==0||len>1024*1024{return Ok("null".into());}
 hatori_ai::memory_suggest::summary_json(unsafe{std::slice::from_raw_parts(data,len)})
 });
 owned(match result{Ok(Ok(s))=>s,_=>"null".into()})
}
