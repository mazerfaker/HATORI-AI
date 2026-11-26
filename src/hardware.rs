#[derive(Debug, Default)]
pub struct Hardware { pub cpu: String, pub ram_gib: Option<u64>, pub gpus: Vec<String> }

pub fn recommend(ram: Option<u64>) -> &'static str {
    match ram { Some(n) if n >= 16 => "qwen2.5:7b", Some(n) if n >= 8 => "qwen2.5:3b", _ => "qwen2.5:1.5b" }
}

pub fn scan() -> Result<Hardware, String> {
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("powershell.exe")
            .creation_flags(0x08000000)
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; $c=Get-CimInstance Win32_ComputerSystem; $p=Get-CimInstance Win32_Processor; $g=Get-CimInstance Win32_VideoController; @{cpu=($p.Name -join '; '); ram=$c.TotalPhysicalMemory; gpus=@($g | ForEach-Object {$_.Name})} | ConvertTo-Json -Compress"])
            .output().map_err(|e|e.to_string())?;
        if !output.status.success() { return Err("Windows hardware query failed".into()); }
        let v:serde_json::Value=serde_json::from_slice(&output.stdout).map_err(|e|e.to_string())?;
        Ok(Hardware {cpu:v["cpu"].as_str().unwrap_or("Unknown").into(),ram_gib:v["ram"].as_u64().map(|n|n/1073741824),gpus:v["gpus"].as_array().into_iter().flatten().filter_map(|x|x.as_str().map(str::to_owned)).collect()})
    }
    #[cfg(not(windows))] { Err("Hardware scan not implemented on this platform".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn conservative_tiers(){assert_eq!(recommend(None),"qwen2.5:1.5b");assert_eq!(recommend(Some(8)),"qwen2.5:3b");assert_eq!(recommend(Some(16)),"qwen2.5:7b");}
}
