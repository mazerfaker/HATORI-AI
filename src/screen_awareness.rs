//! Explicit single-frame capture of the primary Windows desktop.
use eframe::egui;
const MAX_DIM:u32=4096;
const MAX_PIXELS:usize=16*1024*1024;
const PREVIEW_DIM:u32=1920;
pub struct Snapshot{pub bytes:Vec<u8>,pub texture:egui::TextureHandle,pub source_size:[u32;2]}
fn pixel_count(w:u32,h:u32)->Result<usize,String>{
 if w==0||h==0||w>MAX_DIM||h>MAX_DIM{return Err("Primary monitor exceeds supported 4096×4096 bounds".into());}
 let n=(w as usize).checked_mul(h as usize).ok_or("Invalid screen dimensions")?;
 if n>MAX_PIXELS{return Err("Screen pixel limit exceeded".into());}Ok(n)
}
fn convert_bgra(w:u32,h:u32,raw:&[u8])->Result<image::RgbImage,String>{
 let n=pixel_count(w,h)?;
 if raw.len()!=n*4{return Err("Invalid screen pixel buffer".into());}
 let mut rgb=Vec::with_capacity(n*3);
 for p in raw.chunks_exact(4){rgb.extend_from_slice(&[p[2],p[1],p[0]]);}
 image::RgbImage::from_raw(w,h,rgb).ok_or_else(||"Invalid RGB buffer".into())
}
fn encode(w:u32,h:u32,raw:&[u8])->Result<Vec<u8>,String>{
 let img=image::DynamicImage::ImageRgb8(convert_bgra(w,h,raw)?).thumbnail(PREVIEW_DIM,PREVIEW_DIM).to_rgb8();
 let mut data=Vec::new();image::codecs::jpeg::JpegEncoder::new_with_quality(&mut data,90).encode(img.as_raw(),img.width(),img.height(),image::ExtendedColorType::Rgb8).map_err(|e|e.to_string())?;
 if data.is_empty()||data.len()>crate::attachments::MAX_FILE{return Err("Captured image exceeds attachment limit".into());}Ok(data)
}
#[cfg(windows)]
mod native{
 use super::*;
 use std::{ffi::c_void,ptr::null_mut};
 type Handle=*mut c_void;
 #[repr(C)]struct BitmapInfoHeader{size:u32,width:i32,height:i32,planes:u16,bit_count:u16,compression:u32,size_image:u32,x_pels:i32,y_pels:i32,clr_used:u32,clr_important:u32}
 #[repr(C)]struct BitmapInfo{header:BitmapInfoHeader,colors:[u32;1]}
 #[link(name="user32")]extern "system"{
  fn GetSystemMetrics(index:i32)->i32;
  fn GetDC(hwnd:Handle)->Handle;
  fn ReleaseDC(hwnd:Handle,dc:Handle)->i32;
 }
 #[link(name="gdi32")]extern "system"{
  fn CreateCompatibleDC(dc:Handle)->Handle;
  fn DeleteDC(dc:Handle)->i32;
  fn CreateDIBSection(dc:Handle,info:*const BitmapInfo,usage:u32,bits:*mut *mut c_void,section:Handle,offset:u32)->Handle;
  fn SelectObject(dc:Handle,obj:Handle)->Handle;
  fn DeleteObject(obj:Handle)->i32;
  fn BitBlt(dst:Handle,x:i32,y:i32,w:i32,h:i32,src:Handle,sx:i32,sy:i32,rop:u32)->i32;
  fn GdiFlush()->i32;
 }
 struct Resources{screen:Handle,mem:Handle,bitmap:Handle,old:Handle}
 impl Drop for Resources{
  fn drop(&mut self){unsafe{
   if !self.old.is_null()&&self.old as isize != -1&&!self.mem.is_null(){SelectObject(self.mem,self.old);}
   if !self.bitmap.is_null(){DeleteObject(self.bitmap);}
   if !self.mem.is_null(){DeleteDC(self.mem);}
   if !self.screen.is_null(){ReleaseDC(null_mut(),self.screen);}
  }}
 }
 pub fn capture()->Result<(Vec<u8>,[u32;2]),String>{
  unsafe{
   let w=GetSystemMetrics(0);let h=GetSystemMetrics(1);
   if w<=0||h<=0{return Err("Primary screen unavailable".into());}
   let n=pixel_count(w as u32,h as u32)?;
   let screen=GetDC(null_mut());
   if screen.is_null(){return Err("Cannot access desktop DC".into());}
   let mut res=Resources{screen,mem:null_mut(),bitmap:null_mut(),old:null_mut()};
   res.mem=CreateCompatibleDC(screen);
   if res.mem.is_null(){return Err("Cannot allocate capture DC".into());}
   let info=BitmapInfo{header:BitmapInfoHeader{size:std::mem::size_of::<BitmapInfoHeader>() as u32,width:w,height:-h,planes:1,bit_count:32,compression:0,size_image:(n*4) as u32,x_pels:0,y_pels:0,clr_used:0,clr_important:0},colors:[0]};
   let mut bits=null_mut();
   res.bitmap=CreateDIBSection(screen,&info,0,&mut bits,null_mut(),0);
   if res.bitmap.is_null()||bits.is_null(){return Err("Cannot allocate screenshot bitmap".into());}
   res.old=SelectObject(res.mem,res.bitmap);
   if res.old.is_null()||res.old as isize == -1{return Err("Cannot select screenshot bitmap".into());}
   // One foreground user-approved capture. No hooks, timers, or background collection.
   if BitBlt(res.mem,0,0,w,h,screen,0,0,0x00CC0020|0x40000000)==0||GdiFlush()==0{return Err("Desktop capture failed".into());}
   let raw=std::slice::from_raw_parts(bits.cast::<u8>(),n*4);
   Ok((encode(w as u32,h as u32,raw)?,[w as u32,h as u32]))
  }
 }
}
pub fn capture(ctx:&egui::Context)->Result<Snapshot,String>{
 #[cfg(windows)]{
  let (bytes,source_size)=native::capture()?;
  let img=image::load_from_memory_with_format(&bytes,image::ImageFormat::Jpeg).map_err(|e|e.to_string())?.to_rgba8();
  let tex=ctx.load_texture("screen-review",egui::ColorImage::from_rgba_unmultiplied([img.width() as usize,img.height() as usize],img.as_raw()),egui::TextureOptions::LINEAR);
  Ok(Snapshot{bytes,texture:tex,source_size})
 }
 #[cfg(not(windows))]{let _=ctx;Err("Screen capture is Windows only".into())}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn bounds(){assert!(pixel_count(0,5).is_err());assert!(pixel_count(4097,1).is_err());assert_eq!(pixel_count(3840,2160).unwrap(),8294400);}
 #[test]fn bgra_to_rgb(){let img=convert_bgra(2,1,&[3,2,1,0,30,20,10,255]).unwrap();assert_eq!(img.as_raw(),&[1,2,3,10,20,30]);assert!(convert_bgra(2,1,&[0;4]).is_err());}
 #[test]fn jpeg_snapshot(){let bytes=encode(2,2,&[127;16]).unwrap();let img=image::load_from_memory_with_format(&bytes,image::ImageFormat::Jpeg).unwrap();assert_eq!((img.width(),img.height()),(2,2));}
}
