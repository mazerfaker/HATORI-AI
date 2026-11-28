//! Session-only, user-selected Windows executable catalog. Not a process sandbox.
use std::{fs,io::Read,path::{Path,PathBuf},process::Child};
use sha2::{Digest,Sha256};
const MAX_EXE:u64=128*1024*1024;
pub const MAX_APPS:usize=16;
const MAX_RUNNING:usize=8;
#[derive(Debug,Clone,PartialEq,Eq)]
struct Fingerprint{bytes:u64,hash:[u8;32]}
#[derive(Debug)]
pub struct Entry{pub path:PathBuf,pub name:String,fingerprint:Fingerprint}
struct Running{path:PathBuf,child:Child}
#[derive(Default)]
pub struct Catalog{entries:Vec<Entry>,running:Vec<Running>}
fn allowed_name(path:&Path)->bool{
 if !path.extension().is_some_and(|e|e.eq_ignore_ascii_case("exe")){return false;}
 let name=path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
 !matches!(name.as_str(),"cmd"|"powershell"|"pwsh"|"wscript"|"cscript"|"mshta"|"rundll32"|"regsvr32"|"msiexec"|"schtasks"|"reg"|"runas"|"wmic"|"certutil"|"bitsadmin"|"diskpart"|"shutdown"|"bash"|"sh"|"wsl"|"python"|"pythonw"|"py"|"node"|"conhost")
}
fn canonical_exe(path:&Path)->Result<PathBuf,String>{
 if !allowed_name(path){return Err("Choose a trusted ordinary EXE; command interpreters and system tools are not supported".into());}
 crate::file_access::inspect_chain(path)?;
 let canonical=fs::canonicalize(path).map_err(|e|e.to_string())?;
 crate::file_access::inspect_chain(&canonical)?;
 if !allowed_name(&canonical){return Err("Unsupported executable".into());}
 Ok(canonical)
}
fn fingerprint(path:&Path)->Result<Fingerprint,String>{
 let canonical=canonical_exe(path)?;
 if canonical!=path{return Err("Executable path changed".into());}
 let before=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
 if !crate::file_access::ordinary(&before)||!before.is_file()||before.len()<2||before.len()>MAX_EXE{return Err("Regular EXE up to 128 MiB required".into());}
 let file=fs::File::open(path).map_err(|e|e.to_string())?;
 let opened=file.metadata().map_err(|e|e.to_string())?;
 if !crate::file_access::ordinary(&opened)||!opened.is_file()||opened.len()!=before.len(){return Err("Executable changed".into());}
 let mut reader=file.take(MAX_EXE+1);
 let mut magic=[0u8;2];reader.read_exact(&mut magic).map_err(|e|e.to_string())?;
 if &magic!=b"MZ"{return Err("Not a Windows executable header".into());}
 let mut hash=Sha256::new();hash.update(magic);
 let mut count=2u64;let mut buf=[0u8;65536];
 loop{let n=reader.read(&mut buf).map_err(|e|e.to_string())?;if n==0{break;}count+=n as u64;if count>MAX_EXE{return Err("Executable exceeds limit".into());}hash.update(&buf[..n]);}
 let after=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
 if count!=before.len()||!crate::file_access::ordinary(&after)||after.len()!=count||before.modified().ok()!=after.modified().ok()||canonical_exe(path)?!=path{return Err("Executable changed during review".into());}
 Ok(Fingerprint{bytes:count,hash:hash.finalize().into()})
}
impl Catalog{
 pub fn entries(&self)->&[Entry]{&self.entries}
 pub fn clear(&mut self){self.entries.clear();}
 pub fn revoke(&mut self,path:&Path){self.entries.retain(|a|a.path!=path);}
 pub fn register(&mut self,path:&Path)->Result<(),String>{
  let path=canonical_exe(path)?;
  let fp=fingerprint(&path)?;
  if let Some(entry)=self.entries.iter_mut().find(|a|a.path==path){entry.fingerprint=fp;return Ok(());}
  if self.entries.len()>=MAX_APPS{return Err("Maximum 16 apps".into());}
  let name=path.file_name().unwrap_or_default().to_string_lossy().into_owned();
  self.entries.push(Entry{path,name,fingerprint:fp});Ok(())
 }
 fn verify(&self,path:&Path)->Result<(),String>{
  let entry=self.entries.iter().find(|a|a.path==path).ok_or("App is not approved")?;
  if fingerprint(path)?!=entry.fingerprint{return Err("Executable changed. Remove and approve it again.".into());}
  Ok(())
 }
 pub fn poll(&mut self){
  self.running.retain_mut(|p|matches!(p.child.try_wait(),Ok(None)|Err(_)));
 }
 pub fn running_count(&self)->usize{self.running.len()}
 pub fn is_running(&self,path:&Path)->bool{self.running.iter().any(|r|r.path==path)}
 #[cfg(windows)]
 pub fn launch(&mut self,path:&Path)->Result<u32,String>{
  self.poll();
  if self.running.len()>=MAX_RUNNING{return Err("Tracked process limit reached".into());}
  if self.is_running(path){return Err("This launch is still running".into());}
  self.verify(path)?;
  // No command shell, user-supplied arguments, PATH lookup or elevation request.
  // Child inherits user rights and can act independently.
  let cwd=path.parent().ok_or("Missing application directory")?;
  let child=std::process::Command::new(path).current_dir(cwd).spawn().map_err(|e|e.to_string())?;
  let id=child.id();self.running.push(Running{path:path.to_path_buf(),child});Ok(id)
 }
 #[cfg(not(windows))]
 pub fn launch(&mut self,_:&Path)->Result<u32,String>{Err("Application launch is Windows only".into())}
}
#[cfg(test)]mod tests{
 use super::*;
 struct Temp(PathBuf);
 impl Temp{fn new()->Self{let mut id=[0u8;16];getrandom::getrandom(&mut id).unwrap();let name=id.iter().map(|b|format!("{b:02x}")).collect::<String>();let p=std::env::temp_dir().join(format!("hatoriel-app-{name}"));fs::create_dir(&p).unwrap();Self(fs::canonicalize(p).unwrap())}}
 impl Drop for Temp{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
 #[test]fn denies_interpreters_and_scripts(){for name in ["cmd.exe","PowerShell.EXE","wscript.exe","foo.bat","foo.lnk","node.exe"]{assert!(!allowed_name(Path::new(name)));}assert!(allowed_name(Path::new("editor.exe")));}
 #[test]fn registration_fingerprint_and_revoke(){let t=Temp::new();let p=t.0.join("editor.exe");fs::write(&p,b"MZfirst").unwrap();let mut c=Catalog::default();assert!(c.verify(&p).is_err());c.register(&p).unwrap();assert!(c.verify(&p).is_ok());fs::write(&p,b"MZother").unwrap();assert!(c.verify(&p).is_err());c.revoke(&p);assert!(c.verify(&p).is_err());}
 #[test]fn invalid_header(){let t=Temp::new();let p=t.0.join("fake.exe");fs::write(&p,b"not exe").unwrap();assert!(Catalog::default().register(&p).is_err());}
}
