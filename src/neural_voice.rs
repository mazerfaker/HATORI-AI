use serde::{Serialize,Deserialize};
use std::path::{Path,PathBuf};

#[derive(Clone,Serialize,Deserialize)]
#[serde(default)]
pub struct Settings {
 pub enabled:bool,pub python:String,pub model:String,pub speed:f32,pub volume:f32,
}
impl Default for Settings{fn default()->Self{Self{enabled:false,python:String::new(),model:String::new(),speed:1.0,volume:0.85}}}
impl Settings{
 pub fn length_scale(&self)->f32{1.0/if self.speed.is_finite(){self.speed.clamp(0.65,1.5)}else{1.0}}
 pub fn gain(&self)->f32{if self.volume.is_finite(){self.volume.clamp(0.0,1.0)}else{0.85}}
}
fn config_path(model:&Path)->PathBuf{let mut p=model.as_os_str().to_os_string();p.push(".json");PathBuf::from(p)}
fn validate_wav(path:&Path)->Result<(),String>{
 use std::io::Read;
 let mut file=std::fs::File::open(path).map_err(|e|e.to_string())?;
 let len=file.metadata().map_err(|e|e.to_string())?.len();
 if !(44..=64*1024*1024).contains(&len){return Err("Neural WAV missing or exceeds 64 MiB".into());}
 let mut h=[0u8;12];file.read_exact(&mut h).map_err(|e|e.to_string())?;
 if &h[..4]!=b"RIFF"||&h[8..]!=b"WAVE"{return Err("Invalid neural WAV".into());}Ok(())
}
#[cfg(windows)]
pub struct Job{child:std::process::Child,dir:PathBuf,wav:PathBuf,playing:bool,started:std::time::Instant}
#[derive(Default)]
pub struct Neural{
 #[cfg(windows)] job:Option<Job>,
}
impl Neural{
 pub fn active(&self)->bool{
  #[cfg(windows)] {self.job.is_some()}
  #[cfg(not(windows))] {false}
 }
 pub fn playing(&self)->bool{
  #[cfg(windows)] {self.job.as_ref().is_some_and(|j|j.playing)}
  #[cfg(not(windows))] {false}
 }
 pub fn stop(&mut self){
  #[cfg(windows)] if let Some(mut j)=self.job.take(){let _=j.child.kill();let _=j.child.wait();let _=std::fs::remove_dir_all(j.dir);}
 }
 pub fn start(&mut self,text:&str,settings:&Settings)->Result<(),String>{
  self.stop();
  #[cfg(windows)] {
   use std::{io::Write,os::windows::process::CommandExt,process::{Command,Stdio}};
   let text:String=text.chars().take(4000).collect();if text.trim().is_empty(){return Err("No speech text".into());}
   // Only explicitly configured local paths, never selected by the AI.
   let python=std::fs::canonicalize(crate::storage::resolve(&settings.python)).map_err(|_|"Choose a trusted local Python executable with piper-tts installed")?;
   let model=std::fs::canonicalize(crate::storage::resolve(&settings.model)).map_err(|_|"Choose an installed ONNX voice model")?;
   if !python.is_file()||!python.extension().is_some_and(|e|e.eq_ignore_ascii_case("exe")){return Err("Python executable must be an .exe file".into());}
   if !model.is_file()||!model.extension().is_some_and(|e|e.eq_ignore_ascii_case("onnx")){return Err("Voice must be an ONNX file".into());}
   let config=config_path(&model);if !config.is_file(){return Err("Missing voice .onnx.json configuration".into());}
   if std::fs::metadata(&config).map_err(|e|e.to_string())?.len()>1024*1024{return Err("Voice config too large".into());}
   let mut id=[0u8;16];getrandom::getrandom(&mut id).map_err(|e|e.to_string())?;
   let name=id.iter().map(|b|format!("{b:02x}")).collect::<String>();
   let dir=std::env::temp_dir().join(format!("hatoriel-tts-{name}"));std::fs::create_dir(&dir).map_err(|e|e.to_string())?;
   let wav=dir.join("speech.wav");
   let spawn=Command::new(python).args(["-I","-X","utf8","-m","piper","--model"]).arg(&model).arg("--config").arg(&config).arg("--output-file").arg(&wav)
    .arg("--length-scale").arg(format!("{:.3}",settings.length_scale())).arg("--volume").arg(format!("{:.3}",settings.gain()))
    .current_dir(&dir).creation_flags(0x08000000).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
   let mut child=match spawn{Ok(c)=>c,Err(e)=>{let _=std::fs::remove_dir_all(&dir);return Err(e.to_string());}};
   if let Some(mut stdin)=child.stdin.take(){if let Err(e)=std::thread::Builder::new().name("hatoriel-neural-input".into()).spawn(move||{let _=stdin.write_all(text.as_bytes());}){let _=child.kill();let _=child.wait();let _=std::fs::remove_dir_all(&dir);return Err(e.to_string());}}
   self.job=Some(Job{child,dir,wav,playing:false,started:std::time::Instant::now()});Ok(())
  }
  #[cfg(not(windows))] {let _=(text,settings);Err("Neural TTS preview is Windows only".into())}
 }
 pub fn poll(&mut self)->Option<Result<(),String>>{
  #[cfg(windows)] {
   use std::{os::windows::process::CommandExt,process::{Command,Stdio},time::{Duration,Instant}};
   let j=self.job.as_mut()?;
   if j.started.elapsed()>Duration::from_secs(if j.playing{360}else{120})||(!j.playing&&std::fs::metadata(&j.wav).is_ok_and(|m|m.len()>64*1024*1024)){self.stop();return Some(Err("Neural speech exceeded time or size limit".into()));}
   match j.child.try_wait(){
    Ok(None)=>return None,
    Err(e)=>{let e=e.to_string();self.stop();return Some(Err(e));},
    Ok(Some(exit))=>{
     if !exit.success(){self.stop();return Some(Err("Neural speech failed; check Piper, Python and voice compatibility".into()));}
     if j.playing{self.stop();return Some(Ok(()));}
    }
   }
   if let Err(e)=validate_wav(&j.wav){self.stop();return Some(Err(e));}
   let Some(root)=std::env::var_os("SystemRoot")else{self.stop();return Some(Err("Windows directory unavailable".into()));};
   let exe=PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
   let script=r#"$ErrorActionPreference='Stop'; $p=New-Object System.Media.SoundPlayer; try {$p.SoundLocation=$env:HATORIEL_WAV; $p.Load(); $p.PlaySync()} finally {$p.Dispose()}"#;
   match Command::new(exe).args(["-NoProfile","-NonInteractive","-Command",script]).env("HATORIEL_WAV",&j.wav).creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn(){
    Ok(c)=>{j.child=c;j.playing=true;j.started=Instant::now();None},
    Err(e)=>{let e=e.to_string();self.stop();Some(Err(e))}
   }
  }
  #[cfg(not(windows))] {None}
 }
}
impl Drop for Neural{fn drop(&mut self){self.stop();}}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn speed_and_volume_bounds(){let mut s=Settings::default();s.speed=100.;s.volume=-1.;assert!((s.length_scale()-2./3.).abs()<0.001);assert_eq!(s.gain(),0.);s.speed=f32::NAN;assert_eq!(s.length_scale(),1.);}
 #[test]fn config_suffix(){assert_eq!(config_path(Path::new("voice.onnx")),PathBuf::from("voice.onnx.json"));}
 #[test]fn wav_header_validation(){let mut id=[0u8;8];getrandom::getrandom(&mut id).unwrap();let p=std::env::temp_dir().join(format!("hatoriel-wav-test-{}",u64::from_le_bytes(id)));let mut wav=[0u8;44];wav[..4].copy_from_slice(b"RIFF");wav[8..12].copy_from_slice(b"WAVE");std::fs::write(&p,wav).unwrap();assert!(validate_wav(&p).is_ok());std::fs::write(&p,b"bad").unwrap();assert!(validate_wav(&p).is_err());std::fs::remove_file(p).unwrap();}
}
