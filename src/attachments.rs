use std::{io::Read,path::Path};
use eframe::egui;
pub const MAX_FILE:usize=8*1024*1024;
pub const MAX_TOTAL:usize=24*1024*1024;
pub const MAX_COUNT:usize=4;

pub struct Attachment{
 pub name:String,pub bytes:Vec<u8>,pub preview:Option<egui::TextureHandle>,
}
fn safe_name(name:&str)->String{
 let s:String=name.chars().filter(|c|!c.is_control()).take(160).collect();
 if s.trim().is_empty(){"attachment".into()}else{s}
}
fn thumbnail(bytes:&[u8])->Option<egui::ColorImage>{
 let mut reader=image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?;
 if !matches!(reader.format(),Some(image::ImageFormat::Png|image::ImageFormat::Jpeg)){return None;}
 let mut limits=image::Limits::default();limits.max_image_width=Some(4096);limits.max_image_height=Some(4096);limits.max_alloc=Some(80*1024*1024);
 reader.limits(limits);
 let img=reader.decode().ok()?.thumbnail(160,120).to_rgba8();
 let size=[img.width() as usize,img.height() as usize];Some(egui::ColorImage::from_rgba_unmultiplied(size,img.as_raw()))
}
#[derive(Default)]
pub struct Draft{pub items:Vec<Attachment>,pub status:String}
impl Draft{
 pub fn empty(&self)->bool{self.items.is_empty()}
 pub fn clear(&mut self){self.items.clear();self.status.clear();}
 fn capacity(&self,n:usize)->Result<(),String>{
  if self.items.len()>=MAX_COUNT{return Err("Maximum four attachments".into());}
  if n==0||n>MAX_FILE{return Err("File must be 1 byte–8 MiB".into());}
  if self.items.iter().map(|a|a.bytes.len()).sum::<usize>()+n>MAX_TOTAL{return Err("Total limit 24 MiB".into());}Ok(())
 }
 pub fn add(&mut self,ctx:&egui::Context,name:&str,bytes:Vec<u8>)->Result<(),String>{
  self.capacity(bytes.len())?;
  if self.items.iter().any(|a|a.bytes==bytes){return Err("Duplicate attachment".into());}
  let preview=thumbnail(&bytes).map(|img|ctx.load_texture("attachment-preview",img,egui::TextureOptions::LINEAR));
  self.items.push(Attachment{name:safe_name(name),bytes,preview});self.status="Local draft only / Локальна чернетка".into();Ok(())
 }
 pub fn file(&mut self,ctx:&egui::Context,path:&Path)->Result<(),String>{
  let f=std::fs::File::open(path).map_err(|e|e.to_string())?;
  let meta=f.metadata().map_err(|e|e.to_string())?;
  if !meta.is_file(){return Err("Only regular files are allowed".into());}
  if meta.len()>MAX_FILE as u64{return Err("File exceeds 8 MiB".into());}
  self.capacity(meta.len() as usize)?;
  let mut bytes=Vec::new();f.take(MAX_FILE as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
  self.add(ctx,path.file_name().and_then(|n|n.to_str()).unwrap_or("attachment"),bytes)
 }
 #[cfg(windows)]
 pub fn paste_image(&mut self,ctx:&egui::Context)->Result<(),String>{
  self.capacity(1)?;
  let mut clipboard=arboard::Clipboard::new().map_err(|e|e.to_string())?;
  let img=clipboard.get_image().map_err(|_|"Clipboard has no supported image")?;
  let pixels=img.width.checked_mul(img.height).ok_or("Image too large")?;
  if img.width==0||img.height==0||pixels>4096*4096||img.width>4096||img.height>4096||img.bytes.len()!=pixels*4{return Err("Clipboard image exceeds 4096×4096 or is invalid".into());}
  use image::ImageEncoder;
  let mut bytes=Vec::new();
  image::codecs::png::PngEncoder::new(&mut bytes).write_image(img.bytes.as_ref(),img.width as u32,img.height as u32,image::ExtendedColorType::Rgba8).map_err(|e|e.to_string())?;
  self.add(ctx,"clipboard.png",bytes)
 }
 pub fn image_only(&self)->bool{!self.empty()&&self.items.iter().all(|a|a.preview.is_some())}
 pub fn normalized(&self)->Result<Vec<Vec<u8>>,String>{
  if !self.image_only(){return Err("Only valid PNG and JPEG images are supported for vision".into());}
  let mut out=Vec::new();
  for a in &self.items{
   let mut reader=image::ImageReader::new(std::io::Cursor::new(&a.bytes)).with_guessed_format().map_err(|e|e.to_string())?;
   let mut limits=image::Limits::default();limits.max_image_width=Some(4096);limits.max_image_height=Some(4096);limits.max_alloc=Some(80*1024*1024);reader.limits(limits);
   let image=reader.decode().map_err(|e|e.to_string())?.thumbnail(1536,1536).to_rgb8();
   let mut bytes=Vec::new();
   image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes,88).encode(image.as_raw(),image.width(),image.height(),image::ExtendedColorType::Rgb8).map_err(|e|e.to_string())?;
   out.push(bytes);
  }
  Ok(out)
 }
 pub fn panel(&mut self,ui:&mut egui::Ui,enabled:bool){
  ui.horizontal_wrapped(|ui|{
   #[cfg(windows)]{
    if ui.add_enabled(enabled,egui::Button::new("＋ Файл / File")).clicked(){
     if let Some(paths)=rfd::FileDialog::new().pick_files(){for p in paths.into_iter().take(MAX_COUNT){if let Err(e)=self.file(ui.ctx(),&p){self.status=e;break;}}}
    }
    if ui.add_enabled(enabled,egui::Button::new("Вставити зображення / Paste image")).clicked(){if let Err(e)=self.paste_image(ui.ctx()){self.status=e;}}
   }
   if ui.add_enabled(enabled&&!self.empty(),egui::Button::new("Очистити / Clear")).clicked(){self.clear();}
   ui.label(format!("{}/4 · Drop files / Перетягни файли",self.items.len()));
  });
  if !self.empty(){
   ui.label(if self.image_only(){"PNG/JPEG · vision via local Ollama only / Аналіз через локальний Ollama"}else{"Documents · extract and review first / Спочатку вилучи й перевір текст"});
   let mut remove=None;
   egui::ScrollArea::vertical().id_salt("attachment-draft").max_height(144.).show(ui,|ui|{
    for (i,a) in self.items.iter().enumerate(){ui.horizontal(|ui|{
     if let Some(texture)=&a.preview{ui.add(egui::Image::new(texture).max_size(egui::vec2(100.,65.)));}
     ui.vertical(|ui|{ui.label(&a.name);ui.small(format!("{:.1} KiB",a.bytes.len() as f64/1024.));});
     if ui.add_enabled(enabled,egui::Button::new("×")).on_hover_text("Remove attachment").clicked(){remove=Some(i);}
    });}
   });
   if let Some(i)=remove{self.items.remove(i);}
  }
  if !self.status.is_empty(){ui.small(&self.status);}
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn limits(){let mut d=Draft::default();assert!(d.capacity(0).is_err());assert!(d.capacity(MAX_FILE+1).is_err());for _ in 0..3{d.items.push(Attachment{name:"x".into(),bytes:vec![0;MAX_FILE],preview:None});}assert!(d.capacity(1).is_err());d.clear();assert!(d.capacity(MAX_FILE).is_ok());}
 #[test]fn invalid_image(){assert!(thumbnail(b"not image").is_none());assert_eq!(safe_name("a\nb"),"ab");}
}
