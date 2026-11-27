#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui::{self, RichText};
mod presence;
mod sound;
mod hardware;
mod autostart;
mod native_runtime;
mod resources;
mod native_library;
mod storage;
mod memory_store;
mod conversations;
mod desktop_bridge;
mod crimson;
mod voice;
mod neural_voice;
mod speech_recognition;
mod attachments;
mod documents;
use hatori_ai::{core, memory_suggest, proactive, emotion, adaptive, memory_core, model_manager, inference::{Backend, Ollama, Message}, llama_backend::Llama};
use serde::{Serialize, Deserialize};
use std::{sync::mpsc, time::Duration};
use rusqlite::Connection;
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Config { model: String, traits: [u8;7], mode: String, english: bool, avatar: bool, minimize_to_tray: bool, dark: bool, sound: bool, volume: u8, setup_complete: bool, native_backend: bool, gguf_path:String, runtime_path:String, gpu_layers:u32, context_size:u32, cpu_threads:u32, model_dir:String, profile:adaptive::Profile, suggest_memories:bool, semantic_search:bool, embedding_model:String, emotion_enabled:bool, mood:emotion::State, dynamic_style:bool, active_chat:i64, proactive:proactive::Settings, neural_voice:neural_voice::Settings, avatar_motion:bool, pdftotext:String, multimodal_budget:usize }
impl Default for Config { fn default()->Self { Self { model:"qwen2.5:3b".into(), traits:[90,80,90,90,90,80,70], mode:"Companion".into(), english:false, avatar:true, minimize_to_tray:true, dark:true, sound:true, volume:35, setup_complete:false, native_backend:false, gguf_path:String::new(), runtime_path:String::new(), gpu_layers:0, context_size:2048, cpu_threads:4, model_dir:String::new(), profile:Default::default(), suggest_memories:true, semantic_search:false, embedding_model:"bge-m3".into(), emotion_enabled:true, mood:Default::default(), dynamic_style:true, active_chat:1, proactive:Default::default(),neural_voice:Default::default(),avatar_motion:true,pdftotext:String::new(),multimodal_budget:24000 } } }
const TRAITS:[&str;7]=["Аналітичність","Емпатія","Допитливість","Гумор","Ініціативність","Емоційність","Сарказм"];
const EN:[&str;7]=["Analysis","Empathy","Curiosity","Humor","Initiative","Emotion","Sarcasm"];
enum Event { DocumentExtract(u64,Result<documents::Extracted,String>), SyncExport(Result<String,String>), SyncPreview(Result<(String,String,hatori_ai::sync::Plan),String>), Cancelled, Catalog(Result<Vec<native_library::Entry>,String>), Token(String), Done, Error(String), Models(Vec<String>), ModelError(String), Hardware(Result<hardware::Hardware,String>), PullProgress(String,Option<f32>), PullDone(Result<(),String>) }
struct App { presence: presence::Presence, db: Option<Connection>, config:Config, messages:Vec<Message>, memories:Vec<memory_store::Memory>, input:String, memory:String, status:String, page:usize, tx:mpsc::Sender<Event>, rx:mpsc::Receiver<Event>, busy:bool, draft:String, models:Vec<String>, model_status:String, hardware:Option<hardware::Hardware>, hardware_status:String, pulling:bool, pull_status:String, pull_progress:Option<f32>, autostart:bool, memory_search:String, memory_category:String, memory_sort:u8, runtime:native_runtime::Runtime, catalog:Vec<native_library::Entry>, scanning:bool, catalog_status:String, cancel:std::sync::Arc<std::sync::atomic::AtomicBool>, started:Option<std::time::Instant>, chunks:u64, suggestions:Vec<memory_suggest::Suggestion>, chats:Vec<(i64,String)>, chat_title:String, chat_search:String, confirm_delete:bool, reminder_text:String, reminder_minutes:u32, proactive_notice:Option<String>, proactive_check:Option<std::time::Instant>, bridge:desktop_bridge::Bridge, reveal_bridge_token:bool, sync_package:String, sync_password:String, sync_preview:Option<(String,String,hatori_ai::sync::Plan)>, sync_profile:bool, sync_personality:bool, sync_working:bool, voice:voice::Voice, speech:speech_recognition::Capture, speech_language:String, push_to_talk:bool, voice_mode:bool, voice_reply:bool, attachments:attachments::Draft, vision_confirm:bool, doc_epoch:u64, doc_loading:bool, doc_preview:Option<documents::Extracted>, doc_selected:usize, doc_checked:Vec<usize>, doc_ranked:Vec<(usize,u32)> }
impl App {
fn new(cc:&eframe::CreationContext<'_>)->Self {
 cc.egui_ctx.set_visuals(crimson::visuals(true));
 let dir=storage::data_dir();
 let db=std::fs::create_dir_all(&dir).and_then(|_| Ok(())).ok().and_then(|_|Connection::open(dir.join("memory.sqlite3")).ok()).filter(|d|d.execute_batch("CREATE TABLE IF NOT EXISTS messages(id INTEGER PRIMARY KEY, role TEXT, content TEXT); CREATE TABLE IF NOT EXISTS memories(id INTEGER PRIMARY KEY, content TEXT); CREATE TABLE IF NOT EXISTS config(id INTEGER PRIMARY KEY, value TEXT);").is_ok() && memory_store::migrate(d).is_ok() && conversations::migrate(d).is_ok());
 let mut config=Config::default(); let mut messages=vec![]; let mut memories=vec![];
 if let Some(d)=&db { if let Ok(s)=d.query_row("SELECT value FROM config WHERE id=1",[],|r|r.get::<_,String>(0)){config=serde_json::from_str(&s).unwrap_or_default();}  memories=memory_store::load(d).unwrap_or_default(); }
 let mut chats=vec![];if let Some(d)=&db{chats=conversations::list(d).unwrap_or_default();if chats.is_empty(){if let Ok(id)=conversations::create(d){chats.push((id,"New chat".into()));}}if !chats.iter().any(|c|c.0==config.active_chat){config.active_chat=chats.first().map(|c|c.0).unwrap_or(1);}messages=conversations::load(d,config.active_chat).unwrap_or_default();}let chat_title=chats.iter().find(|c|c.0==config.active_chat).map(|c|c.1.clone()).unwrap_or_default();if storage::portable(){if config.runtime_path.is_empty(){config.runtime_path="runtime/llama-server.exe".into();}if config.model_dir.is_empty(){config.model_dir="models".into();}} let status=if db.is_some(){"Ready / local storage"}else{"Storage unavailable — session only"}.into(); let(tx,rx)=mpsc::channel(); let initial_page=if config.setup_complete {0}else{4}; let app=Self{presence:presence::Presence::new(),db,config,messages,memories,input:String::new(),memory:String::new(),status,page:initial_page,tx,rx,busy:false,draft:String::new(),models:vec![],model_status:"Checking Ollama…".into(),hardware:None,hardware_status:"Scanning…".into(),pulling:false,pull_status:String::new(),pull_progress:None,autostart:autostart::enabled(),memory_search:String::new(),memory_category:String::new(),memory_sort:0,runtime:native_runtime::Runtime::default(),catalog:vec![],scanning:false,catalog_status:String::new(),cancel:Default::default(),started:None,chunks:0,suggestions:vec![],chats,chat_title,chat_search:String::new(),confirm_delete:false,reminder_text:String::new(),reminder_minutes:30,proactive_notice:None,proactive_check:None,bridge:Default::default(),reveal_bridge_token:false,sync_package:String::new(),sync_password:String::new(),sync_preview:None,sync_profile:false,sync_personality:false,sync_working:false,voice:Default::default(),speech:Default::default(),speech_language:"uk-UA".into(),push_to_talk:false,voice_mode:false,voice_reply:false,attachments:Default::default(),vision_confirm:false,doc_epoch:0,doc_loading:false,doc_preview:None,doc_selected:0,doc_checked:vec![],doc_ranked:vec![]}; app.refresh(); let tx=app.tx.clone(); std::thread::spawn(move||{let _=tx.send(Event::Hardware(hardware::scan()));}); app
}
fn dictation_panel(&mut self,ui:&mut egui::Ui,en:bool){
 ui.collapsing("Neural TTS / Нейронний голос · Preview",|ui|{
  let mut changed=false;
  ui.add_enabled_ui(!self.busy&&!self.speech.active()&&!self.voice.active(),|ui|{
   changed|=ui.checkbox(&mut self.config.neural_voice.enabled,"Use local Piper / Локальний Piper").changed();
   if self.config.neural_voice.enabled{
    ui.label("Trusted local Python .exe with piper-tts installed / Довірений Python");
    changed|=ui.text_edit_singleline(&mut self.config.neural_voice.python).changed();
    ui.label("ONNX voice file + adjacent .onnx.json / Голосова модель");
    changed|=ui.text_edit_singleline(&mut self.config.neural_voice.model).changed();
    changed|=ui.add(egui::Slider::new(&mut self.config.neural_voice.speed,0.65..=1.5).text("Speed / Швидкість")).changed();
    changed|=ui.add(egui::Slider::new(&mut self.config.neural_voice.volume,0.0..=1.0).text("Gain / Гучність")).changed();
    ui.label("User-installed runtime; no automatic download or fallback. / Без автоматичного завантаження.");
    if ui.button("Test voice / Тест голосу").clicked(){
     let text=if self.speech_language=="en-US"{"Hello. I am Hatoriel."}else{"Вітаю. Я Hatoriel."};
     if let Err(e)=self.voice.speak(text,self.speech_language=="en-US",&self.config.neural_voice){self.speech.status=e;}
    }
   }
  });
  if changed{self.save();}
 });
 if ui.checkbox(&mut self.voice_mode,if en{"Voice dialogue (read new replies)"}else{"Голосовий діалог (озвучувати нові відповіді)"}).changed()&&!self.voice_mode{self.voice_reply=false;self.voice.stop();}
 if self.voice_mode{ui.label(if self.speech.active(){"LISTENING / Запис"}else if self.busy{"THINKING / Обробка"}else if self.voice.synthesizing(){"SYNTHESIZING / Синтез голосу"}else if self.voice.active(){"SPEAKING / Озвучування"}else{"READY / Наступна репліка"});ui.label("Confirm text before sending / Підтверджуй текст перед надсиланням.");}
 ui.horizontal_wrapped(|ui|{
  ui.add_enabled_ui(!self.speech.active(),|ui|{
   egui::ComboBox::from_id_salt("stt_language").selected_text(&self.speech_language).show_ui(ui,|ui|{
    ui.selectable_value(&mut self.speech_language,"uk-UA".into(),"Українська");ui.selectable_value(&mut self.speech_language,"en-US".into(),"English");
   });
  });
  if self.speech.active(){
   if ui.button(if en{"Finish recording"}else{"Завершити запис"}).clicked(){self.speech.stop();self.push_to_talk=false;}
   if ui.button(if en{"Cancel recording"}else{"Скасувати запис"}).clicked(){self.speech.cancel();self.push_to_talk=false;}
  }else if ui.add_enabled(!self.busy,egui::Button::new(if en{"Dictate"}else{"Диктувати"})).clicked(){
   self.voice.stop();if let Err(e)=self.speech.start(&self.speech_language){self.speech.status=e;}
  }
  let hold=ui.add_enabled(!self.busy&&(!self.speech.active()||self.push_to_talk),egui::Button::new(if en{"Hold to talk"}else{"Утримуй для запису"}));
  if hold.is_pointer_button_down_on()&&!self.speech.active()&&!self.push_to_talk{
   self.voice.stop();match self.speech.start(&self.speech_language){Ok(())=>self.push_to_talk=true,Err(e)=>self.speech.status=e}
  }
  if self.speech.listening{ui.label(RichText::new("● REC").color(crimson::RED));}else if self.speech.active(){ui.spinner();}
 });
 ui.label(if en{"Uses the Windows default microphone and installed desktop speech recognizer. Change input device in Windows Settings → Sound."}else{"Системний мікрофон Windows. Змінити пристрій: Налаштування Windows → Звук. Потрібен встановлений настільний мовний пакет."});
 if !self.speech.status.is_empty(){ui.label(&self.speech.status);}
 if !self.speech.text.is_empty(){
  ui.label(&self.speech.text);
  ui.horizontal(|ui|{
   if ui.add_enabled(!self.busy&&!self.speech.active(),egui::Button::new(if en{"Add to draft"}else{"Додати до чернетки"})).clicked(){
    self.input=speech_recognition::append_draft(&self.input,&self.speech.text);self.speech.text.clear();
   }
   if ui.add_enabled(!self.speech.active(),egui::Button::new(if en{"Discard text"}else{"Відхилити текст"})).clicked(){self.speech.text.clear();}
  });
 }
}
fn invalidate_docs(&mut self){self.doc_epoch=self.doc_epoch.wrapping_add(1);self.doc_preview=None;self.doc_selected=0;self.doc_checked.clear();self.doc_ranked.clear();}
fn document_panel(&mut self,ui:&mut egui::Ui){
 if self.attachments.empty()||self.attachments.image_only(){return;}
 let images=self.attachments.image_count();
 if !self.attachments.items.iter().filter(|a|a.preview.is_none()).all(|a|documents::supported(&a.name)){ui.label("Unsupported document / Непідтримуваний документ");return;}
 ui.collapsing("Document analysis / Аналіз документів",|ui|{
  if images>0{ui.label(format!("Mixed request: {images} images + reviewed excerpts / Змішаний запит"));if self.config.native_backend{ui.label("Mixed image requests require Ollama / Потрібен Ollama.");}}
  if self.attachments.items.iter().any(|a|a.preview.is_none()&&a.name.to_lowercase().ends_with(".pdf")){
   ui.label("Trusted local pdftotext.exe path / Шлях до локального pdftotext.exe");
   if ui.add_enabled(!self.doc_loading,egui::TextEdit::singleline(&mut self.config.pdftotext)).changed(){self.invalidate_docs();self.save();}
  }
  if ui.add_enabled(!self.busy&&!self.speech.active()&&!self.doc_loading,egui::Button::new("Extract text locally / Вилучити текст")).clicked(){
   self.invalidate_docs();let epoch=self.doc_epoch;let files=self.attachments.items.iter().filter(|a|a.preview.is_none()).map(|a|(a.name.clone(),a.bytes.clone())).collect();let pdf=self.config.pdftotext.clone();let tx=self.tx.clone();self.doc_loading=true;
   if let Err(e)=std::thread::Builder::new().name("hatoriel-documents".into()).spawn(move||{let result=std::panic::catch_unwind(||documents::extract(files,&pdf)).unwrap_or_else(|_|Err("Document parser failed".into()));let _=tx.send(Event::DocumentExtract(epoch,result));}){self.doc_loading=false;self.attachments.status=e.to_string();}
  }
  if self.doc_loading{ui.spinner();ui.label("Extracting / Обробка…");}
  let mut chosen=None;
  if let Some(ref preview)=self.doc_preview{
   for note in &preview.notes{ui.small(note);}
   if !preview.sections.is_empty(){
    if ui.add_enabled(!self.busy&&!self.input.trim().is_empty(),egui::Button::new("Find relevant excerpts / Знайти релевантні фрагменти")).clicked(){
     let texts:Vec<_>=preview.sections.iter().map(|s|s.text.as_str()).collect();self.doc_ranked=hatori_ai::multimodal::rank(&self.input,&texts);
     self.doc_checked=self.doc_ranked.iter().take(hatori_ai::multimodal::MAX_SELECTED).map(|(i,_)|*i).collect();
     if let Some(&i)=self.doc_checked.first(){self.doc_selected=i;}
    }
    if !self.doc_ranked.is_empty(){ui.small(format!("{} matching passages · lexical search / Пошук за словами",self.doc_ranked.len()));}
    else{ui.small("No ranked results yet; choose excerpts manually / Обери фрагменти вручну.");}
    self.doc_selected=self.doc_selected.min(preview.sections.len()-1);
    egui::ComboBox::from_id_salt("doc-section").selected_text(format!("{}/{}",self.doc_selected+1,preview.sections.len())).show_ui(ui,|ui|{for (i,section) in preview.sections.iter().enumerate(){ui.selectable_value(&mut self.doc_selected,i,format!("{} · {}/{}",section.name,section.number,section.total));}});
    let section=&preview.sections[self.doc_selected];
    let mut checked=self.doc_checked.contains(&self.doc_selected);
    if ui.add_enabled(checked||self.doc_checked.len()<hatori_ai::multimodal::MAX_SELECTED,egui::Checkbox::new(&mut checked,"Include this excerpt / Додати фрагмент")).changed(){
     if checked{self.doc_checked.push(self.doc_selected);}else{self.doc_checked.retain(|i|*i!=self.doc_selected);}
    }
    egui::ScrollArea::vertical().id_salt("doc-preview").max_height(150.).show(ui,|ui|{ui.label(&section.text);});
    self.doc_checked.sort_unstable();self.doc_checked.dedup();
    let reference=self.doc_checked.iter().filter_map(|i|preview.sections.get(*i)).map(|s|format!("File: {} | excerpt {}/{}\n{}",s.name,s.number,s.total,s.text)).collect::<Vec<_>>().join("\n\n---\n\n");
    let count=reference.chars().count();
    ui.label(format!("Selected: {}/3 · {} chars / Вибрано",self.doc_checked.len(),count));
    ui.collapsing("Exact selected text / Увесь вибраний текст",|ui|{egui::ScrollArea::vertical().id_salt("doc-combined").max_height(180.).show(ui,|ui|{ui.label(&reference);});});
    ui.label("Only selected text and attached images (if any) will be sent to the configured local backend. Responses may quote documents and remain in history.");
    let ok=!self.busy&&!self.speech.active()&&!self.input.trim().is_empty()&&count>0&&count<=hatori_ai::multimodal::MAX_REFERENCE&&(images==0||!self.config.native_backend);
    if ui.add_enabled(ok,egui::Button::new("Підтвердити вибраний контекст і надіслати / Send reviewed context")).clicked(){chosen=Some(reference);}
   }
  }
  if let Some(reference)=chosen{if images>0{self.vision_confirm=true;}self.send_inner(images>0,Some(reference));self.vision_confirm=false;}
 });
}
fn sync_snapshot(&self)->Result<hatori_ai::sync::Snapshot,String>{
 let records=self.memories.iter().map(|m|hatori_ai::sync::Record{content:m.content.clone(),category:m.category.clone(),importance:m.importance}).collect();
 let snapshot=hatori_ai::sync::Snapshot{version:1,records,profile:self.config.profile.clone(),options:hatori_ai::mobile_context::Options{mode:self.config.mode.clone(),traits:self.config.traits,dynamic_style:self.config.dynamic_style,emotion_enabled:self.config.emotion_enabled,mood:Default::default()}};
 snapshot.validate()?;Ok(snapshot)
}
fn apply_sync(&mut self,plan:hatori_ai::sync::Plan)->Result<(),String>{
 if self.busy{return Err("Generation active".into());}
 let mut config=self.config.clone();
 if self.sync_profile{config.profile=plan.profile;}
 if self.sync_personality{config.mode=plan.options.mode;config.traits=plan.options.traits;config.dynamic_style=plan.options.dynamic_style;config.emotion_enabled=plan.options.emotion_enabled;config.mood=Default::default();}
 let d=self.db.as_mut().ok_or("Storage unavailable")?;
 let tx=d.transaction().map_err(|e|e.to_string())?;
 let created=emotion::now() as i64;
 for row in plan.additions{tx.execute("INSERT INTO memories(content) VALUES(?1)",[row.content]).map_err(|e|e.to_string())?;let id=tx.last_insert_rowid();tx.execute("INSERT INTO memory_metadata VALUES(?1,?2,?3,?4)",rusqlite::params![id,row.category,row.importance,created]).map_err(|e|e.to_string())?;}
 tx.execute("INSERT OR REPLACE INTO config VALUES(1,?1)",[serde_json::to_string(&config).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;
 self.config=config;self.memories=memory_store::load(d).map_err(|e|e.to_string())?;Ok(())
}
fn sync_panel(&mut self,ui:&mut egui::Ui){
 ui.separator();ui.heading("Encrypted transfer / Захищене перенесення");
 ui.label("Memory, profile, personality only. No keys or chat history. / Пароль передавай окремо.");
 ui.add_enabled(!self.sync_working,egui::TextEdit::singleline(&mut self.sync_password).password(true).hint_text("Passphrase / Парольна фраза (12+)"));
 if self.sync_working{ui.spinner();ui.label("Processing / Обробка…");}
 if ui.add_enabled(!self.sync_working,egui::TextEdit::multiline(&mut self.sync_package).desired_rows(4).desired_width(f32::INFINITY).hint_text("Encrypted package")).changed(){self.sync_preview=None;}
 ui.horizontal(|ui|{
  let ready=!self.busy&&!self.sync_working;
  let export=ui.add_enabled(ready,egui::Button::new("Export / Створити")).clicked();
  if ui.add_enabled(!self.sync_working&&!self.sync_package.is_empty(),egui::Button::new("Copy / Копіювати")).clicked(){ui.ctx().copy_text(self.sync_package.clone());}
  let preview=ui.add_enabled(ready,egui::Button::new("Preview / Переглянути")).clicked();
  if export||preview{
   let pass=zeroize::Zeroizing::new(std::mem::take(&mut self.sync_password));
   self.sync_preview=None;self.sync_profile=false;self.sync_personality=false;
   match self.sync_snapshot(){
    Err(e)=>self.status=e,
    Ok(snapshot)=>{
     if preview&&self.sync_package.len()>2*1024*1024{self.status="Package too large".into();return;}
     let package=if preview{self.sync_package.clone()}else{String::new()};
     let tx=self.tx.clone();self.sync_working=true;
     let spawned=std::thread::Builder::new().name("hatoriel-sync".into()).spawn(move||{
      let event=if export{
       Event::SyncExport(std::panic::catch_unwind(std::panic::AssertUnwindSafe(||hatori_ai::sync::seal(snapshot,&pass))).unwrap_or_else(|_|Err("Sync worker failed".into())))
      }else{
       Event::SyncPreview(std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<_,String>{
        let baseline=serde_json::to_string(&snapshot).map_err(|e|e.to_string())?;
        let incoming=hatori_ai::sync::open(&package,&pass)?;
        Ok((baseline,package,hatori_ai::sync::plan(&snapshot,incoming)?))
       })).unwrap_or_else(|_|Err("Sync worker failed".into())))
      };let _=tx.send(event);
     });
     if let Err(e)=spawned{self.sync_working=false;self.status=e.to_string();}
    }
   }
  }
 });
 if let Some((_,_,plan))=&self.sync_preview{
  ui.label(format!("New / Нові: {} · duplicates: {} · conflicts: {} (local wins)",plan.additions.len(),plan.duplicates,plan.conflicts));
  egui::ScrollArea::vertical().id_salt("sync_review").max_height(160.).show(ui,|ui|{
   for row in &plan.additions{ui.label(format!("{} · {} / 100",row.category,row.importance));ui.label(&row.content);}
   ui.collapsing("Incoming profile / Профіль",|ui|{ui.label(&plan.profile.interests);ui.label(&plan.profile.communication);ui.label(&plan.profile.goals);});
   ui.label(format!("Incoming personality: {} {:?}",plan.options.mode,plan.options.traits));
  });
  ui.checkbox(&mut self.sync_profile,"Replace profile / Замінити профіль");ui.checkbox(&mut self.sync_personality,"Replace personality / Замінити особистість");
  if ui.add_enabled(!self.busy&&!self.sync_working,egui::Button::new("Confirm merge / Підтвердити")).clicked(){
   if let Some((baseline,package,plan))=self.sync_preview.take(){
    let current=self.sync_snapshot().and_then(|s|serde_json::to_string(&s).map_err(|e|e.to_string()));
    if current.as_ref().ok()!=Some(&baseline)||package!=self.sync_package{self.status="Data changed; preview again".into();}
    else{self.status=match self.apply_sync(plan){Ok(())=>"Imported / Імпортовано".into(),Err(e)=>e};}
   }
  }
 }
}
fn save(&mut self){if let Some(d)=&self.db {if let Err(e)=d.execute("INSERT OR REPLACE INTO config VALUES(1,?1)",[serde_json::to_string(&self.config).unwrap_or_default()]){self.status=e.to_string();}}}
fn add(&mut self,role:&str,content:String){if let Some(d)=&self.db{if let Err(e)=conversations::append(d,self.config.active_chat,role,&content){self.status=e.to_string();}} self.messages.push(Message{role:role.into(),content});}
fn switch_chat(&mut self,id:i64){if self.busy{return;}self.voice.stop();self.speech.cancel();self.voice_reply=false;if let Some(d)=&self.db{match conversations::load(d,id){Ok(m)=>{self.messages=m;self.config.active_chat=id;self.chat_title=self.chats.iter().find(|c|c.0==id).map(|c|c.1.clone()).unwrap_or_default();self.input.clear();self.attachments.clear();self.vision_confirm=false;self.invalidate_docs();self.chat_search.clear();self.confirm_delete=false;self.save();},Err(e)=>self.status=e.to_string()}}}
fn send(&mut self){self.send_inner(false,None);}
fn send_inner(&mut self,vision:bool,reference:Option<String>){if self.busy||self.speech.active()||self.input.trim().is_empty(){return;}if !self.attachments.empty()&&!vision&&reference.is_none(){return;}
 let image_count=if vision{self.attachments.image_count()}else{0};
 if (vision||reference.is_some())&&self.input.chars().count()>hatori_ai::multimodal::MAX_QUERY{self.attachments.status="Query exceeds 12000 characters".into();return;}
 let image_backend=if vision{if self.config.native_backend||!self.vision_confirm||image_count==0||(!self.attachments.image_only()&&reference.is_none()){return;}match self.attachments.normalized().and_then(hatori_ai::vision::Vision::new){Ok(v)=>Some(v),Err(e)=>{self.attachments.status=e;return;}}}else{None};
 if vision||reference.is_some(){self.vision_confirm=false;self.attachments.clear();self.invalidate_docs();} self.voice.stop();self.voice_reply=self.voice_mode; let input=std::mem::take(&mut self.input); let texts:Vec<_>=self.memories.iter().map(|m|(m.content.clone(),m.importance,m.id)).collect();let query=input.clone();if self.config.emotion_enabled{self.config.mood.observe(&input,emotion::now(),self.config.traits[5]);self.save();} if self.config.suggest_memories{if let Some(candidate)=memory_suggest::suggest(&input){if self.suggestions.len()<20&&!self.memories.iter().any(|m|memory_suggest::duplicate(&m.content,&candidate.content))&&!self.suggestions.iter().any(|m|memory_suggest::duplicate(&m.content,&candidate.content)){self.suggestions.push(candidate);}}}if self.messages.is_empty(){if let Some(d)=&self.db{let title=input.chars().take(48).collect::<String>();if conversations::rename(d,self.config.active_chat,&title).is_ok(){self.chat_title=title;self.chats=conversations::list(d).unwrap_or_default();}}}self.add("user",input); self.busy=true; self.cancel=Default::default();let cancel=self.cancel.clone();self.started=Some(std::time::Instant::now());self.chunks=0; self.draft.clear(); self.status="Generating…".into(); let tx=self.tx.clone(); let cfg=self.config.clone();let req=core::Request{model:cfg.model,mode:cfg.mode,traits:cfg.traits,query,history:core::recent_context(&self.messages,32,12000),memories:texts,profile:cfg.profile,mood:if cfg.emotion_enabled{Some(cfg.mood)}else{None},dynamic_style:cfg.dynamic_style,embedding_model:if cfg.semantic_search{Some(cfg.embedding_model)}else{None}};
 std::thread::spawn(move||{let backend:&dyn Backend=if let Some(ref v)=image_backend{v}else if cfg.native_backend{&Llama}else{&Ollama};core::run_with_multimodal(backend,&req,reference.as_deref(),image_count,cfg.multimodal_budget,&||cancel.load(std::sync::atomic::Ordering::Relaxed),|event|tx.send(match event{core::Event::Token(t)=>Event::Token(t),core::Event::Done=>Event::Done,core::Event::Cancelled=>Event::Cancelled,core::Event::Error(e)=>Event::Error(e)}).is_ok());});
}
fn pull(&mut self){if self.config.native_backend||self.pulling||self.busy||self.config.model.trim().is_empty(){return;} self.pulling=true;self.pull_progress=None;self.pull_status="Connecting…".into();let model=self.config.model.trim().to_owned();let tx=self.tx.clone();std::thread::spawn(move||{let result=model_manager::pull(&model,|status,progress|{let _=tx.send(Event::PullProgress(status,progress));}).map_err(|e|e.to_string());let _=tx.send(Event::PullDone(result));});}
fn refresh(&self){let native=self.config.native_backend;let tx=self.tx.clone();std::thread::spawn(move||{let _=tx.send(match if native{Llama.models()}else{Ollama.models()}{Ok(m)=>Event::Models(m),Err(e)=>Event::ModelError(e)});});}
}
impl eframe::App for App {fn update(&mut self,ctx:&egui::Context,_:&mut eframe::Frame){
 self.speech.poll();
 if self.page!=0||self.busy{self.vision_confirm=false;}
 if self.page==0&&!self.busy&&!self.speech.active(){
  let dropped=ctx.input(|i|i.raw.dropped_files.clone());
  for file in dropped.into_iter().take(attachments::MAX_COUNT){self.vision_confirm=false;self.invalidate_docs();let result=if let Some(path)=file.path{self.attachments.file(ctx,&path)}else if let Some(bytes)=file.bytes{if bytes.len()>attachments::MAX_FILE{Err("File exceeds 8 MiB".into())}else{self.attachments.add(ctx,&file.name,bytes.to_vec())}}else{Err("File unavailable".into())};if let Err(e)=result{self.attachments.status=e;break;}}
 }
 if self.page!=0 || ctx.input(|i|i.viewport().focused==Some(false)){self.voice.stop();self.voice_reply=false;}
 if self.speech.active() && (self.page!=0 || ctx.input(|i|i.viewport().focused==Some(false))){self.speech.cancel();self.push_to_talk=false;}
 if self.push_to_talk && !ctx.input(|i|i.pointer.primary_down()){self.speech.stop();self.push_to_talk=false;}
 if !self.speech.active(){self.push_to_talk=false;}
 if let Some(result)=self.voice.poll(){match result{Ok(())=>self.speech.status="Speech finished / Озвучування завершено".into(),Err(e)=>self.speech.status=e}}
 if ctx.style().visuals.dark_mode != self.config.dark {ctx.set_visuals(crimson::visuals(self.config.dark));}
 while let Ok(e)=self.rx.try_recv(){match e{Event::DocumentExtract(epoch,result)=>{self.doc_loading=false;if epoch==self.doc_epoch{match result{Ok(data)=>{self.doc_preview=Some(data);self.doc_selected=0;self.doc_checked=vec![0];self.doc_ranked.clear();self.attachments.status="Document preview ready / Текст готовий".into();},Err(e)=>self.attachments.status=e}}},Event::SyncExport(result)=>{self.sync_working=false;match result{Ok(package)=>{self.sync_package=package;self.status="Encrypted package created".into();},Err(e)=>self.status=e}},Event::SyncPreview(result)=>{self.sync_working=false;match result{Ok(p)=>{self.sync_preview=Some(p);self.status="Review before merge".into();},Err(e)=>self.status=e}},Event::Catalog(result)=>{self.scanning=false;match result{Ok(entries)=>{self.catalog_status=format!("{} GGUF models",entries.len());self.catalog=entries;},Err(e)=>self.catalog_status=e}},Event::PullProgress(status,progress)=>{self.pull_status=status;self.pull_progress=progress;},Event::PullDone(result)=>{self.pulling=false;match result{Ok(())=>{self.pull_status="Download complete".into();self.pull_progress=Some(1.0);self.refresh();},Err(e)=>{self.pull_status=format!("Download failed: {e}");self.pull_progress=None;}}},Event::Hardware(result)=>{match result{Ok(h)=>{self.hardware=Some(h);self.hardware_status="Ready".into();},Err(e)=>self.hardware_status=e}},Event::Cancelled=>{self.voice_reply=false;self.busy=false;self.started=None;self.draft.clear();self.status="Generation cancelled".into();},Event::Token(t)=>{self.chunks+=1;if self.config.sound && !self.voice_reply && !t.trim().is_empty(){sound::tick(self.config.volume);} self.draft.push_str(&t)},Event::Done=>{self.busy=false;let s=std::mem::take(&mut self.draft);let speak=std::mem::take(&mut self.voice_reply)&&self.voice_mode&&self.page==0&&!self.speech.active()&&ctx.input(|i|i.viewport().focused!=Some(false));self.add("assistant",s.clone());if speak{if let Err(e)=self.voice.speak(&s,self.speech_language=="en-US",&self.config.neural_voice){self.speech.status=e;}}self.status=if let Some(start)=self.started.take(){format!("Ready · {:.1}s · {} chunks",start.elapsed().as_secs_f64(),self.chunks)}else{"Ready".into()};},Event::Error(e)=>{self.voice_reply=false;self.busy=false;self.started=None;self.draft.clear();self.status=format!("Backend: {e}");},Event::ModelError(e)=>{self.model_status=format!("Backend unavailable: {e}");},Event::Models(m)=>{self.models=m;self.model_status=if self.models.is_empty(){"Backend connected; no models installed".into()}else{"Backend connected".into()};}}}
 if self.proactive_check.map_or(true,|t|t.elapsed()>=Duration::from_secs(1)){self.proactive_check=Some(std::time::Instant::now());if !self.busy&&self.proactive_notice.is_none(){let now=emotion::now();let old=self.config.proactive.last_suggestion;if let Some(text)=self.config.proactive.poll(now){self.proactive_notice=Some(format!("Reminder / Нагадування: {text}"));self.save();}else{if let Some(goal)=self.config.proactive.goal_prompt(now,&self.config.profile.goals,self.config.traits[4]){self.proactive_notice=Some(format!("Goal check-in / Перевірка цілі: {goal}"));}if old!=self.config.proactive.last_suggestion{self.save();}}}}
 let avatar_state=crimson::State::from_activity(self.speech.listening,self.speech.active(),self.busy,!self.draft.is_empty(),self.voice.synthesizing(),self.voice.active());
 let mut mood=self.config.mood.clone();mood.decay(emotion::now());let avatar_mood=if self.config.emotion_enabled{mood.intensity}else{0};
 if self.presence.update(ctx, &mut self.config, avatar_state,avatar_mood) { self.save(); }
 ctx.request_repaint_after(Duration::from_millis(if self.busy||self.pulling||self.scanning||self.sync_working||self.speech.active()||(self.config.avatar_motion&&avatar_state.animated()){33}else{100})); let en=self.config.english; let accent=crimson::RED;
 egui::TopBottomPanel::top("header").show(ctx,|ui|{ui.horizontal(|ui|{ui.heading(RichText::new("H A T O R I E L").strong());ui.label(RichText::new("CRIMSON CORE").color(accent));ui.label(env!("CARGO_PKG_VERSION"));ui.separator();ui.label(&self.status);});if let Some(notice)=self.proactive_notice.clone(){if !self.config.proactive.dnd{ui.horizontal_wrapped(|ui|{ui.label(RichText::new(notice).color(accent));if ui.button(if en{"Dismiss"}else{"Закрити"}).clicked(){self.proactive_notice=None;}});}}});
 egui::SidePanel::left("nav").default_width(205.).show(ctx,|ui|{ui.add_space(12.);let (r,_)=ui.allocate_exact_size(egui::vec2(180.,98.),egui::Sense::hover());crimson::reticle(ui.painter(),r.center(),27.,ctx.input(|i|i.time) as f32,avatar_state,self.config.avatar_motion,avatar_mood);ui.label(avatar_state.label());ui.separator();let labels=if en{["Chat","Memory","Personality","Models","Setup","Reminders","Devices"]}else{["Діалог","Пам'ять","Особистість","Моделі","Налаштування","Нагадування","Пристрої"]};for (i,l) in labels.iter().enumerate(){if ui.add_sized([ui.available_width(),34.],egui::SelectableLabel::new(self.page==i,*l)).clicked(){self.page=i;}}if ui.checkbox(&mut self.config.english,"English").changed(){self.save();} ui.separator(); if ui.checkbox(&mut self.config.dark,"Темна тема / Dark").changed(){self.save();} if ui.checkbox(&mut self.config.sound,"8-bit sound").changed(){self.save();} if ui.add(egui::Slider::new(&mut self.config.volume,0..=100).text("Volume")).changed(){self.save();} if ui.checkbox(&mut self.config.avatar, "Desktop avatar").changed(){self.save();} if ui.checkbox(&mut self.config.avatar_motion,"Avatar motion / Анімація").changed(){self.save();} if ui.checkbox(&mut self.config.minimize_to_tray, "Minimize to tray").changed(){self.save();} if storage::portable(){ui.label("Portable mode — autostart disabled");} if cfg!(windows) && !storage::portable() && ui.checkbox(&mut self.autostart,if en{"Start with Windows"}else{"Запускати з Windows"}).changed(){if let Err(e)=autostart::set(self.autostart){self.status=e;self.autostart=autostart::enabled();}} if let Some(e)=&self.presence.error {ui.label(e);} if self.presence.available() && ui.button("Hide to tray").clicked(){self.presence.hide(ctx);}});
 egui::CentralPanel::default().show(ctx,|ui|{match self.page{0=>{ui.add_enabled_ui(!self.busy&&self.db.is_some(),|ui|{let mut selected=self.config.active_chat;egui::ComboBox::from_id_salt("chats").selected_text(&self.chat_title).show_ui(ui,|ui|{for (id,title) in &self.chats{ui.selectable_value(&mut selected,*id,title);}});if selected!=self.config.active_chat{self.switch_chat(selected);}if ui.button(if en{"New chat"}else{"Новий чат"}).clicked(){if let Some(d)=&self.db{match conversations::create(d){Ok(id)=>{self.chats=conversations::list(d).unwrap_or_default();self.switch_chat(id);},Err(e)=>self.status=e.to_string()}}}ui.horizontal(|ui|{ui.text_edit_singleline(&mut self.chat_title);if ui.button("Rename / Назва").clicked()&&!self.chat_title.trim().is_empty(){if let Some(d)=&self.db{match conversations::rename(d,self.config.active_chat,self.chat_title.trim()){Ok(())=>self.chats=conversations::list(d).unwrap_or_default(),Err(e)=>self.status=e.to_string()}}}});ui.checkbox(&mut self.confirm_delete,"Confirm delete / Підтвердити видалення");if ui.add_enabled(self.confirm_delete&&self.chats.len()>1,egui::Button::new("Delete chat / Видалити чат")).clicked(){if let Some(d)=self.db.as_mut(){match conversations::remove(d,self.config.active_chat){Ok(())=>{self.chats=conversations::list(d).unwrap_or_default();if let Some(id)=self.chats.first().map(|c|c.0){self.switch_chat(id);}},Err(e)=>self.status=e.to_string()}}}});ui.label("Search / Пошук");ui.text_edit_singleline(&mut self.chat_search);egui::ComboBox::from_id_salt("mode").selected_text(&self.config.mode).show_ui(ui,|ui|{for m in ["Companion","Assistant","Developer","Researcher"]{ui.selectable_value(&mut self.config.mode,m.into(),m);}});ui.add_enabled_ui(!self.busy,|ui|{if ui.button(if en{"Save mode"}else{"Зберегти режим"}).clicked(){self.save();}});egui::TopBottomPanel::bottom("compose").show_inside(ui,|ui|{if !self.attachments.empty(){ui.collapsing("Context budget / Бюджет контексту",|ui|{
 if ui.add_enabled(!self.busy,egui::Slider::new(&mut self.config.multimodal_budget,8000..=32000).step_by(1000.).text("Characters / Символи")).changed(){self.save();}
 ui.small("Approximate character limit, not model tokens. Older chat context may be removed; selected excerpts are never silently cut.");
});}
let before=self.attachments.items.iter().map(|a|a.bytes.len()).collect::<Vec<_>>();self.attachments.panel(ui,!self.busy&&!self.speech.active()&&!self.vision_confirm);if before!=self.attachments.items.iter().map(|a|a.bytes.len()).collect::<Vec<_>>(){self.vision_confirm=false;self.invalidate_docs();}
let id=ui.make_persistent_id("compose_input");
#[cfg(windows)]
if !self.busy&&!self.speech.active()&&ui.memory(|m|m.has_focus(id))&&ui.input(|i|i.modifiers.ctrl&&i.key_pressed(egui::Key::V)){
 if self.attachments.paste_image(ui.ctx()).is_ok(){self.vision_confirm=false;self.invalidate_docs();}
}
let enter=!self.busy && !self.speech.active() && self.attachments.empty() && ui.memory(|m|m.has_focus(id)) && ui.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::Enter)); ui.add(egui::TextEdit::multiline(&mut self.input).id(id).desired_rows(3).desired_width(f32::INFINITY));if enter || ui.add_enabled(!self.busy&&!self.speech.active()&&self.attachments.empty(),egui::Button::new(if en{"Send"}else{"Надіслати"})).clicked(){self.send();}
if !self.attachments.empty(){
 if self.attachments.image_only()&&self.config.native_backend{ui.label("Vision requires Ollama / Для аналізу потрібен Ollama");}
 else if self.attachments.image_only()&&!self.busy&&!self.speech.active(){
  ui.label(format!("Vision model: {} · 127.0.0.1:11434",self.config.model));
  ui.label("Images are resized to 1536 px, re-encoded without metadata, and sent to your local Ollama service. Confirm a trusted local model; no image bytes saved in chat history.");
  if ui.button("Аналізувати зображення / Analyze images").clicked()&&!self.input.trim().is_empty(){self.vision_confirm=true;}
  if self.vision_confirm{
   let mut confirmed=false;let mut dismissed=false;
   egui::Window::new("Confirm vision / Підтвердити аналіз").collapsible(false).resizable(false).show(ctx,|ui|{
    ui.label(format!("Передати {} зображень моделі {} через локальний Ollama? Використовуй лише довірений локальний сервер.",self.attachments.items.len(),self.config.model));
    ui.label("Після надсилання файли не залишаються в чернетці. Історія міститиме лише текст.");
    ui.horizontal(|ui|{confirmed=ui.button("Підтвердити й надіслати").clicked();dismissed=ui.button("Скасувати").clicked();});
   });
   if dismissed{self.vision_confirm=false;}if confirmed{self.send_inner(true,None);self.vision_confirm=false;}
  }
 }
}self.document_panel(ui);if cfg!(windows){self.dictation_panel(ui,en);ui.horizontal(|ui|{if ui.add_enabled(!self.busy&&!self.speech.active(),egui::Button::new(if en{"Read last"}else{"Озвучити відповідь"})).clicked(){if let Some(m)=self.messages.iter().rev().find(|m|m.role=="assistant"){if let Err(e)=self.voice.speak(&m.content,self.speech_language=="en-US",&self.config.neural_voice){self.status=e;}}}if ui.button(if en{"Stop speech"}else{"Стоп звук"}).clicked(){self.voice_reply=false;self.voice.stop();}});}if self.busy&&ui.button(if en{"Stop"}else{"Зупинити"}).clicked(){self.voice_reply=false;self.voice.stop();self.cancel.store(true,std::sync::atomic::Ordering::Relaxed);self.status="Stopping…".into();}});egui::ScrollArea::vertical().stick_to_bottom(true).show(ui,|ui|{if self.messages.is_empty()&&!self.busy{ui.vertical_centered(|ui|{ui.add_space(32.);let(r,_)=ui.allocate_exact_size(egui::vec2(160.,150.),egui::Sense::hover());crimson::reticle(ui.painter(),r.center(),42.,ctx.input(|i|i.time) as f32,avatar_state,self.config.avatar_motion,avatar_mood);ui.heading("Hatoriel");ui.label(if en{"Ready to chat"}else{"Готова до діалогу"});});}for m in &self.messages{if !m.content.to_lowercase().contains(&self.chat_search.to_lowercase()){continue;}egui::Frame::group(ui.style()).show(ui,|ui|{ui.set_min_width(ui.available_width().max(0.));ui.label(RichText::new(if m.role=="user"{"YOU"}else{"Hatoriel"}).color(if m.role=="user"{ui.visuals().text_color()}else{accent}).strong());ui.label(&m.content);});ui.add_space(8.);}if !self.draft.is_empty(){ui.label(&self.draft);}});},1=>{ui.heading(if en{"Approved memories"}else{"Затверджені спогади"});if ui.checkbox(&mut self.config.semantic_search,if en{"Semantic search (local Ollama)"}else{"Семантичний пошук (локальний Ollama)"}).changed(){self.save();}if self.config.semantic_search{ui.label("Embedding model (install in Ollama first)");if ui.text_edit_singleline(&mut self.config.embedding_model).changed(){self.save();}}if ui.checkbox(&mut self.config.suggest_memories,if en{"Suggest memories"}else{"Пропонувати спогади"}).changed(){self.save();}let mut decision=None;for (i,m) in self.suggestions.iter().enumerate(){ui.label(format!("{} · {} · {}",m.category,m.importance,m.content));ui.horizontal(|ui|{if ui.button(if en{"Approve"}else{"Підтвердити"}).clicked(){decision=Some((i,true));}if ui.button(if en{"Reject"}else{"Відхилити"}).clicked(){decision=Some((i,false));}});}if let Some((i,approve))=decision{if !approve{self.suggestions.remove(i);}else if let Some(d)=self.db.as_mut(){let m=&self.suggestions[i];match memory_store::add(d,&m.content,m.category,m.importance){Ok(saved)=>{self.memories.push(saved);self.suggestions.remove(i);},Err(e)=>self.status=e.to_string()}}}ui.text_edit_multiline(&mut self.memory);if ui.button(if en{"Add memory"}else{"Додати спогад"}).clicked()&&!self.memory.trim().is_empty(){if let Some(d)=self.db.as_mut(){match memory_store::add(d,self.memory.trim(),"General",50){Ok(m)=>{self.memories.push(m);self.memory.clear();},Err(e)=>self.status=e.to_string()}}else{self.status="Storage unavailable".into();}}ui.label(if en{"Search memories"}else{"Пошук спогадів"});ui.text_edit_singleline(&mut self.memory_search);let mut categories:Vec<String>=memory_core::CATEGORIES.iter().map(|s|s.to_string()).collect();for m in &self.memories{if !categories.contains(&m.category){categories.push(m.category.clone());}}egui::ComboBox::from_id_salt("memory_filter").selected_text(if self.memory_category.is_empty(){"All categories"}else{&self.memory_category}).show_ui(ui,|ui|{ui.selectable_value(&mut self.memory_category,String::new(),"All categories");for c in &categories{ui.selectable_value(&mut self.memory_category,c.clone(),c);}});egui::ComboBox::from_id_salt("memory_sort").selected_text(["Newest","Importance","Category"][self.memory_sort as usize]).show_ui(ui,|ui|{for (i,label) in ["Newest","Importance","Category"].iter().enumerate(){ui.selectable_value(&mut self.memory_sort,i as u8,*label);}});let mut ordered:Vec<_>=self.memories.iter().collect();ordered.sort_by(|a,b|match self.memory_sort{1=>b.importance.cmp(&a.importance).then(b.id.cmp(&a.id)),2=>a.category.cmp(&b.category).then(b.id.cmp(&a.id)),_=>b.id.cmp(&a.id)});let mut remove=None;let mut edit=None;egui::ScrollArea::vertical().show(ui,|ui|{for m in ordered{if (!self.memory_category.is_empty()&&m.category!=self.memory_category)||!m.content.to_lowercase().contains(&self.memory_search.to_lowercase()){continue;}let mut draft=m.clone();ui.push_id(m.id,|ui|{ui.label(format!("{} · {} · {}",m.category,m.importance,if m.created==0{"Legacy date unknown".into()}else{format!("Unix: {}",m.created)}));ui.text_edit_multiline(&mut draft.content);egui::ComboBox::from_id_salt("category").selected_text(&draft.category).show_ui(ui,|ui|{for c in &categories{ui.selectable_value(&mut draft.category,c.clone(),c);}});ui.add(egui::Slider::new(&mut draft.importance,0..=100).text("Importance"));if !draft.content.trim().is_empty()&&(draft.content!=m.content||draft.category!=m.category||draft.importance!=m.importance){edit=Some(draft);}if ui.button(if en{"Delete"}else{"Видалити"}).clicked(){remove=Some(m.id);}ui.separator();});}});if let Some(m)=edit{if let Some(d)=self.db.as_mut(){match memory_store::update(d,&m){Ok(())=>{if let Some(old)=self.memories.iter_mut().find(|x|x.id==m.id){*old=m;}},Err(e)=>self.status=e.to_string()}}}if let Some(id)=remove{if let Some(d)=self.db.as_mut(){match memory_store::remove(d,id){Ok(())=>self.memories.retain(|m|m.id!=id),Err(e)=>self.status=e.to_string()}}}},2=>{ui.heading("Personality / 001");if ui.checkbox(&mut self.config.dynamic_style,if en{"Dynamic communication style"}else{"Динамічний стиль спілкування"}).changed(){self.save();}if ui.checkbox(&mut self.config.emotion_enabled,if en{"Simulated emotions"}else{"Симуляція емоцій"}).changed(){self.save();}if self.config.emotion_enabled{let mut mood=self.config.mood.clone();mood.decay(emotion::now());ui.label(format!("{:?}",mood.mood));ui.add(egui::ProgressBar::new(f32::from(mood.intensity)/100.0).show_percentage());if ui.button(if en{"Reset mood"}else{"Скинути настрій"}).clicked(){self.config.mood=Default::default();self.save();}}ui.collapsing(if en{"User profile"}else{"Профіль користувача"},|ui|{ui.label(if en{"Interests"}else{"Інтереси"});ui.text_edit_multiline(&mut self.config.profile.interests);ui.label(if en{"Communication preferences"}else{"Стиль спілкування"});ui.text_edit_multiline(&mut self.config.profile.communication);ui.label(if en{"Goals"}else{"Цілі"});ui.text_edit_multiline(&mut self.config.profile.goals);if ui.button(if en{"Save profile"}else{"Зберегти профіль"}).clicked(){self.save();}if ui.button(if en{"Clear profile"}else{"Очистити профіль"}).clicked(){self.config.profile=Default::default();self.save();}});for i in 0..7{ui.add(egui::Slider::new(&mut self.config.traits[i],0..=100).text(if en{EN[i]}else{TRAITS[i]}));}if ui.button("Save / Зберегти").clicked(){self.save();}},4=>{ui.heading(if en{"Welcome to Hatoriel"}else{"Вітаю! Я Hatoriel."});ui.label(if en{"Local setup — choose your model and language."}else{"Локальне налаштування — обери модель та мову."});ui.checkbox(&mut self.config.english,"English");ui.separator();ui.label(&self.hardware_status);ui.label(&self.model_status);if let Some(h)=&self.hardware{let recommended=hardware::recommend(h.ram_gib);ui.label(format!("RAM-based recommendation: {recommended}"));if ui.button(if en{"Use recommendation"}else{"Обрати рекомендовану"}).clicked(){self.config.model=recommended.into();}}ui.text_edit_singleline(&mut self.config.model);let installed=self.models.iter().any(|m|m==&self.config.model);if ui.button(if en{"Check connection"}else{"Перевірити підключення"}).clicked(){self.refresh();}if !installed{ui.label(if en{"Ollama must be installed and running. Download requires internet."}else{"Ollama має бути встановлена та запущена. Для завантаження потрібен інтернет."});if ui.add_enabled(!self.config.native_backend&&!self.pulling&&!self.busy,egui::Button::new(if en{"Download model"}else{"Завантажити модель"})).clicked(){self.pull();}}ui.label(&self.pull_status);if let Some(p)=self.pull_progress{ui.add(egui::ProgressBar::new(p).show_percentage());}ui.separator();if ui.add_enabled(installed&&!self.pulling,egui::Button::new(if en{"Finish setup"}else{"Завершити налаштування"})).clicked(){self.config.setup_complete=true;self.save();self.page=0;}if ui.button(if en{"Skip for now"}else{"Пропустити поки що"}).clicked(){self.page=0;}},5=>{ui.heading(if en{"Reminders and initiative"}else{"Нагадування та ініціатива"});if ui.checkbox(&mut self.config.proactive.enabled,if en{"Goal check-ins (opt in)"}else{"Ініціативні перевірки цілей"}).changed(){self.config.proactive.last_suggestion=emotion::now();self.save();}if ui.checkbox(&mut self.config.proactive.dnd,if en{"Do not disturb"}else{"Не турбувати"}).changed(){self.save();}if ui.add(egui::Slider::new(&mut self.config.proactive.interval_minutes,15..=1440).text(if en{"Check-in interval (min)"}else{"Інтервал (хв)"})).changed(){self.save();}ui.separator();ui.text_edit_multiline(&mut self.reminder_text);ui.add(egui::Slider::new(&mut self.reminder_minutes,1..=10080).text(if en{"Remind after (min)"}else{"Нагадати через (хв)"}));if ui.button(if en{"Add reminder"}else{"Додати нагадування"}).clicked(){if self.config.proactive.add(&self.reminder_text,self.reminder_minutes,emotion::now()){self.reminder_text.clear();self.save();}else{self.status="Invalid reminder or limit reached".into();}}let mut changed=false;let mut delete=None;egui::ScrollArea::vertical().show(ui,|ui|{for r in &mut self.config.proactive.reminders{ui.push_id(r.id,|ui|{let seconds=r.due.saturating_sub(emotion::now());ui.label(&r.text);ui.label(if r.completed{"Completed / Виконано".into()}else if seconds==0{"Due / Час настав".into()}else{format!("{} min",seconds.div_ceil(60))});if ui.checkbox(&mut r.completed,if en{"Completed"}else{"Виконано"}).changed(){if !r.completed{r.announced=false;}changed=true;}if ui.button(if en{"Remove reminder"}else{"Видалити нагадування"}).clicked(){delete=Some(r.id);}ui.separator();});}});if let Some(id)=delete{self.config.proactive.reminders.retain(|r|r.id!=id);changed=true;}if changed{self.save();}},6=>{ui.heading(if en{"Desktop connectivity"}else{"Підключення пристроїв"});ui.label("127.0.0.1:17845 · Ollama only");ui.label(if en{"Requires a separately configured trusted HTTPS reverse proxy. Never expose the raw HTTP port."}else{"Потрібен окремо налаштований довірений HTTPS-проксі. Не відкривай HTTP-порт напряму в мережу."});if self.bridge.running(){ui.label(if self.bridge.token.is_empty(){"Stopping / Зупинення"}else{"Running / Активний"});if ui.button("Stop / Зупинити").clicked(){self.bridge.stop();self.reveal_bridge_token=false;}}else if ui.add_enabled(!self.config.native_backend,egui::Button::new("Start bridge / Запустити")).clicked(){match self.bridge.start(self.config.model.clone()){Ok(())=>self.status="Bridge running (loopback only)".into(),Err(e)=>self.status=e}}if !self.bridge.token.is_empty()&&self.bridge.running(){ui.checkbox(&mut self.reveal_bridge_token,"Show session key / Показати ключ");if self.reveal_bridge_token{ui.label(&self.bridge.token);}if ui.button("Copy session key / Копіювати ключ").clicked(){ui.ctx().copy_text(self.bridge.token.clone());}ui.label("New key on each start. Stop to revoke access. / Новий ключ при кожному запуску.");}self.sync_panel(ui);},_=>{ui.heading(if en{"Hardware / recommended model"}else{"Обладнання / рекомендована модель"});ui.label(&self.hardware_status);if let Some(h)=&self.hardware{ui.label(&h.cpu);ui.label(format!("RAM: {:?}",h.ram_gib));for gpu in &h.gpus{ui.label(gpu);}let recommended=hardware::recommend(h.ram_gib);ui.label(format!("RAM-based estimate: {recommended}"));if ui.button("Select recommended model").clicked(){self.config.model=recommended.into();self.save();}}ui.separator();if ui.add_enabled(!self.busy&&!self.pulling,egui::Checkbox::new(&mut self.config.native_backend,"llama.cpp (local server)")).changed(){self.models.clear();self.save();self.refresh();}ui.heading(if self.config.native_backend{"llama.cpp / localhost:8080"}else{"Ollama / localhost:11434"});if self.config.native_backend{ui.label("llama-server executable");ui.add_enabled(!self.busy,egui::TextEdit::singleline(&mut self.config.runtime_path));ui.label("GGUF model path");ui.text_edit_singleline(&mut self.config.gguf_path);let running=self.runtime.running();ui.label("GGUF library directory");ui.text_edit_singleline(&mut self.config.model_dir);if ui.add_enabled(!self.scanning,egui::Button::new("Scan models")).clicked(){self.scanning=true;self.catalog_status="Scanning…".into();self.save();let path=storage::resolve(&self.config.model_dir);let tx=self.tx.clone();std::thread::spawn(move||{let _=tx.send(Event::Catalog(native_library::scan(&path)));});}ui.label(&self.catalog_status);let mut selected=None;egui::ScrollArea::vertical().id_salt("gguf_catalog").max_height(140.).show(ui,|ui|{for entry in &self.catalog{let label=format!("{} — {:.2} GiB",entry.path.display(),entry.bytes as f64/1073741824.0);if ui.add_enabled(!running&&!self.busy,egui::Button::new(label)).clicked(){selected=Some(storage::store(&entry.path));}}});if let Some(path)=selected{self.config.gguf_path=path;self.save();}ui.add_enabled_ui(!running,|ui|{let ram=self.hardware.as_ref().and_then(|h|h.ram_gib);if ui.button("Apply conservative resource profile").clicked(){let p=resources::recommend(ram,std::thread::available_parallelism().map(|n|n.get() as u32).unwrap_or(1));self.config.context_size=p.context;self.config.cpu_threads=p.threads;self.config.gpu_layers=p.gpu_layers;self.save();}ui.add(egui::Slider::new(&mut self.config.context_size,512..=32768).text("Context tokens"));ui.add(egui::Slider::new(&mut self.config.cpu_threads,1..=64).text("CPU threads"));if let Ok(meta)=std::fs::metadata(storage::resolve(&self.config.gguf_path)){if resources::warning(ram,meta.len()){ui.label("Warning: model may exceed available RAM budget; estimate only");}}ui.add(egui::Slider::new(&mut self.config.gpu_layers,0..=999).text("GPU layers"));if ui.button("CPU only").clicked(){self.config.gpu_layers=0;}if ui.button("Maximum GPU offload").clicked(){self.config.gpu_layers=999;}ui.label(format!("Requested GPU layers: {}",self.config.gpu_layers));ui.label("Requires compatible CUDA or Vulkan llama-server build");});ui.label(if running{"Process running — check model readiness"}else{"Managed runtime stopped"});if ui.add_enabled(!running&&!self.busy,egui::Button::new("Start local runtime")).clicked(){match self.runtime.start(&storage::resolve(&self.config.runtime_path),&storage::resolve(&self.config.gguf_path),self.config.gpu_layers,self.config.context_size,self.config.cpu_threads){Ok(())=>{self.config.model="hatori-local".into();self.save();self.model_status="Runtime starting; refresh models when ready".into();},Err(e)=>self.model_status=e}}if ui.add_enabled(running&&!self.busy,egui::Button::new("Stop local runtime")).clicked(){self.runtime.stop();self.models.clear();self.model_status="Runtime stopped".into();}}ui.label(&self.model_status);ui.label(if self.models.iter().any(|m|m==&self.config.model){"Selected model available"}else{"Selected model not confirmed"});if ui.add_enabled(!self.busy,egui::Button::new("Refresh models")).clicked(){self.refresh();}ui.text_edit_singleline(&mut self.config.model);for m in &self.models{ui.selectable_value(&mut self.config.model,m.clone(),m);}if ui.button("Save").clicked(){self.save();}if ui.add_enabled(!self.config.native_backend&&!self.pulling&&!self.busy,egui::Button::new(if en{"Download selected model"}else{"Завантажити обрану модель"})).clicked(){self.pull();}ui.label(&self.pull_status);if let Some(p)=self.pull_progress{ui.add(egui::ProgressBar::new(p).show_percentage());ui.label(if en{"Current layer progress"}else{"Прогрес поточного шару"});}}}});}}
fn main()->eframe::Result<()> {eframe::run_native("Hatoriel",eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([1050.,720.]),..Default::default()},Box::new(|cc|Ok(Box::new(App::new(cc)))))}
