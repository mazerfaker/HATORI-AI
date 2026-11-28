//! Explicit clipboard actions only. No clipboard polling, history or background capture.
use eframe::egui;
pub const MAX_TEXT:usize=64*1024;
const MAX_DIM:usize=4096;
const PREVIEW_DIM:u32=1920;
pub enum Snapshot{
 Text(String),
 Image{bytes:Vec<u8>,texture:egui::TextureHandle,width:usize,height:usize},
}
impl Snapshot{
 pub fn attachment(&self)->(&'static str,Vec<u8>){match self{
  Self::Text(text)=>("clipboard.txt",text.as_bytes().to_vec()),
  Self::Image{bytes,..}=>("clipboard.png",bytes.clone()),
 }}
 pub fn is_image(&self)->bool{matches!(self,Self::Image{..})}
}
pub fn validate_text(text:&str)->Result<(),String>{
 if text.is_empty()||text.len()>MAX_TEXT{return Err("Clipboard text must be 1 byte–64 KiB".into());}
 if text.chars().any(|c|c.is_control()&&!matches!(c,'\n'|'\r'|'\t')){return Err("Clipboard contains unsupported control characters".into());}
 Ok(())
}
fn validate_image(w:usize,h:usize,len:usize)->Result<(),String>{
 if w==0||h==0||w>MAX_DIM||h>MAX_DIM||w.checked_mul(h).and_then(|n|n.checked_mul(4))!=Some(len){return Err("Clipboard image must be valid RGBA up to 4096×4096".into());}Ok(())
}
fn encode_image(w:usize,h:usize,raw:Vec<u8>)->Result<(Vec<u8>,egui::ColorImage),String>{
 validate_image(w,h,raw.len())?;
 let img=image::RgbaImage::from_raw(w as u32,h as u32,raw).ok_or("Invalid image buffer")?;
 let img=image::DynamicImage::ImageRgba8(img).thumbnail(PREVIEW_DIM,PREVIEW_DIM).to_rgba8();
 let mut bytes=Vec::new();
 use image::ImageEncoder;
 image::codecs::png::PngEncoder::new(&mut bytes).write_image(img.as_raw(),img.width(),img.height(),image::ExtendedColorType::Rgba8).map_err(|e|e.to_string())?;
 if bytes.len()>crate::attachments::MAX_FILE{return Err("Encoded clipboard image exceeds 8 MiB".into());}
 let preview=egui::ColorImage::from_rgba_unmultiplied([img.width() as usize,img.height() as usize],img.as_raw());
 Ok((bytes,preview))
}
pub fn read_text()->Result<Snapshot,String>{
 #[cfg(windows)]{
  let mut clipboard=arboard::Clipboard::new().map_err(|e|e.to_string())?;
  let text=clipboard.get_text().map_err(|_|"Clipboard text unavailable")?;
  validate_text(&text)?;Ok(Snapshot::Text(text))
 }
 #[cfg(not(windows))]{Err("Clipboard access is Windows only".into())}
}
pub fn read_image(ctx:&egui::Context)->Result<Snapshot,String>{
 #[cfg(windows)]{
  let mut clipboard=arboard::Clipboard::new().map_err(|e|e.to_string())?;
  let img=clipboard.get_image().map_err(|_|"Clipboard image unavailable")?;
  let (width,height)=(img.width,img.height);
  validate_image(width,height,img.bytes.len())?;
  let (bytes,preview)=encode_image(width,height,img.bytes.into_owned())?;
  let texture=ctx.load_texture("clipboard-review",preview,egui::TextureOptions::LINEAR);
  Ok(Snapshot::Image{bytes,texture,width,height})
 }
 #[cfg(not(windows))]{let _=ctx;Err("Clipboard access is Windows only".into())}
}
pub fn write_text(text:&str)->Result<(),String>{
 validate_text(text)?;
 #[cfg(windows)]{
  let mut clipboard=arboard::Clipboard::new().map_err(|e|e.to_string())?;
  clipboard.set_text(text.to_owned()).map_err(|e|e.to_string())
 }
 #[cfg(not(windows))]{Err("Clipboard access is Windows only".into())}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn text_limits_and_unicode(){assert!(validate_text("Привіт\n\tRust").is_ok());assert!(validate_text("").is_err());assert!(validate_text("a\0b").is_err());assert!(validate_text(&"ї".repeat(MAX_TEXT/2+1)).is_err());}
 #[test]fn image_limits(){assert!(validate_image(2,2,16).is_ok());assert!(validate_image(0,2,0).is_err());assert!(validate_image(4097,2,32776).is_err());assert!(validate_image(2,2,15).is_err());}
 #[test]fn image_snapshot_and_text_attachment(){let (data,preview)=encode_image(2,2,vec![255;16]).unwrap();assert_eq!(preview.size,[2,2]);assert_eq!(image::load_from_memory_with_format(&data,image::ImageFormat::Png).unwrap().to_rgba8().as_raw(),&[255;16]);let s=Snapshot::Text("code".into());assert_eq!(s.attachment(),("clipboard.txt",b"code".to_vec()));}
}
