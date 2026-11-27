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
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum State{Idle,Preparing,Listening,Thinking,Responding,Synthesizing,Speaking}
impl State{
 pub fn from_activity(listening:bool,capturing:bool,busy:bool,streaming:bool,synthesizing:bool,speaking:bool)->Self{
  if listening{Self::Listening}else if capturing{Self::Preparing}else if busy{if streaming{Self::Responding}else{Self::Thinking}}else if synthesizing{Self::Synthesizing}else if speaking{Self::Speaking}else{Self::Idle}
 }
 pub fn label(self)->&'static str{match self{Self::Idle=>"STANDBY",Self::Preparing=>"MIC SETUP",Self::Listening=>"LISTENING",Self::Thinking=>"THINKING",Self::Responding=>"RESPONDING",Self::Synthesizing=>"SYNTHESIZING",Self::Speaking=>"SPEAKING"}}
 pub fn animated(self)->bool{self!=Self::Idle}
 fn pulse(self,t:f32,motion:bool)->f32{
  if !motion{return 1.;}
  let (speed,amp)=match self{Self::Idle=>(1.5,0.025),Self::Preparing=>(2.,0.015),Self::Listening=>(3.,0.05),Self::Thinking=>(2.8,0.03),Self::Responding=>(5.,0.055),Self::Synthesizing=>(3.8,0.035),Self::Speaking=>(6.,0.065)};
  1.+(t*speed).sin()*amp
 }
}
pub fn reticle(p:&Painter,c:Pos2,r:f32,t:f32,state:State,motion:bool,mood:u8){
 let t=if motion{t}else{0.};let r=r*state.pulse(t,motion);
 let glow=0.035+f32::from(mood.min(100))*0.00035;
 for i in 1..=3{p.circle_stroke(c,r,Stroke::new(i as f32*2.,RED.gamma_multiply(glow)));}
 p.circle_stroke(c,r,Stroke::new(2.,RED));
 for i in 0..4{let a=i as f32*std::f32::consts::FRAC_PI_2;let d=Vec2::new(a.cos(),a.sin());p.line_segment([c+d*r*0.68,c+d*r*1.42],Stroke::new(2.,RED));}
 p.circle_filled(c,(r*0.055).max(1.5),RED);
 match state{
  State::Thinking|State::Synthesizing|State::Preparing=>{
   let speed=if state==State::Synthesizing{-2.}else{1.5};
   let count=if state==State::Preparing{2}else{3};
   for i in 0..count{let a=t*speed+i as f32*std::f32::consts::TAU/count as f32;let points=(0..=16).map(|j|{let b=a+j as f32*0.065;c+Vec2::new(b.cos(),b.sin())*r*1.65}).collect();p.add(egui::Shape::line(points,Stroke::new(1.5,RED.gamma_multiply(0.7))));}
  },
  State::Listening=>{p.circle_stroke(c,r*1.54,Stroke::new(1.,RED.gamma_multiply(0.45)));p.circle_filled(c+Vec2::new(0.,-r*1.72),2.,RED);},
  State::Responding=>{p.circle_stroke(c,r*(1.48+0.08*(t*4.).sin()),Stroke::new(1.,RED.gamma_multiply(0.35)));},
  State::Speaking=>{
   for i in -3i32..=3{let x=i as f32*r*0.28;let h=r*(0.12+0.17*(t*6.+i as f32*0.9).sin().abs());let y=r*1.55;p.line_segment([c+Vec2::new(x,y-h),c+Vec2::new(x,y+h)],Stroke::new(1.4,RED.gamma_multiply(0.8)));}
  },
  State::Idle=>{}
 }
}
pub fn icon_rgba()->Vec<u8>{
 let mut rgba=Vec::with_capacity(32*32*4);
 for y in 0..32{for x in 0..32{let dx=x as f32-15.5;let dy=y as f32-15.5;let d=(dx*dx+dy*dy).sqrt();let mark=(d-10.).abs()<1.1||(dx.abs()<1.&&dy.abs()>7.&&dy.abs()<15.)||(dy.abs()<1.&&dx.abs()>7.&&dx.abs()<15.)||d<1.8;rgba.extend_from_slice(if mark{&[238,52,52,255]}else{&[0,0,0,0]});}}rgba
}
#[cfg(test)]mod tests{use super::*;#[test]fn state_priority(){assert_eq!(State::from_activity(true,true,true,true,true,true),State::Listening);assert_eq!(State::from_activity(false,true,false,false,false,false),State::Preparing);assert_eq!(State::from_activity(false,false,true,true,false,false),State::Responding);assert_eq!(State::from_activity(false,false,false,false,true,true),State::Synthesizing);assert_eq!(State::from_activity(false,false,false,false,false,true),State::Speaking);assert_eq!(State::from_activity(false,false,false,false,false,false),State::Idle);}#[test]fn motion_disabled(){for s in [State::Idle,State::Listening,State::Speaking,State::Thinking]{assert_eq!(s.pulse(100.,false),1.);assert!((0.9..=1.1).contains(&s.pulse(100.,true)));}}#[test]fn icon_has_transparency(){let v=icon_rgba();assert_eq!(v.len(),4096);assert!(v.chunks_exact(4).any(|p|p[3]==255));assert!(v.chunks_exact(4).any(|p|p[3]==0));}}
