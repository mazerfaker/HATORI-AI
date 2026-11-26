use eframe::egui::{self,Color32,Stroke,Vec2,Pos2,Painter};
pub const RED:Color32=Color32::from_rgb(238,52,52);
pub fn visuals(dark:bool)->egui::Visuals{
 if !dark{return egui::Visuals::light();}
 let mut v=egui::Visuals::dark();
 v.panel_fill=Color32::from_rgb(28,28,30);
 v.window_fill=Color32::from_rgb(33,33,35);
 v.extreme_bg_color=Color32::from_rgb(20,20,22);
 v.faint_bg_color=Color32::from_rgb(36,36,39);
 v.override_text_color=Some(Color32::from_rgb(242,242,242));
 v.selection.bg_fill=Color32::from_rgb(112,22,26);
 v.selection.stroke=Stroke::new(1.,RED);
 v.widgets.inactive.bg_fill=Color32::from_rgb(43,43,46);
 v.widgets.inactive.weak_bg_fill=Color32::from_rgb(43,43,46);
 v.widgets.hovered.bg_fill=Color32::from_rgb(58,58,61);
 v.widgets.hovered.weak_bg_fill=Color32::from_rgb(58,58,61);
 v.widgets.active.bg_fill=Color32::from_rgb(112,22,26);
 v.widgets.active.weak_bg_fill=Color32::from_rgb(112,22,26);
 v
}
pub fn reticle(p:&Painter,c:Pos2,r:f32,t:f32,busy:bool){
 let r=r*(1.+(t*if busy{4.}else{1.8}).sin()*0.035);
 for i in 1..=3{p.circle_stroke(c,r,Stroke::new(i as f32*2.,RED.gamma_multiply(0.035)));}
 p.circle_stroke(c,r,Stroke::new(2.,RED));
 for i in 0..4{let a=i as f32*std::f32::consts::FRAC_PI_2;let d=Vec2::new(a.cos(),a.sin());p.line_segment([c+d*r*0.68,c+d*r*1.42],Stroke::new(2.,RED));}
 p.circle_filled(c,(r*0.055).max(1.5),RED);
 if busy{for i in 0..3{let a=t*1.5+i as f32*std::f32::consts::TAU/3.;let points=(0..=16).map(|j|{let b=a+j as f32*0.065;c+Vec2::new(b.cos(),b.sin())*r*1.65}).collect();p.add(egui::Shape::line(points,Stroke::new(1.5,RED.gamma_multiply(0.7))));}}
}
pub fn icon_rgba()->Vec<u8>{
 let mut rgba=Vec::with_capacity(32*32*4);
 for y in 0..32{for x in 0..32{let dx=x as f32-15.5;let dy=y as f32-15.5;let d=(dx*dx+dy*dy).sqrt();let mark=(d-10.).abs()<1.1||(dx.abs()<1.&&dy.abs()>7.&&dy.abs()<15.)||(dy.abs()<1.&&dx.abs()>7.&&dx.abs()<15.)||d<1.8;rgba.extend_from_slice(if mark{&[238,52,52,255]}else{&[0,0,0,0]});}}rgba
}
#[cfg(test)]mod tests{use super::*;#[test]fn icon_has_transparency(){let v=icon_rgba();assert_eq!(v.len(),4096);assert!(v.chunks_exact(4).any(|p|p[3]==255));assert!(v.chunks_exact(4).any(|p|p[3]==0));}}
