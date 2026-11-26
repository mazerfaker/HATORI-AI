#[cfg(windows)]
fn shortcut() -> Result<std::path::PathBuf,String> {
 let base=std::env::var_os("APPDATA").ok_or("APPDATA unavailable")?;
 Ok(std::path::PathBuf::from(base).join("Microsoft/Windows/Start Menu/Programs/Startup/HATORI AI.lnk"))
}
pub fn enabled()->bool {
 #[cfg(windows)] { shortcut().map(|p|p.is_file()).unwrap_or(false) }
 #[cfg(not(windows))] {false}
}
pub fn set(on:bool)->Result<(),String>{
 #[cfg(windows)] {
 use std::os::windows::process::CommandExt;
 let path=shortcut()?;
 if !on {return match std::fs::remove_file(path){Ok(())=>Ok(()),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),Err(e)=>Err(e.to_string())};}
 let exe=std::env::current_exe().map_err(|e|e.to_string())?;
 let status=std::process::Command::new("powershell.exe").creation_flags(0x08000000)
 .env("HATORI_SHORTCUT",path).env("HATORI_EXE",exe)
 .args(["-NoProfile","-NonInteractive","-Command","$ErrorActionPreference='Stop'; $s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:HATORI_SHORTCUT); $s.TargetPath=$env:HATORI_EXE; $s.Save()"])
 .status().map_err(|e|e.to_string())?;
 if status.success(){Ok(())}else{Err("Startup shortcut creation failed".into())}
 }
 #[cfg(not(windows))] {let _=on;Err("Windows only".into())}
}
