//! Read-only, explicitly granted folder scopes. Not a security sandbox.
use std::{fs,io::Read,path::{Component,Path,PathBuf}};
pub const MAX_ROOTS:usize=8;
const MAX_ENTRIES:usize=200;
const MAX_TEXT:usize=64*1024;
const MAX_DEPTH:usize=24;
#[derive(Clone,Debug)]
pub struct Entry{pub path:PathBuf,pub name:String,pub dir:bool,pub bytes:u64}
#[derive(Default)]
pub struct Scopes{roots:Vec<PathBuf>}
fn ordinary(meta:&fs::Metadata)->bool{
 if meta.file_type().is_symlink(){return false;}
 #[cfg(windows)]{
  use std::os::windows::fs::MetadataExt;
  if meta.file_attributes()&0x400!=0{return false;} // FILE_ATTRIBUTE_REPARSE_POINT
 }
 meta.is_file()||meta.is_dir()
}
fn drive_path(path:&Path)->bool{
 #[cfg(windows)]{
  use std::path::Prefix;
  matches!(path.components().next(),Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_)))
 }
 #[cfg(not(windows))]{path.is_absolute()}
}
fn inspect_chain(path:&Path)->Result<(),String>{
 if !drive_path(path){return Err("Only local absolute paths are supported".into());}
 let mut current=PathBuf::new();let mut depth=0;
 for part in path.components(){
  match part{
   Component::Prefix(p)=>current.push(p.as_os_str()),
   Component::RootDir=>current.push(part.as_os_str()),
   Component::Normal(s)=>{
    depth+=1;if depth>MAX_DEPTH{return Err("Path depth exceeds limit".into());}
    // No alternate data stream, drive-relative, or device-like component.
    #[cfg(windows)] if s.to_string_lossy().contains(':'){return Err("Alternate data streams are not allowed".into());}
    current.push(s);
    let m=fs::symlink_metadata(&current).map_err(|e|e.to_string())?;
    if !ordinary(&m){return Err("Symbolic links, reparse points and special files are not allowed".into());}
   },
   _=>return Err("Relative traversal is not allowed".into()),
  }
 }
 Ok(())
}
impl Scopes{
 pub fn roots(&self)->&[PathBuf]{&self.roots}
 pub fn clear(&mut self){self.roots.clear();}
 pub fn remove(&mut self,path:&Path){self.roots.retain(|r|r!=path);}
 pub fn grant(&mut self,path:&Path)->Result<(),String>{
  if self.roots.len()>=MAX_ROOTS{return Err("Maximum eight folder grants".into());}
  inspect_chain(path)?;
  let canon=fs::canonicalize(path).map_err(|e|e.to_string())?;
  inspect_chain(&canon)?;
  if !canon.is_dir()||canon.parent().is_none(){return Err("Select a specific folder, not a drive root".into());}
  if !self.roots.contains(&canon){self.roots.push(canon);}
  Ok(())
 }
 pub fn checked(&self,path:&Path)->Result<PathBuf,String>{
  if !self.roots.iter().any(|r|path.starts_with(r)){return Err("Path is outside granted folders".into());}
  inspect_chain(path)?;
  let canon=fs::canonicalize(path).map_err(|e|e.to_string())?;
  if canon!=path||!self.roots.iter().any(|r|canon.starts_with(r)){return Err("Path changed or escaped its grant".into());}
  inspect_chain(&canon)?;Ok(canon)
 }
 pub fn list(&self,path:&Path)->Result<(Vec<Entry>,bool),String>{
  let path=self.checked(path)?;
  if !path.is_dir(){return Err("Not a folder".into());}
  let mut entries=Vec::new();let mut capped=false;
  for (i,row) in fs::read_dir(&path).map_err(|e|e.to_string())?.enumerate(){
   if i>=MAX_ENTRIES{capped=true;break;}
   let e=row.map_err(|e|e.to_string())?;
   let name=e.file_name().to_string_lossy().into_owned();
   let child=e.path();let Ok(meta)=fs::symlink_metadata(&child)else{continue;};
   if !ordinary(&meta){continue;}
   if self.checked(&child).is_err(){continue;}
   entries.push(Entry{path:child,name,dir:meta.is_dir(),bytes:meta.len()});
  }
  entries.sort_by(|a,b|b.dir.cmp(&a.dir).then_with(||a.name.to_lowercase().cmp(&b.name.to_lowercase())));
  Ok((entries,capped))
 }
 pub fn read_text(&self,path:&Path)->Result<String,String>{
  let path=self.checked(path)?;
  let name=path.file_name().and_then(|n|n.to_str()).ok_or("Unsupported filename")?;
  if !crate::documents::supported(name)||name.to_ascii_lowercase().ends_with(".pdf")||name.to_ascii_lowercase().ends_with(".docx"){
   return Err("Only supported plain-text file formats can be previewed here".into());
  }
  let meta=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
  if !ordinary(&meta)||!meta.is_file()||meta.len()>MAX_TEXT as u64{return Err("Preview requires a regular text file up to 64 KiB".into());}
  let file=fs::File::open(&path).map_err(|e|e.to_string())?;
  let opened=file.metadata().map_err(|e|e.to_string())?;
  if !ordinary(&opened)||!opened.is_file()||opened.len()>MAX_TEXT as u64{return Err("File changed or exceeds preview limit".into());}
  self.checked(&path)?;
  let mut bytes=Vec::new();file.take(MAX_TEXT as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
  if bytes.len()>MAX_TEXT{return Err("File grew beyond preview limit".into());}
  self.checked(&path)?;
  let text=crate::documents::plain(&bytes)?;
  Ok(text.chars().filter(|c|!c.is_control()||matches!(c,'\t'|'\r'|'\n')).collect())
 }
}
#[cfg(test)]mod tests{
 use super::*;
 struct Temp(PathBuf);
 impl Temp{fn new()->Self{let mut id=[0u8;16];getrandom::getrandom(&mut id).unwrap();let name=id.iter().map(|b|format!("{b:02x}")).collect::<String>();let p=std::env::temp_dir().join(format!("hatoriel-scope-{name}"));fs::create_dir(&p).unwrap();Self(fs::canonicalize(p).unwrap())}}
 impl Drop for Temp{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
 #[test]fn grant_and_revoke(){let t=Temp::new();let mut s=Scopes::default();let p=t.0.join("note.txt");fs::write(&p,"Привіт").unwrap();assert!(s.read_text(&p).is_err());s.grant(&t.0).unwrap();assert_eq!(s.read_text(&p).unwrap(),"Привіт");assert_eq!(s.list(&t.0).unwrap().0.len(),1);s.remove(&t.0);assert!(s.read_text(&p).is_err());}
 #[test]fn traversal_and_outside(){let t=Temp::new();let other=Temp::new();let mut s=Scopes::default();s.grant(&t.0).unwrap();assert!(s.checked(&t.0.join("..")).is_err());assert!(s.checked(&other.0).is_err());}
 #[test]fn rejects_binary_and_large(){let t=Temp::new();let mut s=Scopes::default();s.grant(&t.0).unwrap();let p=t.0.join("large.txt");fs::write(&p,vec![b'a';MAX_TEXT+1]).unwrap();assert!(s.read_text(&p).is_err());let p=t.0.join("binary.txt");fs::write(&p,[0,1,2]).unwrap();assert!(s.read_text(&p).is_err());}
 #[cfg(unix)]#[test]fn symlink_escape(){let t=Temp::new();let other=Temp::new();let mut s=Scopes::default();s.grant(&t.0).unwrap();std::os::unix::fs::symlink(&other.0,t.0.join("link")).unwrap();assert!(s.checked(&t.0.join("link")).is_err());assert!(s.list(&t.0).unwrap().0.is_empty());}
}
