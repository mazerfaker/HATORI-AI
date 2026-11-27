//! Local, opt-in Windows desktop dictation. Audio never passes through the AI backend.
use base64::{Engine,engine::general_purpose::STANDARD};
#[cfg(windows)]
use std::{io::{BufRead,BufReader,Read},process::{Child,ChildStdin,Command,Stdio},sync::mpsc,time::{Duration,Instant},os::windows::process::CommandExt};

const MAX_TEXT:usize=12000;
#[derive(Debug,PartialEq)]
enum Packet{Ready,Text(String),Error(String),Done}
fn parse(line:&str)->Option<Packet>{
 let (tag,encoded)=line.trim_end_matches(['\r','\n']).split_once(' ')?;
 if encoded.len()>65536{return None;}
 let value=String::from_utf8(STANDARD.decode(encoded).ok()?).ok()?;
 match tag{"READY"=>Some(Packet::Ready),"TEXT"=>Some(Packet::Text(value)),"ERROR"=>Some(Packet::Error(value.chars().take(500).collect())),"DONE"=>Some(Packet::Done),_=>None}
}
pub fn append_draft(draft:&str,text:&str)->String{
 let mut value=draft.to_owned();
 if !value.is_empty()&&!value.ends_with(char::is_whitespace)&&!text.is_empty(){value.push(' ');}
 value.push_str(text);
 value.chars().take(MAX_TEXT).collect()
}
#[cfg(windows)]
struct Session{
 child:Child,stdin:Option<ChildStdin>,rx:mpsc::Receiver<Packet>,started:Instant,stopping:Option<Instant>,failed:bool,done:bool,
}
#[derive(Default)]
pub struct Capture{
 #[cfg(windows)] session:Option<Session>,
 pub text:String,pub status:String,pub listening:bool,
}
impl Capture{
 pub fn active(&self)->bool{
  #[cfg(windows)] {self.session.is_some()}
  #[cfg(not(windows))] {false}
 }
 pub fn start(&mut self,language:&str)->Result<(),String>{
  if self.active(){return Err("Dictation already active".into());}
  if !matches!(language,"uk-UA"|"en-US"){return Err("Unsupported speech language".into());}
  #[cfg(windows)] {
   let root=std::env::var_os("SystemRoot").ok_or("Windows directory unavailable")?;
   let exe=std::path::PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
   let mut child=Command::new(exe).args(["-NoProfile","-NonInteractive","-Command",include_str!("speech_windows.ps1")])
    .env("HATORIEL_STT_LANG",language).creation_flags(0x08000000).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e|e.to_string())?;
   let stdin=child.stdin.take();
   let Some(stdout)=child.stdout.take()else{let _=child.kill();let _=child.wait();return Err("Dictation pipe unavailable".into());};
   let(tx,rx)=mpsc::channel();
   if let Err(e)=std::thread::Builder::new().name("hatoriel-dictation".into()).spawn(move||{
    // Bound the total protocol stream, even if the recognizer produces excessive output.
    for line in BufReader::new(stdout.take(256*1024)).lines(){
     let Ok(line)=line else{break;};
     if let Some(packet)=parse(&line){if tx.send(packet).is_err(){break;}}
    }
   }){let _=child.kill();let _=child.wait();return Err(e.to_string());}
   self.text.clear();self.status="Starting microphone / Запуск мікрофона…".into();self.listening=false;
   self.session=Some(Session{child,stdin,rx,started:Instant::now(),stopping:None,failed:false,done:false});Ok(())
  }
  #[cfg(not(windows))] {Err("Windows dictation only".into())}
 }
 pub fn stop(&mut self){
  #[cfg(windows)] if let Some(s)=self.session.as_mut(){
   if s.stopping.is_none(){s.stdin.take();s.stopping=Some(Instant::now());self.status="Finishing dictation / Завершення…".into();self.listening=false;}
  }
 }
 pub fn cancel(&mut self){
  #[cfg(windows)] if let Some(mut s)=self.session.take(){let _=s.child.kill();let _=s.child.wait();self.status="Dictation cancelled / Запис скасовано".into();}
  self.listening=false;
 }
 pub fn poll(&mut self){
  #[cfg(windows)] {
   let Some(s)=self.session.as_mut()else{return;};
   let mut closed=false;
   loop{match s.rx.try_recv(){
    Ok(Packet::Ready)=>{if s.stopping.is_none(){self.listening=true;self.status="Listening / Слухаю · 45 s max".into();}},
    Ok(Packet::Text(t))=>self.text=append_draft(&self.text,&t),
    Ok(Packet::Error(e))=>{s.failed=true;self.status=format!("Dictation: {e}");},
    Ok(Packet::Done)=>s.done=true,
    Err(mpsc::TryRecvError::Empty)=>break,
    Err(mpsc::TryRecvError::Disconnected)=>{closed=true;break;}
   }}
   if s.started.elapsed()>Duration::from_secs(65)||s.stopping.is_some_and(|t|t.elapsed()>Duration::from_secs(12)){
    self.cancel();self.status="Dictation time limit / Ліміт часу".into();return;
   }
   match s.child.try_wait(){
    Ok(Some(exit)) if closed=>{
     let failed=s.failed||!exit.success()||!s.done;
     if !s.failed{self.status=if failed{"Dictation unavailable; check Windows speech and microphone / Перевір мовний пакет і мікрофон".into()}else if self.text.trim().is_empty(){"No speech recognized / Не розпізнано мовлення".into()}else{"Draft ready / Чернетка готова".into()};}
     self.session=None;self.listening=false;
    },
    Err(e)=>{let error=e.to_string();self.cancel();self.status=error;},
    _=>{}
   }
  }
 }
}
impl Drop for Capture{fn drop(&mut self){self.cancel();}}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn protocol_unicode(){let b=STANDARD.encode("Привіт 🌹");assert_eq!(parse(&format!("TEXT {b}\r\n")),Some(Packet::Text("Привіт 🌹".into())));assert_eq!(parse("READY "),Some(Packet::Ready));assert!(parse("TEXT invalid!").is_none());assert!(parse("OTHER ").is_none());}
 #[test]fn draft_bounds(){assert_eq!(append_draft("текст","слово"),"текст слово");assert_eq!(append_draft("текст ","слово"),"текст слово");assert_eq!(append_draft("",&"🌹".repeat(12001)).chars().count(),12000);}
}
