#[derive(Default)]
pub struct Voice {
 neural:crate::neural_voice::Neural,
 #[cfg(windows)] child:Option<std::process::Child>,
}
impl Voice {
 pub fn active(&self)->bool{
  if self.neural.active(){return true;}
  #[cfg(windows)] {self.child.is_some()}
  #[cfg(not(windows))] {false}
 }
 pub fn synthesizing(&self)->bool{self.neural.active()&&!self.neural.playing()}
 pub fn stop(&mut self){
  self.neural.stop();
  #[cfg(windows)] if let Some(mut child)=self.child.take(){let _=child.kill();let _=child.wait();}
 }
 pub fn speak(&mut self,text:&str,english:bool,settings:&crate::neural_voice::Settings)->Result<(),String>{
  self.stop();
  if settings.enabled{return self.neural.start(text,settings);}
  #[cfg(windows)] {
   use std::{io::Write,process::{Command,Stdio},os::windows::process::CommandExt};
   let text:String=text.chars().take(4000).collect();
   if text.trim().is_empty(){return Err("No text to speak".into());}
   let root=std::env::var_os("SystemRoot").ok_or("Windows directory unavailable")?;
   let exe=std::path::PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
   let script=r#"$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Speech; $s=New-Object System.Speech.Synthesis.SpeechSynthesizer; try { $culture=if($env:HATORIEL_SPEECH_LANG -eq 'en-US'){'en-US'}else{'uk-UA'}; $s.SelectVoiceByHints([System.Speech.Synthesis.VoiceGender]::NotSet,[System.Speech.Synthesis.VoiceAge]::NotSet,0,[System.Globalization.CultureInfo]::GetCultureInfo($culture)); $s.Speak([Console]::In.ReadToEnd()) } finally { $s.Dispose() }"#;
   // The message is supplied as UTF-8 stdin, never interpolated into the command.
   let script=format!("[Console]::InputEncoding=[System.Text.Encoding]::UTF8; {script}");
   let mut child=Command::new(exe).args(["-NoProfile","-NonInteractive","-Command",&script]).env("HATORIEL_SPEECH_LANG",if english{"en-US"}else{"uk-UA"}).creation_flags(0x08000000).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e|e.to_string())?;
   if let Some(mut stdin)=child.stdin.take(){std::thread::spawn(move||{let _=stdin.write_all(text.as_bytes());});}
   self.child=Some(child);Ok(())
  }
  #[cfg(not(windows))] {let _=(text,english);Err("Windows speech only".into())}
 }
 pub fn poll(&mut self)->Option<Result<(),String>>{
  if self.neural.active(){return self.neural.poll();}
  #[cfg(windows)] {
   if let Some(child)=self.child.as_mut(){match child.try_wait(){Ok(Some(s))=>{self.child=None;return Some(if s.success(){Ok(())}else{Err("Speech unavailable; check installed Windows voices".into())});},Err(e)=>{self.stop();return Some(Err(e.to_string()));},Ok(None)=>{}}}
  }
  None
 }
}
impl Drop for Voice{fn drop(&mut self){self.stop();}}
