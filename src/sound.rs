#[cfg(windows)]
#[link(name="winmm")]
extern "system" { fn PlaySoundW(sound: *const u16, module: usize, flags: u32) -> i32; }
pub fn tick(volume:u8) {
    #[cfg(windows)] {
        use std::sync::{OnceLock,Mutex};
        use std::time::{Instant,Duration};
        static LAST:OnceLock<Mutex<Instant>>=OnceLock::new();
        if volume==0{return;}
        let mut last=LAST.get_or_init(||Mutex::new(Instant::now()-Duration::from_secs(1))).lock().unwrap();
        if last.elapsed()<Duration::from_millis(65){return;} *last=Instant::now();
        std::thread::spawn(move||{
            let n=320u32; let mut wav=Vec::new();
            wav.extend(b"RIFF");wav.extend((36+n*2).to_le_bytes());wav.extend(b"WAVEfmt ");wav.extend(16u32.to_le_bytes());wav.extend(1u16.to_le_bytes());wav.extend(1u16.to_le_bytes());wav.extend(16000u32.to_le_bytes());wav.extend(32000u32.to_le_bytes());wav.extend(2u16.to_le_bytes());wav.extend(16u16.to_le_bytes());wav.extend(b"data");wav.extend((n*2).to_le_bytes());
            for i in 0..n {let a=(1.0-i as f32/n as f32)*90.0*volume as f32;let v=if (i/7)%2==0{a}else{-a};wav.extend((v as i16).to_le_bytes());}
            unsafe{PlaySoundW(wav.as_ptr().cast(),0,0x0004|0x0002);}
        });
    }
    #[cfg(not(windows))] let _=volume;
}
