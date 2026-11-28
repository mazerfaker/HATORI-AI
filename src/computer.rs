//! Desktop action foundation. No arbitrary shell, model-issued or remote actions.
use std::{path::{Path,PathBuf},time::{Duration,Instant}};
use eframe::egui;
use rusqlite::{Connection,params};
const TTL:Duration=Duration::from_secs(60);
const MAX_AUDIT:usize=200;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Action{InspectStatus,OpenData,GrantFolder,ListFolder,ReadText,Search,OfferFile,RegisterApp,LaunchApp,CaptureScreen}
impl Action{
 fn key(self)->&'static str{match self{Self::InspectStatus=>"inspect_status",Self::OpenData=>"open_app_data",Self::GrantFolder=>"grant_folder",Self::ListFolder=>"list_folder",Self::ReadText=>"read_text",Self::Search=>"search_files",Self::OfferFile=>"offer_file",Self::RegisterApp=>"register_app",Self::LaunchApp=>"launch_app",Self::CaptureScreen=>"capture_screen"}}
 fn label(self,en:bool)->&'static str{match (self,en){
  (Self::InspectStatus,true)=>"Inspect local application status",
  (Self::InspectStatus,false)=>"Переглянути локальний стан застосунку",
  (Self::OpenData,true)=>"Open Hatoriel data folder in Explorer",
  (Self::OpenData,false)=>"Відкрити папку даних Hatoriel у Провіднику",
  (Self::GrantFolder,true)=>"Grant read-only folder access", (Self::GrantFolder,false)=>"Дозволити читання папки",
  (Self::ListFolder,true)=>"List granted folder", (Self::ListFolder,false)=>"Переглянути вміст папки",
  (Self::ReadText,true)=>"Preview local text file", (Self::ReadText,false)=>"Прочитати текстовий файл",
  (Self::Search,true)=>"Search granted folder",(Self::Search,false)=>"Знайти файли в дозволеній папці",
  (Self::OfferFile,true)=>"Prepare original file for phone",(Self::OfferFile,false)=>"Підготувати оригінал для смартфона",
  (Self::RegisterApp,true)=>"Approve executable for this session",(Self::RegisterApp,false)=>"Дозволити програму на цей сеанс",
  (Self::LaunchApp,true)=>"Launch approved application",(Self::LaunchApp,false)=>"Запустити дозволену програму",
  (Self::CaptureScreen,true)=>"Capture primary monitor",(Self::CaptureScreen,false)=>"Зробити знімок основного монітора",
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
  if (!matches!(action,Action::InspectStatus|Action::CaptureScreen))!=target.is_some(){return Err("Invalid action target".into());}
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
struct SearchJob{
 id:i64,cancel:std::sync::Arc<std::sync::atomic::AtomicBool>,
 rx:std::sync::mpsc::Receiver<Result<crate::file_search::Report,String>>,
}
#[derive(Default)]
pub struct Computer{gate:Gate,status:String,scopes:crate::file_access::Scopes,current:Option<PathBuf>,entries:Vec<crate::file_access::Entry>,preview:Option<(PathBuf,String)>,filter:crate::file_search::Filter,search_pending:Option<crate::file_search::Filter>,search_job:Option<SearchJob>,search_report:Option<crate::file_search::Report>,apps:crate::app_control::Catalog,screen:Option<crate::screen_awareness::Snapshot>,screen_attach:bool}
impl Computer{
 pub fn leave(&mut self){self.screen=None;self.screen_attach=false;self.gate.cancel();self.preview=None;self.search_pending=None;self.cancel_search();}
 fn cancel_search(&mut self){if let Some(j)=&self.search_job{j.cancel.store(true,std::sync::atomic::Ordering::Relaxed);}self.search_report=None;}
 pub fn poll(&mut self,db:Option<&Connection>){
  self.apps.poll();
  let result=match self.search_job.as_ref().map(|j|j.rx.try_recv()){Some(Ok(r))=>r,Some(Err(std::sync::mpsc::TryRecvError::Disconnected))=>Err("Search worker disconnected".into()),_=>return};
  let job=self.search_job.take().unwrap();let cancelled=job.cancel.load(std::sync::atomic::Ordering::Relaxed)||result.as_ref().is_ok_and(|r|r.cancelled);
  let outcome=if cancelled{"cancelled"}else if result.is_ok(){"completed"}else{"failed"};
  if db.is_none_or(|d|finish(d,job.id,outcome).is_err()){self.status="Search audit completion failed; permissions revoked".into();self.gate.enable(false);self.scopes.clear();self.apps.clear();self.clear_view();return;}
  if cancelled{self.status="Search cancelled / Пошук скасовано".into();return;}
  match result{Ok(r)=>{self.status=format!("{} matches · {} scanned · {} text reads{}",r.hits.len(),r.scanned,r.text_reads,if r.limited{" · partial results"}else{""});self.search_report=Some(r);},Err(e)=>self.status=e}
 }
 fn start_search(&mut self,id:i64,root:PathBuf,filter:crate::file_search::Filter)->Result<(),String>{
  if self.search_job.is_some(){return Err("Search already running".into());}let scopes=self.scopes.clone();let cancel=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));let flag=cancel.clone();let (tx,rx)=std::sync::mpsc::channel();
  std::thread::Builder::new().name("hatoriel-file-search".into()).spawn(move||{let result=std::panic::catch_unwind(||crate::file_search::search(&scopes,&root,&filter,&flag)).unwrap_or_else(|_|Err("Search worker failed".into()));let _=tx.send(result);}).map_err(|e|e.to_string())?;
  self.search_report=None;self.search_job=Some(SearchJob{id,cancel,rx});Ok(())
 }
 pub fn screen_for_chat(&mut self)->Option<Vec<u8>>{if !std::mem::take(&mut self.screen_attach){return None;}self.screen.as_ref().map(|s|s.bytes.clone())}
 pub fn screen_feedback(&mut self,message:String){self.status=message;}
 fn clear_view(&mut self){self.screen=None;self.screen_attach=false;self.current=None;self.entries.clear();self.preview=None;self.search_pending=None;self.cancel_search();}
 fn request(&mut self,action:Action,target:PathBuf){match self.gate.propose(action,Some(target),Instant::now()){Ok(())=>self.status="Review required / Потрібне підтвердження".into(),Err(e)=>self.status=e}}
 pub fn panel(&mut self,ui:&mut egui::Ui,db:Option<&Connection>,busy:bool,en:bool,backend:&str,model:&str,bridge:bool,transfers:&crate::file_transfer::Store){
  self.gate.expire(Instant::now());
  ui.heading(if en{"Computer interaction · File Access"}else{"Взаємодія з комп’ютером · File Access"});
  ui.label(if en{"Local, manual requests only. No AI-generated commands, remote requests or file modifications. Only explicitly granted folders."}else{"Лише локальні ручні запити. Без команд від ШІ, віддалених дій або зміни файлів. Лише дозволені папки."});
  let mut enabled=self.gate.enabled;
  if ui.checkbox(&mut enabled,if en{"Enable actions for this session"}else{"Дозволити дії в поточному сеансі"}).changed(){self.gate.enable(enabled);if !enabled{transfers.clear();self.scopes.clear();self.apps.clear();self.clear_view();}self.status=if enabled{"Session permission granted / Дозвіл надано".into()}else{"Permission revoked / Дозвіл відкликано".into()};}
  if busy{self.screen=None;self.screen_attach=false;self.gate.cancel();self.search_pending=None;self.cancel_search();}
  if !bridge||!self.gate.enabled{transfers.clear();}
  if db.is_none(){self.screen=None;self.screen_attach=false;transfers.clear();ui.label("Action journal unavailable. Execution blocked / Журнал недоступний. Дії заблоковані.");self.gate.cancel();}
  let can=self.gate.enabled&&!busy&&db.is_some()&&self.gate.pending.is_none()&&self.search_job.is_none();
  for action in [Action::InspectStatus,Action::OpenData]{
   if ui.add_enabled(can&&(action!=Action::OpenData||cfg!(windows)),egui::Button::new(action.label(en))).clicked(){
    let target=match action{Action::InspectStatus=>Ok(None),Action::OpenData=>local_data().map(Some),_=>Err("Invalid quick action".into())};
    match target.and_then(|t|self.gate.propose(action,t,Instant::now())){Ok(())=>self.status="Review required / Потрібне підтвердження".into(),Err(e)=>self.status=e}
   }
  }
  ui.separator();ui.heading("Screen awareness / Аналіз екрана");
  ui.small("One screenshot of the primary monitor, including Hatoriel and any visible private content. No background capture. Review before adding to chat.");
  if ui.add_enabled(can&&cfg!(windows),egui::Button::new("Capture screen / Знімок екрана")).clicked(){
   match self.gate.propose(Action::CaptureScreen,None,Instant::now()){Ok(())=>self.status="Review required / Потрібне підтвердження".into(),Err(e)=>self.status=e}
  }
  if let Some(screen)=&self.screen{
   ui.label(format!("{} × {} source · JPEG {} KiB · RAM only",screen.source_size[0],screen.source_size[1],screen.bytes.len()/1024));
   ui.add(egui::Image::new(&screen.texture).max_size(egui::vec2(ui.available_width().min(760.),380.)));
   ui.collapsing("Inspect captured pixels / Детальний перегляд",|ui|{egui::ScrollArea::both().id_salt("screen-detail").max_height(380.).show(ui,|ui|{ui.add(egui::Image::new(&screen.texture));});});
   ui.horizontal(|ui|{
    if ui.add_enabled(can&&backend=="Ollama",egui::Button::new("Add to chat draft / Додати в чат")).clicked(){self.screen_attach=true;}
    if ui.button("Discard / Видалити").clicked(){self.screen=None;self.screen_attach=false;}
   });
   ui.small("Nothing is sent until you enter a question and confirm Vision in chat. / Надсилання лише після підтвердження в чаті.");
  }
  ui.separator();ui.heading("Application control / Керування програмами");
  ui.small("Trusted EXE only, no arguments or shell. Approval is session-only. Programs run with your account's permissions; revoke does not close existing processes.");
  ui.label(format!("{} tracked processes / Відстежувані процеси",self.apps.running_count()));
  #[cfg(windows)]
  if ui.add_enabled(can&&self.apps.entries().len()<crate::app_control::MAX_APPS,egui::Button::new("Choose EXE / Обрати програму")).clicked(){
   if let Some(path)=rfd::FileDialog::new().add_filter("Windows EXE",&["exe"]).pick_file(){self.request(Action::RegisterApp,path);}
  }
  let mut app_request=None;let mut app_revoke=None;
  for app in self.apps.entries(){
   ui.push_id(&app.path,|ui|{
    ui.label(&app.name);ui.small(app.path.display().to_string());
    ui.horizontal(|ui|{
     let running=self.apps.is_running(&app.path);
     if ui.add_enabled(can&&!running&&cfg!(windows),egui::Button::new("Launch / Запустити")).clicked(){app_request=Some(app.path.clone());}
     if running{ui.small("Tracked process running");}
     if ui.button("Revoke / Відкликати").clicked(){app_revoke=Some(app.path.clone());}
    });
   });
  }
  if let Some(path)=app_revoke{self.gate.cancel();self.apps.revoke(&path);app_request=None;self.status="App permission revoked; running process unaffected / Дозвіл відкликано".into();}
  if let Some(path)=app_request{if self.gate.pending.is_none(){self.request(Action::LaunchApp,path);}}
  ui.separator();ui.heading("File transfer / Передавання файлів");
  ui.small("One original file up to 8 MiB. Start the trusted HTTPS bridge in Devices first. Any client with the session key AND code can redeem once.");
  if let Some((code,name,size,seconds))=transfers.info(){ui.label(format!("{name} · {size} bytes · {seconds}s"));ui.monospace(&code);if ui.button("Copy one-time code / Копіювати код").clicked(){ui.ctx().copy_text(code);}if ui.button("Revoke transfer / Скасувати передавання").clicked(){transfers.clear();}}
  ui.separator();ui.heading("Read-only folders / Дозволені папки");
  ui.small("Session-only grants. No writes, bounded confirmed search, no automatic model access. / Дозволи лише на сеанс.");
  #[cfg(windows)]
  if ui.add_enabled(can&&self.scopes.roots().len()<crate::file_access::MAX_ROOTS,egui::Button::new("Choose folder / Обрати папку")).clicked(){
   if let Some(path)=rfd::FileDialog::new().pick_folder(){self.request(Action::GrantFolder,path);}
  }
  ui.collapsing("Search filters / Фільтри пошуку",|ui|{
   ui.add_enabled_ui(can,|ui|{
    ui.label("Name contains / Назва");ui.text_edit_singleline(&mut self.filter.name);
    ui.label("Extension / Розширення");ui.text_edit_singleline(&mut self.filter.extension);
    ui.label("Text contains (optional) / Текст містить (необов’язково)");ui.text_edit_singleline(&mut self.filter.content);
    ui.add(egui::DragValue::new(&mut self.filter.min_kib).range(0..=4_194_304).prefix("Min KiB: "));
    ui.add(egui::DragValue::new(&mut self.filter.max_kib).range(0..=4_194_304).prefix("Max KiB (0 = any): "));
    ui.add(egui::DragValue::new(&mut self.filter.recent_days).range(0..=3650).prefix("Modified within days (0 = any): "));
   });
  });
  if self.search_job.is_some(){ui.horizontal(|ui|{ui.spinner();ui.label("Searching / Пошук…");if ui.button("Stop / Зупинити").clicked(){self.cancel_search();}});}
  let mut revoke=None;let mut request=None;
  for root in self.scopes.roots(){
   ui.horizontal_wrapped(|ui|{ui.label(root.display().to_string());if ui.add_enabled(can,egui::Button::new("List / Вміст")).clicked(){request=Some((Action::ListFolder,root.clone()));}if ui.add_enabled(can,egui::Button::new("Search / Пошук")).clicked(){request=Some((Action::Search,root.clone()));}if ui.button("Revoke / Відкликати").clicked(){revoke=Some(root.clone());}});
  }
  if let Some(path)=revoke{self.gate.cancel();self.scopes.remove(&path);transfers.clear();self.clear_view();request=None;self.status="Folder access revoked / Доступ відкликано".into();}
  if let Some(current)=&self.current{
   ui.label(current.display().to_string());
   if let Some(parent)=current.parent(){if self.scopes.roots().iter().any(|r|parent.starts_with(r))&&ui.add_enabled(can,egui::Button::new("Up / Назад")).clicked(){request=Some((Action::ListFolder,parent.to_path_buf()));}}
   egui::ScrollArea::vertical().id_salt("computer-files").max_height(220.).show(ui,|ui|{
    for entry in &self.entries{ui.horizontal(|ui|{
     let title=if entry.dir{format!("[DIR] {}",entry.name)}else{format!("{} · {} bytes",entry.name,entry.bytes)};
     if ui.add_enabled(can,egui::Button::new(title)).clicked(){request=Some((if entry.dir{Action::ListFolder}else{Action::ReadText},entry.path.clone()));}
     if !entry.dir&&ui.add_enabled(can&&bridge&&entry.bytes<=crate::file_transfer::MAX_BYTES as u64,egui::Button::new("Send file / Файл на телефон")).clicked(){request=Some((Action::OfferFile,entry.path.clone()));}
    });}
   });
  }
  if let Some(report)=&self.search_report{
   ui.separator();ui.label(format!("{} files · {} scanned · {} skipped · {} text reads",report.hits.len(),report.scanned,report.skipped,report.text_reads));
   if report.limited{ui.label("Partial search: a limit was reached / Неповний пошук: досягнуто ліміту.");}
   egui::ScrollArea::vertical().id_salt("search-results").max_height(220.).show(ui,|ui|{
    for hit in &report.hits{ui.push_id(&hit.path,|ui|{
     ui.label(hit.path.display().to_string());ui.small(format!("{} bytes · modified UTC epoch: {}",hit.bytes,hit.modified.map(|x|x.to_string()).unwrap_or_else(||"unknown".into())));
     ui.horizontal(|ui|{
      if ui.add_enabled(can,egui::Button::new("Preview text / Текст")).clicked(){request=Some((Action::ReadText,hit.path.clone()));}
      if ui.add_enabled(can&&bridge&&hit.bytes<=crate::file_transfer::MAX_BYTES as u64,egui::Button::new("Send file / Файл на телефон")).clicked(){request=Some((Action::OfferFile,hit.path.clone()));}
      if let Some(parent)=hit.path.parent(){if ui.add_enabled(can,egui::Button::new("Parent folder / Папка")).clicked(){request=Some((Action::ListFolder,parent.to_path_buf()));}}
     });
    });}
   });
  }
  if let Some((action,target))=request{if self.gate.pending.is_none(){
   if action==Action::Search{match self.filter.validate(){Ok(())=>{self.search_pending=Some(self.filter.clone());self.request(action,target);},Err(e)=>self.status=e}}
   else{self.search_pending=None;self.request(action,target);}
  }}
  if let Some((path,text))=&self.preview{ui.label(path.display().to_string());egui::ScrollArea::vertical().id_salt("computer-file-preview").max_height(180.).show(ui,|ui|{ui.label(text);});if ui.button("Close preview / Закрити").clicked(){self.preview=None;}}
  if let Some(p)=self.gate.pending.as_ref(){
   ui.separator();ui.label(egui::RichText::new("REVIEW / ПІДТВЕРДЖЕННЯ").color(crate::crimson::RED).strong());
   ui.label(p.action.label(en));
   if let Some(path)=&p.target{ui.label(path.display().to_string());ui.small(match p.action{Action::OpenData=>"Opens Explorer, no content read.",Action::GrantFolder=>"Grants read-only access for this session, not automatic scanning.",Action::ListFolder=>"Lists at most 200 immediate entries, no recursion.",Action::ReadText=>"Reads at most 64 KiB of supported plain text, displayed locally.",Action::Search=>"Recursive scan: up to 4000 entries, 200 results, 64 text reads, cooperative 5s limit. No upload.",Action::OfferFile=>"Creates a memory-only copy of the original file for one authenticated HTTPS download within 120 seconds. Not bound to one device; protect both codes.",Action::RegisterApp=>"Trust this specific executable for this session. Its SHA-256 will be recorded in memory; only choose software you trust.",Action::LaunchApp=>"Launch this executable without arguments. It runs with your user permissions and can act independently. No sandbox or elevation request.",_=>""});}
   else if p.action==Action::CaptureScreen{ui.small("Capture the current visible primary monitor now, including sensitive windows. Close private content first. No file write or model request.");}
   else{ui.small("Displays current backend, selected model, app storage mode and bridge state. / Лише стан застосунку.");}
   if p.action==Action::Search{if let Some(f)=&self.search_pending{ui.label(format!("Name: {:?} · Extension: {:?} · Content: {:?} · size {}–{} KiB (0 max = any) · recent {} days",f.name,f.extension,f.content,f.min_kib,f.max_kib,f.recent_days));}}
   ui.small(format!("{} s · one-time approval",TTL.saturating_sub(p.created.elapsed()).as_secs()));
   let mut approve=false;let mut cancel=false;
   ui.horizontal(|ui|{
    approve=ui.add_enabled(!busy&&self.gate.enabled&&db.is_some()&&(p.action!=Action::OfferFile||bridge),egui::Button::new("Confirm once / Підтвердити один раз")).clicked();
    cancel=ui.button("Cancel / Скасувати").clicked();
   });
   if cancel{self.gate.cancel();self.search_pending=None;self.status="Cancelled / Скасовано".into();}
   else if approve{
    match self.gate.approve(Instant::now()){
     Ok(p)=>if let Some(db)=db{
      match begin(db,p.action){
       Ok(id)=>{
        if p.action==Action::Search{
         let result=match (p.target,self.search_pending.take()){(Some(root),Some(filter))=>self.start_search(id,root,filter),_=>Err("Missing approved search parameters".into())};
         match result{Ok(())=>self.status="Search started / Пошук розпочато".into(),Err(e)=>{self.status=e;if finish(db,id,"failed").is_err(){self.gate.enable(false);self.scopes.clear();self.clear_view();}}}
         return;
        }
        let result=match p.action{
         Action::Search=>Err("Search dispatch error".into()),
         Action::CaptureScreen=>{self.screen=None;self.screen_attach=false;crate::screen_awareness::capture(ui.ctx()).map(|shot|{self.screen=Some(shot);"Screenshot captured locally / Знімок готовий до перегляду".into()})},
         Action::RegisterApp=>p.target.as_deref().ok_or_else(||"Missing executable".to_string()).and_then(|path|self.apps.register(path)).map(|_|"Executable approved for this session / Програму дозволено".into()),
         Action::LaunchApp=>p.target.as_deref().ok_or_else(||"Missing executable".to_string()).and_then(|path|self.apps.launch(path)).map(|pid|format!("Launch requested · PID {pid} / Запуск запитано")),
         Action::OfferFile=>if !bridge{Err("Bridge stopped".into())}else{p.target.as_deref().ok_or_else(||"Missing file".to_string()).and_then(|path|transfers.prepare(&self.scopes,path)).map(|_|"File prepared; enter one-time code on phone / Файл підготовлено".into())},
         Action::InspectStatus=>Ok(format!("Backend: {backend}\nModel: {model}\nStorage: {}\nBridge: {}",if crate::storage::portable(){"portable"}else{"installed"},if bridge{"running"}else{"stopped"})),
         Action::OpenData=>p.target.as_deref().ok_or_else(||"Missing approved target".to_string()).and_then(open_data),
         Action::GrantFolder=>p.target.as_deref().ok_or_else(||"Missing folder".to_string()).and_then(|path|self.scopes.grant(path)).map(|_|"Read-only grant added / Дозвіл надано".into()),
         Action::ListFolder=>p.target.as_deref().ok_or_else(||"Missing folder".to_string()).and_then(|path|self.scopes.list(path)).map(|(entries,capped)|{self.current=p.target.clone();self.preview=None;let n=entries.len();self.entries=entries;format!("{n} entries{}",if capped{" · capped at 200 scanned entries"}else{""})}),
         Action::ReadText=>p.target.as_deref().ok_or_else(||"Missing file".to_string()).and_then(|path|self.scopes.read_text(path)).map(|text|{self.preview=p.target.clone().map(|path|(path,text));"Local text preview / Локальне прев’ю".into()}),
        };
        let outcome=if result.is_ok(){if matches!(p.action,Action::OpenData|Action::LaunchApp){"launch_requested"}else if p.action==Action::OfferFile{"offer_created"}else{"completed"}}else{"failed"};
        self.status=match result{Ok(s)=>s,Err(e)=>format!("Action failed: {e}")};
        if finish(db,id,outcome).is_err(){self.status.push_str("\nAudit completion write failed; initial approval remains recorded.");self.gate.enable(false);transfers.clear();self.scopes.clear();self.apps.clear();self.clear_view();}
       },Err(e)=>{self.status=format!("Action blocked: audit write failed: {e}");transfers.clear();self.gate.enable(false);self.scopes.clear();self.apps.clear();self.clear_view();}
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
 #[test]fn screen_confirmation_is_one_shot(){let now=Instant::now();let mut g=Gate::default();assert!(g.propose(Action::CaptureScreen,None,now).is_err());g.enable(true);assert!(g.propose(Action::CaptureScreen,Some(PathBuf::from("invalid")),now).is_err());g.propose(Action::CaptureScreen,None,now).unwrap();assert_eq!(g.approve(now).unwrap().action,Action::CaptureScreen);assert!(g.approve(now).is_err());}
 #[test]fn journal_bounds(){let db=Connection::open_in_memory().unwrap();for _ in 0..MAX_AUDIT+3{let id=begin(&db,Action::InspectStatus).unwrap();finish(&db,id,"completed").unwrap();}let count:i64=db.query_row("SELECT COUNT(*) FROM computer_audit",[],|r|r.get(0)).unwrap();assert_eq!(count,MAX_AUDIT as i64);let rows=recent(&db).unwrap();assert_eq!(rows.len(),30);assert_eq!(rows[0].1,"inspect_status");assert_eq!(rows[0].2,"completed");}
 #[test]fn audit_fail_closed(){let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE computer_audit(wrong INTEGER);").unwrap();assert!(begin(&db,Action::InspectStatus).is_err());}
}
