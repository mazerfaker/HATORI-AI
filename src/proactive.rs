use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Reminder{pub id:u64,pub text:String,pub due:u64,pub announced:bool,pub completed:bool}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(default)]
pub struct Settings{pub enabled:bool,pub dnd:bool,pub interval_minutes:u32,pub last_suggestion:u64,pub next_id:u64,pub reminders:Vec<Reminder>,pub projects:bool,pub resume_chats:bool,pub cursor:usize}
impl Default for Settings{fn default()->Self{Self{enabled:false,dnd:false,interval_minutes:60,last_suggestion:0,next_id:1,reminders:vec![],projects:false,resume_chats:false,cursor:0}}}
impl Settings{
 pub fn add(&mut self,text:&str,minutes:u32,now:u64)->bool{let text=text.trim();if text.is_empty()||text.chars().count()>500||self.reminders.len()>=100{return false;}let id=self.next_id;if id==u64::MAX||self.reminders.iter().any(|r|r.id==id){return false;}self.next_id+=1;self.reminders.push(Reminder{id,text:text.into(),due:now.saturating_add(u64::from(minutes.clamp(1,10080))*60),announced:false,completed:false});true}
 pub fn poll(&mut self,now:u64)->Option<String>{self.poll_reminder(now).map(|(_,text)|text)}
 pub fn poll_reminder(&mut self,now:u64)->Option<(u64,String)>{if self.dnd{return None;}let r=self.reminders.iter_mut().filter(|r|!r.completed&&!r.announced&&r.due<=now).min_by_key(|r|(r.due,r.id))?;r.announced=true;Some((r.id,r.text.clone()))}
 pub fn snooze(&mut self,id:u64,minutes:u32,now:u64)->bool{let Some(r)=self.reminders.iter_mut().find(|r|r.id==id&&!r.completed) else{return false;};r.due=now.saturating_add(u64::from(minutes.clamp(1,10080))*60);r.announced=false;true}
 pub fn complete(&mut self,id:u64)->bool{let Some(r)=self.reminders.iter_mut().find(|r|r.id==id) else{return false;};r.completed=true;true}
 pub fn next_checkin(&mut self,now:u64,idle_seconds:u64,initiative:u8,candidates:&[CheckIn])->Option<CheckIn>{
 if !self.enabled||self.dnd||initiative==0||idle_seconds<120||candidates.is_empty(){return None;}
 let interval=u64::from(self.interval_minutes.clamp(15,1440))*60;
 if self.last_suggestion==0||self.last_suggestion>now{self.last_suggestion=now;return None;}
 // Lower initiative means fewer notices, never more than the chosen frequency.
 let interval=interval.saturating_mul(if initiative<=30{3}else if initiative<=60{2}else{1});
 if now.saturating_sub(self.last_suggestion)<interval{return None;}
 let result=candidates[self.cursor%candidates.len()].clone();self.cursor=(self.cursor%candidates.len()+1)%candidates.len();self.last_suggestion=now;Some(result)
 }
 pub fn goal_prompt(&mut self,now:u64,goals:&str,initiative:u8)->Option<String>{
 if !self.enabled||self.dnd||initiative==0||goals.trim().is_empty(){return None;}
 let interval=u64::from(self.interval_minutes.clamp(15,1440))*60;
 if self.last_suggestion==0{self.last_suggestion=now;return None;}
 if now.saturating_sub(self.last_suggestion)<interval{return None;}
 self.last_suggestion=now;Some(goals.trim().chars().take(240).collect())
 }
}
#[cfg(test)]mod tests{use super::*;#[test]fn reminders(){let mut s=Settings::default();assert!(s.add("test",1,100));assert!(s.poll(159).is_none());s.dnd=true;assert!(s.poll(160).is_none());s.dnd=false;assert_eq!(s.poll(160).as_deref(),Some("test"));assert!(s.poll(161).is_none());}#[test]fn opt_in(){let mut s=Settings::default();assert!(s.goal_prompt(100,"goal",90).is_none());s.enabled=true;assert!(s.goal_prompt(100,"goal",90).is_none());assert!(s.goal_prompt(3700,"goal",90).is_some());assert!(s.goal_prompt(3701,"goal",90).is_none());}}

#[derive(Clone,Debug,PartialEq,Eq)]
pub enum CheckIn{Goal(String),Project(String),Resume(i64,String)}
#[derive(Clone,Debug)]
pub enum Notice{Reminder(u64,String),CheckIn(CheckIn)}
impl Notice{
 pub fn text(&self,en:bool)->String{match self{
 Self::Reminder(_,s)=>format!("{}: {s}",if en{"Reminder"}else{"Нагадування"}),
 Self::CheckIn(CheckIn::Goal(s))=>format!("{}: {s}",if en{"Goal check-in"}else{"Перевірка цілі"}),
 Self::CheckIn(CheckIn::Project(s))=>format!("{}: {s}",if en{"Review project"}else{"Переглянути проєкт"}),
 Self::CheckIn(CheckIn::Resume(_,s))=>format!("{}: {s}",if en{"Resume previous chat?"}else{"Продовжити попередній чат?"}),
 }}
}
#[cfg(test)]mod initiative_tests{
 use super::*;
 #[test]fn rotation_and_quiet(){let mut s=Settings{enabled:true,last_suggestion:100,interval_minutes:15,..Default::default()};let c=vec![CheckIn::Goal("goal".into()),CheckIn::Project("project".into())];assert!(s.next_checkin(1000,119,90,&c).is_none());assert_eq!(s.next_checkin(1000,120,90,&c),Some(c[0].clone()));s.dnd=true;assert!(s.next_checkin(1900,120,90,&c).is_none());s.dnd=false;assert_eq!(s.next_checkin(1900,120,90,&c),Some(c[1].clone()));}
 #[test]fn low_initiative_and_clock(){let mut s=Settings{enabled:true,last_suggestion:100,interval_minutes:15,..Default::default()};let c=vec![CheckIn::Goal("goal".into())];assert!(s.next_checkin(1000,120,30,&c).is_none());assert!(s.next_checkin(2800,120,30,&c).is_some());assert!(s.next_checkin(10,120,90,&c).is_none());assert_eq!(s.last_suggestion,10);}
 #[test]fn due_order_and_snooze(){let mut s=Settings::default();s.add("later",10,100);s.add("earlier",1,100);let(id,_)=s.poll_reminder(1000).unwrap();assert_eq!(id,2);assert!(s.snooze(id,10,1000));assert_eq!(s.poll_reminder(1000).unwrap().0,1);assert!(s.complete(id));assert!(s.poll_reminder(1600).is_none());}
 #[test]fn legacy_settings(){let s:Settings=serde_json::from_str(r#"{"enabled":true,"interval_minutes":60}"#).unwrap();assert!(s.enabled);assert!(!s.projects&&!s.resume_chats);}
}
