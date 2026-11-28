//! Reviewed, bounded sequence of local typed actions. Every step uses the regular approval gate.
use std::path::PathBuf;
use crate::{computer::Action,file_search::Filter};
pub const MAX_STEPS:usize=8;
#[derive(Clone)]
pub struct Step{
 pub action:Action,pub target:Option<PathBuf>,pub filter:Option<Filter>,pub text:Option<String>,
}
impl Step{
 pub fn new(action:Action,target:Option<PathBuf>,filter:Option<Filter>,text:Option<String>)->Result<Self,String>{
  if !matches!(action,Action::InspectStatus|Action::OpenData|Action::ListFolder|Action::ReadText|Action::Search|Action::LaunchApp|Action::CaptureScreen|Action::ReadClipboardText|Action::ReadClipboardImage|Action::WriteClipboardText){return Err("Action not supported in a workflow".into());}
  let requires_target=matches!(action,Action::OpenData|Action::ListFolder|Action::ReadText|Action::Search|Action::LaunchApp);
  if requires_target!=target.is_some()||(action==Action::Search)!=filter.is_some()||(action==Action::WriteClipboardText)!=text.is_some(){return Err("Invalid workflow parameters".into());}
  if let Some(f)=&filter{f.validate()?;}
  if let Some(t)=&text{crate::clipboard_intelligence::validate_text(t)?;}
  Ok(Self{action,target,filter,text})
 }
}
#[derive(Default)]
pub struct Workflow{
 steps:Vec<Step>,index:usize,active:bool,awaiting:bool,pub status:String,
}
impl Workflow{
 pub fn steps(&self)->&[Step]{&self.steps}
 pub fn active(&self)->bool{self.active}
 pub fn awaiting(&self)->bool{self.awaiting}
 pub fn progress(&self)->(usize,usize){(self.index,self.steps.len())}
 pub fn add(&mut self,step:Step)->Result<(),String>{
  if self.active||self.steps.len()>=MAX_STEPS{return Err("Workflow is active or full".into());}
  self.steps.push(step);self.status="Plan updated / План оновлено".into();Ok(())
 }
 pub fn remove(&mut self,i:usize){if !self.active&&i<self.steps.len(){self.steps.remove(i);}}
 pub fn up(&mut self,i:usize){if !self.active&&i>0&&i<self.steps.len(){self.steps.swap(i-1,i);}}
 pub fn start(&mut self)->Result<(),String>{
  if self.active||self.steps.is_empty(){return Err("Workflow is empty or active".into());}
  self.index=0;self.awaiting=false;self.active=true;self.status="Plan started; confirm each step / Підтверджуй кожний крок".into();Ok(())
 }
 pub fn next(&mut self)->Option<Step>{
  if !self.active||self.awaiting{return None;}
  let step=self.steps.get(self.index)?.clone();self.awaiting=true;Some(step)
 }
 pub fn complete(&mut self,ok:bool){
  if !self.active||!self.awaiting{return;}
  if !ok{self.stop("Workflow stopped on failed step / Помилка кроку");return;}
  self.index+=1;self.awaiting=false;
  if self.index>=self.steps.len(){let total=self.steps.len();self.stop(&format!("Workflow completed: {total}/{total} / План виконано"));}
 }
 pub fn stop(&mut self,reason:&str){
  if self.active||!self.steps.is_empty(){self.status=reason.into();}
  self.steps.clear();self.active=false;self.awaiting=false;self.index=0;
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn status()->Step{Step::new(Action::InspectStatus,None,None,None).unwrap()}
 #[test]fn ordered_steps_and_single_dispatch(){let mut w=Workflow::default();w.add(status()).unwrap();w.add(Step::new(Action::CaptureScreen,None,None,None).unwrap()).unwrap();w.start().unwrap();assert_eq!(w.next().unwrap().action,Action::InspectStatus);assert!(w.next().is_none());assert!(w.add(status()).is_err());w.complete(true);assert_eq!(w.progress(),(1,2));assert_eq!(w.next().unwrap().action,Action::CaptureScreen);w.complete(true);assert!(!w.active());assert!(w.next().is_none());assert!(w.status.contains("2/2"));}
 #[test]fn failure_and_stop(){let mut w=Workflow::default();w.add(status()).unwrap();w.start().unwrap();w.next();w.complete(false);assert!(!w.active());assert!(w.steps().is_empty());w.add(status()).unwrap();w.stop("revoked");assert!(w.start().is_err());}
 #[test]fn rejects_permission_changes_and_missing_payload(){assert!(Step::new(Action::GrantFolder,Some(PathBuf::from("x")),None,None).is_err());assert!(Step::new(Action::OfferFile,Some(PathBuf::from("x")),None,None).is_err());assert!(Step::new(Action::WriteClipboardText,None,None,None).is_err());assert!(Step::new(Action::Search,Some(PathBuf::from("x")),None,None).is_err());}
 #[test]fn cap_and_reorder(){let mut w=Workflow::default();for _ in 0..MAX_STEPS-1{w.add(status()).unwrap();}w.add(Step::new(Action::CaptureScreen,None,None,None).unwrap()).unwrap();assert!(w.add(status()).is_err());w.up(MAX_STEPS-1);assert_eq!(w.steps()[MAX_STEPS-2].action,Action::CaptureScreen);w.remove(0);assert_eq!(w.steps().len(),MAX_STEPS-1);}
}
