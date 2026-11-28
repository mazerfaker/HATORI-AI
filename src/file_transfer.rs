//! One-time, bounded immutable file snapshot. No remote filesystem paths.
use std::{io::Read,path::Path,sync::{Arc,Mutex},time::{Duration,Instant}};
use sha2::{Digest,Sha256};
use crate::file_access::{Scopes,ordinary};
pub const MAX_BYTES:usize=8*1024*1024;
const TTL:Duration=Duration::from_secs(120);
pub struct Payload{pub name:String,pub bytes:Vec<u8>,pub digest:String}
struct Offer{code:String,expires:Instant,payload:Payload}
#[derive(Clone,Default)]
pub struct Store(Arc<Mutex<Option<Offer>>>);
fn hex(bytes:&[u8])->String{bytes.iter().map(|b|format!("{b:02x}")).collect()}
fn same(a:&str,b:&str)->bool{a.len()==b.len()&&a.bytes().zip(b.bytes()).fold(0u8,|x,(a,b)|x|(a^b))==0}
pub fn safe_name(name:&str)->String{
 let s:String=name.chars().filter(|c|!c.is_control()&&!matches!(c,'/'|'\\'|':'|'?'|'*'|'<'|'>'|'|'|'"')).take(160).collect();
 let s=s.trim_matches(|c:char|c=='.'||c.is_whitespace());
 if s.is_empty(){"attachment".into()}else{s.into()}
}
impl Store{
 pub fn clear(&self){if let Ok(mut slot)=self.0.lock(){*slot=None;}}
 pub fn info(&self)->Option<(String,String,usize,u64)>{
  let mut slot=self.0.lock().ok()?;
  if slot.as_ref().is_some_and(|o|Instant::now()>=o.expires){*slot=None;}
  let o=slot.as_ref()?;Some((o.code.clone(),o.payload.name.clone(),o.payload.bytes.len(),o.expires.saturating_duration_since(Instant::now()).as_secs()))
 }
 pub fn prepare(&self,scopes:&Scopes,path:&Path,deadline:Instant)->Result<(),String>{
  if Instant::now()>=deadline{return Err("Permission session expired".into());}
  let path=scopes.checked(path)?;
  let before=std::fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
  if !ordinary(&before)||!before.is_file()||before.len()>MAX_BYTES as u64{return Err("Transfer requires a regular file up to 8 MiB".into());}
  let file=std::fs::File::open(&path).map_err(|e|e.to_string())?;
  let meta=file.metadata().map_err(|e|e.to_string())?;
  if !ordinary(&meta)||!meta.is_file()||meta.len()!=before.len()||meta.len()>MAX_BYTES as u64{return Err("File changed".into());}
  scopes.checked(&path)?;
  let mut bytes=Vec::new();file.take(MAX_BYTES as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
  if bytes.len()!=before.len() as usize||bytes.len()>MAX_BYTES{return Err("File changed during reading".into());}
  scopes.checked(&path)?;
  let after=std::fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
  if !ordinary(&after)||after.len()!=before.len()||after.modified().ok()!=before.modified().ok(){return Err("File changed during reading".into());}
  let name=safe_name(&path.file_name().unwrap_or_default().to_string_lossy());
  self.put_until(name,bytes,Instant::now(),deadline)
 }
 #[cfg(test)]
 fn put(&self,name:String,bytes:Vec<u8>,now:Instant)->Result<(),String>{self.put_until(name,bytes,now,now+TTL)}
 fn put_until(&self,name:String,bytes:Vec<u8>,now:Instant,deadline:Instant)->Result<(),String>{
  if now>=deadline{return Err("Permission session expired".into());}
  if bytes.len()>MAX_BYTES{return Err("File exceeds limit".into());}
  let mut id=[0u8;16];getrandom::getrandom(&mut id).map_err(|e|e.to_string())?;
  let digest=hex(&Sha256::digest(&bytes));
  let mut slot=self.0.lock().map_err(|_|"Transfer store unavailable")?;
  *slot=Some(Offer{code:hex(&id),expires:(now+TTL).min(deadline),payload:Payload{name,bytes,digest}});Ok(())
 }
 pub fn take(&self,code:&str,now:Instant)->Option<Payload>{
  let mut slot=self.0.lock().ok()?;
  if slot.as_ref().is_some_and(|o|now>=o.expires){*slot=None;return None;}
  if !slot.as_ref().is_some_and(|o|same(&o.code,code)){return None;}
  slot.take().map(|o|o.payload)
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn one_time_and_digest(){let s=Store::default();let n=Instant::now();s.put("a.txt".into(),b"abc".to_vec(),n).unwrap();let code=s.info().unwrap().0;assert!(s.take("bad",n).is_none());let p=s.take(&code,n).unwrap();assert_eq!(p.bytes,b"abc");assert_eq!(p.digest,"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");assert!(s.take(&code,n).is_none());}
 #[test]fn expire_and_revoke(){let s=Store::default();let n=Instant::now();s.put("a".into(),vec![],n).unwrap();let c=s.info().unwrap().0;assert!(s.take(&c,n+TTL).is_none());s.put("b".into(),vec![1],n).unwrap();let c=s.info().unwrap().0;s.clear();assert!(s.take(&c,n).is_none());}
 #[test]fn offer_cannot_outlive_permission(){let s=Store::default();let n=Instant::now();let deadline=n+Duration::from_secs(3);s.put_until("a".into(),vec![1],n,deadline).unwrap();let code=s.info().unwrap().0;assert!(s.take(&code,deadline).is_none());assert!(s.put_until("b".into(),vec![1],deadline,deadline).is_err());}
 #[test]fn names_and_limit(){assert_eq!(safe_name("../a:b.txt"),"ab.txt");assert_eq!(safe_name("..."),"attachment");assert!(Store::default().put("x".into(),vec![0;MAX_BYTES+1],Instant::now()).is_err());}
}
