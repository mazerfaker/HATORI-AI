use std::{path::Path,process::{Child,Command,Stdio},io::Read};
pub fn validate_model(path:&Path)->Result<(),String>{
 let mut f=std::fs::File::open(path).map_err(|e|e.to_string())?;
 let mut header=[0u8;4];f.read_exact(&mut header).map_err(|e|e.to_string())?;
 if &header!=b"GGUF"{return Err("Not a GGUF model".into());}Ok(())
}
#[derive(Default)]
pub struct Runtime{child:Option<Child>}
impl Runtime{
 pub fn running(&mut self)->bool{
 if let Some(c)=self.child.as_mut(){match c.try_wait(){Ok(None)=>return true,Ok(Some(_))=>{},Err(_)=>return true}}self.child=None;false
 }
 pub fn start(&mut self,exe:&Path,model:&Path,gpu_layers:u32,context:u32,threads:u32)->Result<(),String>{
 if self.running(){return Err("Runtime already running".into());}
 validate_model(model)?;
 let exe=exe.canonicalize().map_err(|e|e.to_string())?;
 let model=model.canonicalize().map_err(|e|e.to_string())?;
 let mut cmd=Command::new(exe);cmd.arg("-m").arg(model).args(["--host","127.0.0.1","--port","8080","--alias","hatori-local","-ngl"]).arg(gpu_layers.min(999).to_string()).arg("-c").arg(context.clamp(512,32768).to_string()).arg("-t").arg(threads.clamp(1,64).to_string()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
 #[cfg(windows)]{use std::os::windows::process::CommandExt;cmd.creation_flags(0x08000000);}
 self.child=Some(cmd.spawn().map_err(|e|e.to_string())?);Ok(())
 }
 pub fn stop(&mut self){if let Some(mut c)=self.child.take(){let _=c.kill();let _=c.wait();}}
}
impl Drop for Runtime{fn drop(&mut self){self.stop();}}
