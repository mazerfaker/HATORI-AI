use std::path::{Path,PathBuf};
#[derive(Debug)]
pub struct Entry{pub path:PathBuf,pub bytes:u64}
pub fn scan(root:&Path)->Result<Vec<Entry>,String>{
 if !root.is_dir(){return Err("Select a model directory".into());}
 let mut out=Vec::new();let mut pending=vec![(root.to_path_buf(),0)];let mut visited=0;
 while let Some((dir,depth))=pending.pop(){
 for item in std::fs::read_dir(dir).map_err(|e|e.to_string())?{
 visited+=1;if visited>20000{return Err("Scan limit reached; select a smaller directory".into());}
 let item=item.map_err(|e|e.to_string())?;let ty=item.file_type().map_err(|e|e.to_string())?;let path=item.path();
 if ty.is_dir()&&depth<4{pending.push((path,depth+1));}
 else if ty.is_file()&&path.extension().and_then(|x|x.to_str()).map(|x|x.eq_ignore_ascii_case("gguf")).unwrap_or(false)&&crate::native_runtime::validate_model(&path).is_ok(){out.push(Entry{path,bytes:item.metadata().map_err(|e|e.to_string())?.len()});}
 }}
 out.sort_by(|a,b|a.path.cmp(&b.path));Ok(out)
}
