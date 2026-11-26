use std::path::{Path,PathBuf};
pub fn root()->Option<PathBuf>{std::env::current_exe().ok()?.parent().map(Path::to_path_buf)}
pub fn portable()->bool{root().map(|p|p.join("portable.flag").is_file()).unwrap_or(false)}
pub fn data_dir()->PathBuf{
 if portable(){return root().unwrap_or_else(||PathBuf::from(".")).join("data");}
 std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(||PathBuf::from(".")).join("HATORI_AI")
}
pub fn resolve(value:&str)->PathBuf{
 let p=PathBuf::from(value.trim());
 if portable()&&p.is_relative(){root().unwrap_or_else(||PathBuf::from(".")).join(p)}else{p}
}
pub fn store(path:&Path)->String{
 if portable(){if let Some(base)=root(){if let Ok(relative)=path.strip_prefix(base){return relative.to_string_lossy().into_owned();}}}
 path.to_string_lossy().into_owned()
}
