//! Desktop action foundation. No arbitrary shell, model-issued or remote actions.
use std::{path::{Path,PathBuf},time::{Duration,Instant}};
use eframe::egui;
use rusqlite::{Connection,params};
const TTL:Duration=Duration::from_secs(60);
const MAX_AUDIT:usize=200;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Action{InspectStatus,OpenData,GrantFolder,ListFolder,ReadText}
impl Action{
 fn key(self)->&'static str{match self{Self::InspectStatus=>"inspect_status",Self::OpenData=>"open_app_data",Self::GrantFolder=>"grant_folder",Self::ListFolder=>"list_folder",Self::ReadText=>"read_text"}}
 fn label(self,en:bool)->&'static str{match (self,en){
  (Self::InspectStatus,true)=>"Inspect local application status",
  (Self::InspectStatus,false)=>"Переглянути локальний стан застосунку",
  (Self::OpenData,true)=>"Open Hatoriel data folder in Explorer",
  (Self::OpenData,false)=>"Відкрити папку даних Hatoriel у Провіднику",
  (Self::GrantFolder,true)=>"Grant read-only folder access", (Self::GrantFolder,false)=>"Дозволити читання папки",
  (Self::ListFolder,true)=>"List granted folder", (Self::ListFolder,false)=>"Переглянути вміст папки",
  (Self::ReadText,true)=>"Preview local text file", (Self::ReadText,false)=>"Прочитати текстовий файл",
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
  if (action!=Action::InspectStatus)!=target.is_some(){return Err("Invalid action target".into());}
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
pub struct Computer{gate:Gate,status:String,scopes:crate::file_access::Scopes,current:Option<PathBuf>,entries:Vec<crate::file_access::Entry>,preview:Option<(PathBuf,String)>}
impl Computer{
 pub fn leave(&mut self){self.gate.cancel();self.preview=None;}
 fn clear_view(&mut self){self.current=None;self.entries.clear();self.preview=None;}
 fn request(&mut self,action:Action,target:PathBuf){match self.gate.propose(action,Some(target),Instant::now()){Ok(())=>self.status="Review required / Потрібне підтвердження".into(),Err(e)=>self.status=e}}
 pub fn panel(&mut self,ui:&mut egui::Ui,db:Option<&Connection>,busy:bool,en:bool,backend:&str,model:&str,bridge:bool){
  self.gate.expire(Instant::now());
  ui.heading(if en{"Computer interaction · File Access"}else{"Взаємодія з комп’ютером · File Access"});
  ui.label(if en{"Local, manual requests only. No AI-generated commands, remote requests or file modifications. Only explicitly granted folders."}else{"Лише локальні ручні запити. Без команд від ШІ, віддалених дій або зміни файлів. Лише дозволені папки."});
  let mut enabled=self.gate.enabled;
  if ui.checkbox(&mut enabled,if en{"Enable actions for this session"}else{"Дозволити дії в поточному сеансі"}).changed(){self.gate.enable(enabled);if !enabled{self.scopes.clear();self.clear_view();}self.status=if enabled{"Session permission granted / Дозвіл надано".into()}else{"Permission revoked / Дозвіл відкликано".into()};}
  if busy{self.gate.cancel();}
  if db.is_none(){ui.label("Action journal unavailable. Execution blocked / Журнал недоступний. Дії заблоковані.");self.gate.cancel();}
  let can=self.gate.enabled&&!busy&&db.is_some()&&self.gate.pending.is_none();
  for action in [Action::InspectStatus,Action::OpenData]{
   if ui.add_enabled(can&&(action!=Action::OpenData||cfg!(windows)),egui::Button::new(action.label(en))).clicked(){
    let target=match action{Action::InspectStatus=>Ok(None),Action::OpenData=>local_data().map(Some),_=>Err("Invalid quick action".into())};
    match target.and_then(|t|self.gate.propose(action,t,Instant::now())){Ok(())=>self.status="Review required / Потрібне підтвердження".into(),Err(e)=>self.status=e}
   }
  }
  ui.separator();ui.heading("Read-only folders / Дозволені папки");
  ui.small("Session-only grants. No writes, no recursive scan, no automatic model access. / Дозволи лише на сеанс.");
  #[cfg(windows)]
  if ui.add_enabled(can&&self.scopes.roots().len()<crate::file_access::MAX_ROOTS,egui::Button::new("Choose folder / Обрати папку")).clicked(){
   if let Some(path)=rfd::FileDialog::new().pick_folder(){self.request(Action::GrantFolder,path);}
  }
  let mut revoke=None;let mut request=None;
  for root in self.scopes.roots(){
   ui.horizontal_wrapped(|ui|{ui.label(root.display().to_string());if ui.add_enabled(can,egui::Button::new("List / Вміст")).clicked(){request=Some((Action::ListFolder,root.clone()));}if ui.button("Revoke / Відкликати").clicked(){revoke=Some(root.clone());}});
  }
  if let Some(path)=revoke{self.gate.cancel();self.scopes.remove(&path);self.clear_view();request=None;self.status="Folder access revoked / Доступ відкликано".into();}
  if let Some(current)=&self.current{
   ui.label(current.display().to_string());
   if let Some(parent)=current.parent(){if self.scopes.roots().iter().any(|r|parent.starts_with(r))&&ui.add_enabled(can,egui::Button::new("Up / Назад")).clicked(){request=Some((Action::ListFolder,parent.to_path_buf()));}}
   egui::ScrollArea::vertical().id_salt("computer-files").max_height(220.).show(ui,|ui|{
    for entry in &self.entries{ui.horizontal(|ui|{
     let title=if entry.dir{format!("[DIR] {}",entry.name)}else{format!("{} · {} bytes",entry.name,entry.bytes)};
     if ui.add_enabled(can,egui::Button::new(title)).clicked(){request=Some((if entry.dir{Action::ListFolder}else{Action::ReadText},entry.path.clone()));}
    });}
   });
  }
  if let Some((action,target))=request{if self.gate.pending.is_none(){self.request(action,target);}}
  if let Some((path,text))=&self.preview{ui.label(path.display().to_string());egui::ScrollArea::vertical().id_salt("computer-file-preview").max_height(180.).show(ui,|ui|{ui.label(text);});if ui.button("Close preview / Закрити").clicked(){self.preview=None;}}
  if let Some(p)=self.gate.pending.as_ref(){
   ui.separator();ui.label(egui::RichText::new("REVIEW / ПІДТВЕРДЖЕННЯ").color(crate::crimson::RED).strong());
   ui.label(p.action.label(en));
   if let Some(path)=&p.target{ui.label(path.display().to_string());ui.small(match p.action{Action::OpenData=>"Opens Explorer, no content read.",Action::GrantFolder=>"Grants read-only access for this session, not automatic scanning.",Action::ListFolder=>"Lists at most 200 immediate entries, no recursion.",Action::ReadText=>"Reads at most 64 KiB of supported plain text, displayed locally.",_=>""});}
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
         Action::GrantFolder=>p.target.as_deref().ok_or_else(||"Missing folder".to_string()).and_then(|path|self.scopes.grant(path)).map(|_|"Read-only grant added / Дозвіл надано".into()),
         Action::ListFolder=>p.target.as_deref().ok_or_else(||"Missing folder".to_string()).and_then(|path|self.scopes.list(path)).map(|(entries,capped)|{self.current=p.target.clone();self.preview=None;let n=entries.len();self.entries=entries;format!("{n} entries{}",if capped{" · capped at 200 scanned entries"}else{""})}),
         Action::ReadText=>p.target.as_deref().ok_or_else(||"Missing file".to_string()).and_then(|path|self.scopes.read_text(path)).map(|text|{self.preview=p.target.clone().map(|path|(path,text));"Local text preview / Локальне прев’ю".into()}),
        };
        let outcome=if result.is_ok(){if p.action==Action::OpenData{"launch_requested"}else{"completed"}}else{"failed"};
        self.status=match result{Ok(s)=>s,Err(e)=>format!("Action failed: {e}")};
        if finish(db,id,outcome).is_err(){self.status.push_str("\nAudit completion write failed; initial approval remains recorded.");self.gate.enable(false);self.scopes.clear();self.clear_view();}
       },Err(e)=>{self.status=format!("Action blocked: audit write failed: {e}");self.gate.enable(false);self.scopes.clear();self.clear_view();}
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
