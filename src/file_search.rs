//! Bounded, cooperative search over a snapshot of explicitly granted roots.
use crate::file_access::{Scopes,ordinary};
use std::{fs,path::{Path,PathBuf},sync::atomic::{AtomicBool,Ordering},time::{Duration,Instant,SystemTime}};
const SCAN_LIMIT:usize=4000;
const RESULT_LIMIT:usize=200;
const TEXT_LIMIT:usize=64;
const TIME_LIMIT:Duration=Duration::from_secs(5);
#[derive(Clone,Default,Debug)]
pub struct Filter{
 pub name:String,pub extension:String,pub content:String,pub min_kib:u32,pub max_kib:u32,pub recent_days:u32,
}
impl Filter{
 pub fn validate(&self)->Result<(),String>{
  if self.name.chars().count()>160||self.extension.chars().count()>20||self.content.chars().count()>200||self.recent_days>3650||self.min_kib>4_194_304||self.max_kib>4_194_304{return Err("Filter exceeds limits".into());}
  if self.max_kib!=0&&self.min_kib>self.max_kib{return Err("Minimum size exceeds maximum".into());}
  if self.extension.trim_start_matches('.').chars().any(|c|!c.is_alphanumeric()){return Err("Use a single extension, e.g. pdf".into());}
  Ok(())
 }
}
#[derive(Debug)]
pub struct Hit{pub path:PathBuf,pub bytes:u64,pub modified:Option<u64>}
#[derive(Debug,Default)]
pub struct Report{pub hits:Vec<Hit>,pub scanned:usize,pub text_reads:usize,pub skipped:usize,pub limited:bool,pub cancelled:bool}
fn matches_metadata(f:&Filter,path:&Path,size:u64,modified:Option<SystemTime>,now:SystemTime)->bool{
 let name=path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
 let ext=f.extension.trim().trim_start_matches('.').to_lowercase();
 if !name.contains(&f.name.trim().to_lowercase())||(!ext.is_empty()&&path.extension().and_then(|x|x.to_str()).unwrap_or("").to_lowercase()!=ext){return false;}
 if size<u64::from(f.min_kib)*1024||(f.max_kib>0&&size>u64::from(f.max_kib)*1024){return false;}
 if f.recent_days>0&&!modified.and_then(|m|now.duration_since(m).ok()).is_some_and(|d|d<=Duration::from_secs(u64::from(f.recent_days)*86400)){return false;}
 true
}
#[cfg(test)]
pub fn search(scopes:&Scopes,root:&Path,f:&Filter,cancel:&AtomicBool)->Result<Report,String>{search_until(scopes,root,f,cancel,Instant::now()+Duration::from_secs(60))}
pub fn search_until(scopes:&Scopes,root:&Path,f:&Filter,cancel:&AtomicBool,deadline:Instant)->Result<Report,String>{
 if cancel.load(Ordering::Relaxed)||Instant::now()>=deadline{return Ok(Report{cancelled:true,..Default::default()});}
 f.validate()?;let root=scopes.checked(root)?;
 if !root.is_dir(){return Err("Search root must be a folder".into());}
 let start=Instant::now();let now=SystemTime::now();let needle=f.content.trim().to_lowercase();
 let mut report=Report::default();let mut dirs=vec![root];
 'scan:while let Some(dir)=dirs.pop(){
  if cancel.load(Ordering::Relaxed)||Instant::now()>=deadline{report.cancelled=true;break;}
  if start.elapsed()>=TIME_LIMIT{report.limited=true;break;}
  if scopes.checked(&dir).is_err(){report.skipped+=1;continue;}
  let rows=match fs::read_dir(&dir){Ok(r)=>r,Err(_)=>{report.skipped+=1;continue;}};
  for row in rows{
   if cancel.load(Ordering::Relaxed)||Instant::now()>=deadline{report.cancelled=true;break 'scan;}
   if report.scanned>=SCAN_LIMIT||report.hits.len()>=RESULT_LIMIT||start.elapsed()>=TIME_LIMIT{report.limited=true;break 'scan;}
   report.scanned+=1;
   let Ok(row)=row else{report.skipped+=1;continue;};
   let path=row.path();let Ok(meta)=fs::symlink_metadata(&path)else{report.skipped+=1;continue;};
   if !ordinary(&meta)||scopes.checked(&path).is_err(){report.skipped+=1;continue;}
   if meta.is_dir(){dirs.push(path);continue;}
   let modified=meta.modified().ok();
   if !matches_metadata(f,&path,meta.len(),modified,now){continue;}
   if !needle.is_empty(){
    if report.text_reads>=TEXT_LIMIT{report.limited=true;break 'scan;}
    if cancel.load(Ordering::Relaxed)||Instant::now()>=deadline{report.cancelled=true;break 'scan;}
    report.text_reads+=1;
    if !scopes.read_text(&path).is_ok_and(|t|t.to_lowercase().contains(&needle)){continue;}
   }
   if scopes.checked(&path).is_err(){report.skipped+=1;continue;}
   report.hits.push(Hit{path,bytes:meta.len(),modified:modified.and_then(|m|m.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d|d.as_secs())});
  }
 }
 if cancel.load(Ordering::Relaxed)||Instant::now()>=deadline{report.cancelled=true;report.hits.clear();}
 report.hits.sort_by(|a,b|b.modified.cmp(&a.modified).then(a.path.cmp(&b.path)));
 Ok(report)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn metadata_filters(){
  let f=Filter{name:"ЗВІТ".into(),extension:"PDF".into(),min_kib:1,max_kib:2,recent_days:1,..Default::default()};
  let now=SystemTime::now();assert!(matches_metadata(&f,Path::new("Звіт.pdf"),1500,Some(now),now));
  assert!(!matches_metadata(&f,Path::new("Звіт.txt"),1500,Some(now),now));
  assert!(!matches_metadata(&f,Path::new("Звіт.pdf"),3000,Some(now),now));
  assert!(!matches_metadata(&f,Path::new("Звіт.pdf"),1500,Some(now-Duration::from_secs(172800)),now));
 }
 #[test]fn expired_search_never_reads_root(){let scopes=Scopes::default();let report=search_until(&scopes,Path::new("not-granted"),&Filter::default(),&AtomicBool::new(false),Instant::now()).unwrap();assert!(report.cancelled);assert!(report.hits.is_empty());assert_eq!(report.scanned,0);}
 #[test]fn invalid_filters(){assert!(Filter{min_kib:5,max_kib:3,..Default::default()}.validate().is_err());assert!(Filter{extension:"../pdf".into(),..Default::default()}.validate().is_err());}
 #[test]fn scoped_search(){
  let mut id=[0u8;16];getrandom::getrandom(&mut id).unwrap();let name=id.iter().map(|b|format!("{b:02x}")).collect::<String>();
  let path=std::env::temp_dir().join(format!("hatoriel-search-{name}"));fs::create_dir(&path).unwrap();let root=fs::canonicalize(&path).unwrap();
  fs::create_dir(root.join("nested")).unwrap();fs::write(root.join("nested").join("note.txt"),"Привіт Rust").unwrap();fs::write(root.join("other.txt"),"no match").unwrap();
  let mut scopes=Scopes::default();let cancel=AtomicBool::new(false);let f=Filter{extension:"txt".into(),content:"привіт".into(),..Default::default()};
  assert!(search(&scopes,&root,&f,&cancel).is_err());scopes.grant(&root).unwrap();
  let report=search(&scopes,&root,&f,&cancel).unwrap();assert_eq!(report.hits.len(),1);assert_eq!(report.hits[0].path,root.join("nested").join("note.txt"));
  cancel.store(true,Ordering::Relaxed);assert!(search(&scopes,&root,&f,&cancel).unwrap().cancelled);
  fs::remove_dir_all(path).unwrap();
 }
}
