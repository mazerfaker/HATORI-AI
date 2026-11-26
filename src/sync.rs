use crate::{adaptive::Profile,mobile_context::Options,memory_core::CATEGORIES,memory_suggest::duplicate};
use serde::{Serialize,Deserialize};
use aes_gcm::{Aes256Gcm,KeyInit,Nonce,aead::{Aead,Payload}};
use argon2::{Argon2,Algorithm,Version,Params};
use base64::{Engine,engine::general_purpose::STANDARD};
use zeroize::Zeroizing;
const MAGIC:&[u8]=b"HATORIEL-SYNC-1";
const MAX:usize=1024*1024;
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Record{pub content:String,pub category:String,pub importance:u8}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Snapshot{pub version:u32,pub records:Vec<Record>,pub profile:Profile,pub options:Options}
impl Snapshot{
 pub fn validate(&self)->Result<(),String>{
  if self.version!=1||self.records.len()>100{return Err("Unsupported snapshot version or record limit".into());}
  if self.records.iter().any(|r|r.content.trim().is_empty()||r.content.chars().count()>500||!CATEGORIES.contains(&r.category.as_str())||r.importance>100){return Err("Invalid memory record".into());}
  if [&self.profile.interests,&self.profile.communication,&self.profile.goals].iter().any(|s|s.chars().count()>1000)||!matches!(self.options.mode.as_str(),"Companion"|"Assistant"|"Developer"|"Researcher")||self.options.traits.iter().any(|x|*x>100){return Err("Invalid profile or traits".into());}
  Ok(())
 }
 fn clean(mut self)->Self{self.options.mood=Default::default();self}
}
fn key(password:&str,salt:&[u8])->Result<Zeroizing<[u8;32]>,String>{
 if password.chars().count()<12||password.len()>1024{return Err("Use a passphrase of 12 or more characters".into());}
 let mut out=Zeroizing::new([0u8;32]);
 Argon2::new(Algorithm::Argon2id,Version::V0x13,Params::new(32768,3,1,Some(32)).map_err(|_|"KDF parameters")?).hash_password_into(password.as_bytes(),salt,&mut *out).map_err(|_|"Key derivation failed")?;Ok(out)
}
pub fn seal(snapshot:Snapshot,password:&str)->Result<String,String>{
 snapshot.validate()?;let data=Zeroizing::new(serde_json::to_vec(&snapshot.clean()).map_err(|e|e.to_string())?);
 if data.len()>MAX{return Err("Snapshot too large".into());}
 let mut salt=[0u8;16];let mut nonce=[0u8;12];getrandom::getrandom(&mut salt).map_err(|_|"Random generator failed")?;getrandom::getrandom(&mut nonce).map_err(|_|"Random generator failed")?;
 let k=key(password,&salt)?;let cipher=Aes256Gcm::new_from_slice(&*k).map_err(|_|"Invalid key")?;
 let encrypted=cipher.encrypt(Nonce::from_slice(&nonce),Payload{msg:&data,aad:MAGIC}).map_err(|_|"Encryption failed")?;
 let mut bytes=MAGIC.to_vec();bytes.extend(salt);bytes.extend(nonce);bytes.extend(encrypted);Ok(STANDARD.encode(bytes))
}
pub fn open(encoded:&str,password:&str)->Result<Snapshot,String>{
 let encoded=encoded.trim();if encoded.len()>2*MAX{return Err("Package too large".into());}
 let bytes=STANDARD.decode(encoded).map_err(|_|"Invalid package encoding")?;
 let n=MAGIC.len();if !bytes.starts_with(MAGIC)||bytes.len()<n+44||bytes.len()>MAX+n+44{return Err("Invalid package header".into());}
 let k=key(password,&bytes[n..n+16])?;let cipher=Aes256Gcm::new_from_slice(&*k).map_err(|_|"Invalid key")?;
 let data=Zeroizing::new(cipher.decrypt(Nonce::from_slice(&bytes[n+16..n+28]),Payload{msg:&bytes[n+28..],aad:MAGIC}).map_err(|_|"Wrong passphrase or damaged package")?);
 let snapshot:Snapshot=serde_json::from_slice(&data).map_err(|_|"Invalid snapshot")?;snapshot.validate()?;Ok(snapshot.clean())
}
#[derive(Serialize)]
pub struct Plan{pub additions:Vec<Record>,pub duplicates:usize,pub conflicts:usize,pub profile:Profile,pub options:Options}
pub fn plan(local:&Snapshot,incoming:Snapshot)->Result<Plan,String>{
 local.validate()?;incoming.validate()?;let mut additions:Vec<Record>=vec![];let(mut duplicates,mut conflicts)=(0,0);
 for row in incoming.records{
  if let Some(existing)=local.records.iter().chain(additions.iter()).find(|x|duplicate(&x.content,&row.content)){
   if existing.category==row.category&&existing.importance==row.importance{duplicates+=1;}else{conflicts+=1;}
  }else{additions.push(row);}
 }
 if local.records.len()+additions.len()>100{return Err("Merged memory exceeds 100 records; no changes applied".into());}
 Ok(Plan{additions,duplicates,conflicts,profile:incoming.profile,options:incoming.options})
}
#[derive(Deserialize)]
#[serde(tag="action",rename_all="snake_case")]
enum Command{Seal{snapshot:Snapshot,password:String},Preview{local:Snapshot,package:String,password:String}}
pub fn command(input:&str)->Result<String,String>{
 if input.len()>3*MAX{return Err("Request too large".into());}
 match serde_json::from_str::<Command>(input).map_err(|_|"Invalid sync request")?{
 Command::Seal{snapshot,password}=>{let password=Zeroizing::new(password);Ok(serde_json::json!({"package":seal(snapshot,&password)?}).to_string())},
 Command::Preview{local,package,password}=>{let password=Zeroizing::new(password);serde_json::to_string(&plan(&local,open(&package,&password)?)?).map_err(|e|e.to_string())}
 }
}
#[cfg(test)]mod tests{use super::*;
 fn sample()->Snapshot{Snapshot{version:1,records:vec![Record{content:"Люблю гітару".into(),category:"Interests".into(),importance:80}],profile:Default::default(),options:Default::default()}}
 #[test]fn roundtrip(){let p=seal(sample(),"test passphrase 123").unwrap();assert_eq!(open(&p,"test passphrase 123").unwrap().records[0].content,"Люблю гітару");assert!(open(&p,"wrong passphrase").is_err());let mut b=STANDARD.decode(p).unwrap();let i=b.len()-1;b[i]^=1;assert!(open(&STANDARD.encode(b),"test passphrase 123").is_err());}
 #[test]fn local_wins(){let local=sample();let mut other=sample();other.records[0].importance=50;let p=plan(&local,other).unwrap();assert_eq!(p.conflicts,1);assert!(p.additions.is_empty());}
 #[test]fn duplicate(){assert_eq!(plan(&sample(),sample()).unwrap().duplicates,1);}
}
