//! Session-only policy for typed local computer actions.
use std::time::{Duration,Instant};
use crate::computer::Action;
pub const SESSION_TTL:Duration=Duration::from_secs(30*60);
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Category{Status,Files,Apps,Screen,ClipboardRead,ClipboardWrite,Transfer}
impl Category{
 pub const ALL:[Self;7]=[Self::Status,Self::Files,Self::Apps,Self::Screen,Self::ClipboardRead,Self::ClipboardWrite,Self::Transfer];
 fn bit(self)->u8{1<<(self as u8)}
 pub fn label(self,en:bool)->&'static str{match(self,en){
 (Self::Status,true)=>"Application status",(Self::Status,false)=>"Стан застосунку",
 (Self::Files,true)=>"Read-only files",(Self::Files,false)=>"Читання файлів",
 (Self::Apps,true)=>"Application launch",(Self::Apps,false)=>"Запуск програм",
 (Self::Screen,true)=>"Screen capture",(Self::Screen,false)=>"Захоплення екрана",
 (Self::ClipboardRead,true)=>"Clipboard read",(Self::ClipboardRead,false)=>"Читання буфера",
 (Self::ClipboardWrite,true)=>"Clipboard write",(Self::ClipboardWrite,false)=>"Запис у буфер",
 (Self::Transfer,true)=>"File transfer",(Self::Transfer,false)=>"Передавання файлів",
 }}
}
pub fn category(action:Action)->Category{match action{
 Action::InspectStatus=>Category::Status,
 Action::OpenData|Action::GrantFolder|Action::ListFolder|Action::ReadText|Action::Search=>Category::Files,
 Action::RegisterApp|Action::LaunchApp=>Category::Apps,
 Action::CaptureScreen=>Category::Screen,
 Action::ReadClipboardText|Action::ReadClipboardImage=>Category::ClipboardRead,
 Action::WriteClipboardText=>Category::ClipboardWrite,
 Action::OfferFile=>Category::Transfer,
}}
pub fn risk(action:Action,en:bool)->&'static str{match (action,en){
 (Action::InspectStatus,true)=>"Low",(Action::InspectStatus,false)=>"Низький",
 (Action::OpenData|Action::GrantFolder|Action::ListFolder|Action::ReadText|Action::Search,true)=>"Sensitive local access",
 (Action::OpenData|Action::GrantFolder|Action::ListFolder|Action::ReadText|Action::Search,false)=>"Доступ до локальних даних",
 (_,true)=>"High: sensitive data or external effect",(_,false)=>"Високий: чутливі дані або зовнішня дія",
}}
#[derive(Default)]
pub struct Policy{started:Option<Instant>,mask:u8}
impl Policy{
 pub fn start(&mut self,now:Instant){self.started=Some(now);self.mask=Category::Status.bit();}
 pub fn stop(&mut self){self.started=None;self.mask=0;}
 pub fn remaining(&self,now:Instant)->Duration{self.started.map(|t|SESSION_TTL.saturating_sub(now.saturating_duration_since(t))).unwrap_or_default()}
 pub fn active(&self,now:Instant)->bool{!self.remaining(now).is_zero()}
 pub fn granted(&self,c:Category,now:Instant)->bool{self.active(now)&&self.mask&c.bit()!=0}
 pub fn set(&mut self,c:Category,value:bool,now:Instant)->Result<(),String>{
  if !self.active(now){return Err("Permission session expired or inactive".into());}
  if value{self.mask|=c.bit();}else{self.mask&=!c.bit();}Ok(())
 }
 pub fn allows(&self,action:Action,now:Instant)->bool{
  self.granted(category(action),now)&& (action!=Action::OfferFile||self.granted(Category::Files,now))
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn default_deny_and_expiry(){let n=Instant::now();let mut p=Policy::default();assert!(!p.allows(Action::InspectStatus,n));p.start(n);assert!(p.allows(Action::InspectStatus,n));assert!(!p.allows(Action::CaptureScreen,n));p.set(Category::Screen,true,n).unwrap();assert!(p.allows(Action::CaptureScreen,n));assert!(!p.allows(Action::CaptureScreen,n+SESSION_TTL));assert!(p.set(Category::Screen,true,n+SESSION_TTL).is_err());}
 #[test]fn independent_clipboard_and_transfer(){let n=Instant::now();let mut p=Policy::default();p.start(n);p.set(Category::ClipboardRead,true,n).unwrap();assert!(p.allows(Action::ReadClipboardText,n));assert!(!p.allows(Action::WriteClipboardText,n));p.set(Category::Transfer,true,n).unwrap();assert!(!p.allows(Action::OfferFile,n));p.set(Category::Files,true,n).unwrap();assert!(p.allows(Action::OfferFile,n));p.set(Category::Files,false,n).unwrap();assert!(!p.allows(Action::OfferFile,n));}
 #[test]fn restart_does_not_restore_grants(){let n=Instant::now();let mut p=Policy::default();p.start(n);p.set(Category::Apps,true,n).unwrap();p.stop();assert!(!p.allows(Action::LaunchApp,n));p.start(n);assert!(!p.allows(Action::LaunchApp,n));}
}
