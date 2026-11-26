use serde::{Serialize,Deserialize};
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub enum Mood{Calm,Joy,Concern,Surprise,Curiosity}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(default)]
pub struct State{pub mood:Mood,pub intensity:u8,pub updated:u64}
impl Default for State{fn default()->Self{Self{mood:Mood::Calm,intensity:0,updated:0}}}
pub fn now()->u64{std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()}
impl State{
 pub fn decay(&mut self,t:u64){let steps=t.saturating_sub(self.updated)/60;if steps>0{self.intensity=self.intensity.min(100).saturating_sub(steps.saturating_mul(5).min(100) as u8);self.updated=t;if self.intensity==0{self.mood=Mood::Calm;}}}
 pub fn observe(&mut self,input:&str,t:u64,expressiveness:u8){self.decay(t);let s=input.to_lowercase();let has=|terms:&[&str]|terms.iter().any(|x|s.contains(*x));let mood=if has(&["сумно","боляче","важко","sad","upset"]){Mood::Concern}else if has(&["дякую","чудово","ура!","thank you","great!"]){Mood::Joy}else if has(&["ого","wow","неочікувано"]){Mood::Surprise}else if s.contains('?'){Mood::Curiosity}else{Mood::Calm};if mood!=Mood::Calm{self.mood=mood;self.intensity=expressiveness.min(100);self.updated=t;}}
 pub fn guidance(&self)->String{let tone=match self.mood{Mood::Calm=>"steady",Mood::Joy=>"warm and upbeat",Mood::Concern=>"gentle and supportive",Mood::Surprise=>"mildly surprised",Mood::Curiosity=>"interested and attentive"};format!("Simulated expression: {tone}, intensity {}/100. Adapt to the user's actual needs first; never claim genuine feelings or consciousness.",self.intensity.min(100))}
}
#[cfg(test)]mod tests{use super::*;#[test]fn decay(){let mut s=State::default();s.observe("Дякую",100,80);assert_eq!(s.mood,Mood::Joy);s.decay(1060);assert_eq!(s.mood,Mood::Calm);assert_eq!(s.intensity,0);}#[test]fn concern_first(){let mut s=State::default();s.observe("Мені сумно?",10,200);assert_eq!(s.mood,Mood::Concern);assert_eq!(s.intensity,100);}}
