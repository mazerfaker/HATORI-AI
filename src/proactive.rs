use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Reminder{pub id:u64,pub text:String,pub due:u64,pub announced:bool,pub completed:bool}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(default)]
pub struct Settings{pub enabled:bool,pub dnd:bool,pub interval_minutes:u32,pub last_suggestion:u64,pub next_id:u64,pub reminders:Vec<Reminder>}
impl Default for Settings{fn default()->Self{Self{enabled:false,dnd:false,interval_minutes:60,last_suggestion:0,next_id:1,reminders:vec![]}}}
impl Settings{
 pub fn add(&mut self,text:&str,minutes:u32,now:u64)->bool{let text=text.trim();if text.is_empty()||text.chars().count()>500||self.reminders.len()>=100{return false;}let id=self.next_id;self.next_id=self.next_id.saturating_add(1);self.reminders.push(Reminder{id,text:text.into(),due:now.saturating_add(u64::from(minutes.clamp(1,10080))*60),announced:false,completed:false});true}
 pub fn poll(&mut self,now:u64)->Option<String>{if self.dnd{return None;}let r=self.reminders.iter_mut().find(|r|!r.completed&&!r.announced&&r.due<=now)?;r.announced=true;Some(r.text.clone())}
 pub fn goal_prompt(&mut self,now:u64,goals:&str,initiative:u8)->Option<String>{
 if !self.enabled||self.dnd||initiative==0||goals.trim().is_empty(){return None;}
 let interval=u64::from(self.interval_minutes.clamp(15,1440))*60;
 if self.last_suggestion==0{self.last_suggestion=now;return None;}
 if now.saturating_sub(self.last_suggestion)<interval{return None;}
 self.last_suggestion=now;Some(goals.trim().chars().take(240).collect())
 }
}
#[cfg(test)]mod tests{use super::*;#[test]fn reminders(){let mut s=Settings::default();assert!(s.add("test",1,100));assert!(s.poll(159).is_none());s.dnd=true;assert!(s.poll(160).is_none());s.dnd=false;assert_eq!(s.poll(160).as_deref(),Some("test"));assert!(s.poll(161).is_none());}#[test]fn opt_in(){let mut s=Settings::default();assert!(s.goal_prompt(100,"goal",90).is_none());s.enabled=true;assert!(s.goal_prompt(100,"goal",90).is_none());assert!(s.goal_prompt(3700,"goal",90).is_some());assert!(s.goal_prompt(3701,"goal",90).is_none());}}
