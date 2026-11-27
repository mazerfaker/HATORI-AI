//! Desktop action foundation. No arbitrary shell, model-issued or remote actions.
use std::{path::{Path,PathBuf},time::{Duration,Instant}};
use eframe::egui;
use rusqlite::{Connection,params};
const TTL:Duration=Duration::from_secs(60);
const MAX_AUDIT:usize=200;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Action{InspectStatus,OpenData}
impl Action{
 fn key(self)->&'static str{match self{Self::InspectStatus=>"inspect_status",Self::OpenData=>"open_app_data"}}
 fn label(self,en:bool)->&'static str{match (self,en){
  (Self::InspectStatus,true)=>"Inspect local application status",
  (Self::InspectStatus,false)=>"Переглянути локальний стан застосунку",
  (Self::OpenData,true)=>"Open Hatoriel data folder in Explorer",
  (Self::OpenData,false)=>"Відкрити папку даних Hatoriel у Провіднику",
 }}
}
#[derive(Clone,Debug)]
struct Proposal{action:Action,target:Option<PathBuf>,created:Instant}
#[derive(Default)]
struct Gate{enabled:bool,pending:Option<Proposal>}
impl Gate{
 fn enable(&mut self,value:bool){self.enabled=value;if !value{self.pending=None;}}
 fn cancel(&mut self){self.pending=None;}
 fn expire(&mut self,now:Instant){if self.pending.as_ref().is_some_and(|p|now.saturating_duration_since(p.created)>=TTL){self.cancel();}}
 fn propose(&mut self,action:Action,target:Option<PathBuf>,now:Instant)->Result<(),String>{
  if !self.enabled{return Err("Computer interaction disabled".into());}
  if self.pending.is_some(){return Err("Resolve the existing proposal first".into());}
  if (action==Action::OpenData)!=target.is_some(){return Err("Invalid action target".into());}
  self.pending=Some(Proposal{action,target,created:now});Ok(())
 }
 fn approve(&mut self,now:Instant)->Result<Proposal,String>{
  if !self.enabled{self.cancel();return Err("Permission revoked".into());}
  self.expire(now);self.pending.take().ok_or_else(||"No active proposal (expired or already used)".into())
 }
}
fn migrate(db:&Connection)->rusqlite::Result<()>{
 db.execute_batch("CREATE TABLE IF NOT EXISTS computer_audit(id INTEGER PRIMARY KEY AUTOINCREMENT, stamp INTEGER NOT NULL, action TEXT NOT NULL, outcome TEXT NOT NULL);")
}
fn begin(db:&Connection,action:Action)->rusqlite::Result<i64>{
 migrate(db)?;
 let tx=db.unchecked_transaction()?;
 tx.execute("INSERT INTO computer_audit(stamp,action,outcome) VALUES(?1,?2,'approved_pending')",params![hatori_ai::emotion::now().min(i64::MAX as u64) as i64,action.key()])?;
 let id=tx.last_insert_rowid();
 tx.execute("DELETE FROM computer_audit WHERE id NOT IN (SELECT id FROM computer_audit ORDER BY id DESC LIMIT ?1)",[MAX_AUDIT as i64])?;
 tx.commit()?;Ok(id)
}
fn finish(db:&Connection,id:i64,outcome:&str)->rusqlite::Result<()>{
 db.execute("UPDATE computer_audit SET outcome=?1 WHERE id=?2",params![outcome,id])?;Ok(())
}
fn recent(db:&Connection)->rusqlite::Result<Vec<(i64,String,String)>>{
 migrate(db)?;
 let mut q=db.prepare("SELECT stamp,action,outcome FROM computer_audit ORDER BY id DESC LIMIT 30")?;
 let rows=q.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;rows.collect()
}
fn local_data()->Result<PathBuf,String>{
 let dir=std::fs::canonicalize(crate::storage::data_dir()).map_err(|e|format!("App data folder unavailable: {e}"))?;
 if !dir.is_dir(){return Err("App data path is not a directory".into());}
 #[cfg(windows)]{
  use std::path::{Component,Prefix};
  if !matches!(dir.components().next(),Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_))){return Err("Only drive-letter app data folders are supported; no UNC or device paths".into());}
 }
 Ok(dir)
}
#[cfg(windows)]
fn open_data(target:&Path)->Result<String,String>{
 use std::os::windows::process::CommandExt;
 if local_data()?.as_path()!=target{return Err("App data folder changed; review a fresh proposal".into());}
 let root=std::env::var_os("SystemRoot").ok_or("Windows directory unavailable")?;
 let explorer=PathBuf::from(root).join("explorer.exe");
 // Explorer receives one fixed app-folder argument, never a shell command.
 let path=target.to_str().ok_or("Unsupported folder path encoding")?;
 let path=path.strip_prefix(r"\\?\").unwrap_or(path);
 std::process::Command::new(explorer).arg(path).creation_flags(0x08000000).spawn().map_err(|e|e.to_string())?;
 Ok("Explorer launch requested / Запуск Провідника запитано".into())
}
#[cfg(not(windows))]
fn open_data(_: &Path)->Result<String,String>{Err("Explorer action is Windows only".into())}
#[derive(Default)]
pub struct Computer{gate:Gate,status:String}
impl Computer{
 pub fn leave(&mut self){self.gate.cancel();}
 pub fn panel(&mut self,ui:&mut egui::Ui,db:Option<&Connection>,busy:bool,en:bool,backend:&str,model:&str,bridge:bool){
  self.gate.expire(Instant::now());
  ui.heading(if en{"Computer interaction · Foundation"}else{"Взаємодія з комп’ютером · Foundation"});
  ui.label(if en{"Local, manual requests only. No AI-generated commands, remote requests, arbitrary file reads or modifications."}else{"Лише локальні ручні запити. Без команд від ШІ, віддалених дій, довільного читання або зміни файлів."});
  let mut enabled=self.gate.enabled;
  if ui.checkbox(&mut enabled,if en{"Enable actions for this session"}else{"Дозволити дії в поточному сеансі"}).changed(){self.gate.enable(enabled);self.status=if enabled{"Session permission granted / Дозвіл надано".into()}else{"Permission revoked / Дозвіл відкликано".into()};}
  if busy{self.gate.cancel();}
  if db.is_none(){ui.label("Action journal unavailable. Execution blocked / Журнал недоступний. Дії заблоковані.");self.gate.cancel();}
  let can=self.gate.enabled&&!busy&&db.is_some()&&self.gate.pending.is_none();
  for action in [Action::InspectStatus,Action::OpenData]{
   if ui.add_enabled(can&&(action!=Action::OpenData||cfg!(windows)),egui::Button::new(action.label(en))).clicked(){
    let target=match action{Action::InspectStatus=>Ok(None),Action::OpenData=>local_data().map(Some)};
    match target.and_then(|t|self.gate.propose(action,t,Instant::now())){Ok(())=>self.status="Review required / Потрібне підтвердження".into(),Err(e)=>self.status=e}
   }
  }
  if let Some(p)=self.gate.pending.as_ref(){
   ui.separator();ui.label(egui::RichText::new("REVIEW / ПІДТВЕРДЖЕННЯ").color(crate::crimson::RED).strong());
   ui.label(p.action.label(en));
   if let Some(path)=&p.target{ui.label(path.display().to_string());ui.small("Opens Explorer; does not read file contents. / Відкриває Провідник, не читає файли.");}
   else{ui.small("Displays current backend, selected model, app storage mode and bridge state. / Лише стан застосунку.");}
   ui.small(format!("{} s · one-time approval",TTL.saturating_sub(p.created.elapsed()).as_secs()));
   let mut approve=false;let mut cancel=false;
   ui.horizontal(|ui|{
    approve=ui.add_enabled(!busy&&self.gate.enabled&&db.is_some(),egui::Button::new("Confirm once / Підтвердити один раз")).clicked();
    cancel=ui.button("Cancel / Скасувати").clicked();
   });
   if cancel{self.gate.cancel();self.status="Cancelled / Скасовано".into();}
   else if approve{
    match self.gate.approve(Instant::now()){
     Ok(p)=>if let Some(db)=db{
      match begin(db,p.action){
       Ok(id)=>{
        let result=match p.action{
         Action::InspectStatus=>Ok(format!("Backend: {backend}\nModel: {model}\nStorage: {}\nBridge: {}",if crate::storage::portable(){"portable"}else{"installed"},if bridge{"running"}else{"stopped"})),
         Action::OpenData=>p.target.as_deref().ok_or_else(||"Missing approved target".to_string()).and_then(open_data),
        };
        let outcome=if result.is_ok(){if p.action==Action::OpenData{"launch_requested"}else{"completed"}}else{"failed"};
        self.status=match result{Ok(s)=>s,Err(e)=>format!("Action failed: {e}")};
        if finish(db,id,outcome).is_err(){self.status.push_str("\nAudit completion write failed; initial approval remains recorded.");self.gate.enable(false);}
       },Err(e)=>{self.status=format!("Action blocked: audit write failed: {e}");self.gate.enable(false);}
      }
     },
     Err(e)=>self.status=e,
    }
   }
  }
  if !self.status.is_empty(){ui.separator();ui.label(&self.status);}
  ui.separator();ui.heading(if en{"Local action journal"}else{"Локальний журнал дій"});
  ui.small("Last 30 of up to 200 records · UTC Unix timestamp · no paths, prompts or contents stored");
  if let Some(db)=db{
   match recent(db){
    Ok(rows)=>egui::ScrollArea::vertical().id_salt("computer-audit").max_height(240.).show(ui,|ui|{
     if rows.is_empty(){ui.label("No actions yet / Дій ще немає");}
     for (stamp,action,outcome) in rows{ui.label(format!("{stamp} · {action} · {outcome}"));}
    }).inner,
    Err(e)=>{ui.label(format!("Audit unavailable: {e}"));},
   }
  }
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn permission_one_shot(){let now=Instant::now();let mut g=Gate::default();assert!(g.propose(Action::InspectStatus,None,now).is_err());g.enable(true);g.propose(Action::InspectStatus,None,now).unwrap();assert!(g.propose(Action::InspectStatus,None,now).is_err());assert_eq!(g.approve(now).unwrap().action,Action::InspectStatus);assert!(g.approve(now).is_err());}
 #[test]fn expiry_revocation_and_target(){let now=Instant::now();let mut g=Gate::default();g.enable(true);assert!(g.propose(Action::OpenData,None,now).is_err());g.propose(Action::OpenData,Some(PathBuf::from("approved")),now).unwrap();assert!(g.approve(now+TTL).is_err());g.propose(Action::InspectStatus,None,now).unwrap();g.enable(false);assert!(g.approve(now).is_err());}
 #[test]fn journal_bounds(){let db=Connection::open_in_memory().unwrap();for _ in 0..MAX_AUDIT+3{let id=begin(&db,Action::InspectStatus).unwrap();finish(&db,id,"completed").unwrap();}let count:i64=db.query_row("SELECT COUNT(*) FROM computer_audit",[],|r|r.get(0)).unwrap();assert_eq!(count,MAX_AUDIT as i64);let rows=recent(&db).unwrap();assert_eq!(rows.len(),30);assert_eq!(rows[0].1,"inspect_status");assert_eq!(rows[0].2,"completed");}
 #[test]fn audit_fail_closed(){let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE computer_audit(wrong INTEGER);").unwrap();assert!(begin(&db,Action::InspectStatus).is_err());}
}
