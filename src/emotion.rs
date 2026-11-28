//! Local heuristic tone cues, not a diagnosis or genuine emotional state.
use serde::{Serialize,Deserialize};
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub enum Mood{Calm,Joy,Concern,Surprise,Curiosity}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(default)]
pub struct State{pub mood:Mood,pub intensity:u8,pub updated:u64}
impl Default for State{fn default()->Self{Self{mood:Mood::Calm,intensity:0,updated:0}}}
pub fn now()->u64{std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Cue{pub mood:Mood,pub strength:u8,pub serious:bool}
fn words(input:&str)->Vec<String>{
 let input:String=input.chars().take(12000).collect::<String>().to_lowercase();
 input.split(|c:char|!c.is_alphanumeric()&&c!='\''&&c!='’').filter(|s|!s.is_empty()).map(str::to_owned).collect()
}
fn has(w:&[String],phrases:&[&str])->bool{
 phrases.iter().any(|p|{let parts:Vec<_>=p.split_whitespace().collect();!parts.is_empty()&&w.windows(parts.len()).any(|s|s.iter().map(String::as_str).eq(parts.iter().copied()))})
}
pub fn classify(input:&str)->Cue{
 let w=words(input);
 // Prefer explicit requests and problem context over cheerfulness.
 let serious=has(&w,&["серйозно","без жартів","терміново","помилка","помилки","не працює","не вийшло","не виходить","зламалося","error","failed","failure","crash","crashed","serious","no jokes","urgent","not working","doesn't work","не работает","ошибка","срочно"]);
 let distress=has(&w,&["мені сумно","мені боляче","мені важко","я засмучений","я засмучена","я боюся","мені страшно","сумно","боляче","sad","upset","i am scared","i'm scared","i feel awful","i'm struggling","мне грустно","мне больно","мне страшно"]);
 let negated=has(&w,&["не сумно","не боляче","не страшно","not sad","not upset","not scared","не грустно","не больно"]);
 let neutral=has(&w,&["нейтрально","без емоцій","спокійно","neutral tone","no emotion","calm tone","без эмоций"]);
 let (mood,strength)=if distress&&!negated{(Mood::Concern,100)}
 else if neutral{(Mood::Calm,0)}
 else if serious{(Mood::Calm,0)}
 else if has(&w,&["дякую","чудово","ура","супер","все працює","thank you","thanks","great","awesome","it works","спасибо","отлично"]){(Mood::Joy,85)}
 else if has(&w,&["ого","wow","неочікувано","несподівано","unexpected","удивительно"]){(Mood::Surprise,70)}
 else if input.contains('?')||has(&w,&["цікаво","поясни","чому","як працює","curious","explain","why","интересно"]){(Mood::Curiosity,55)}
 else{(Mood::Calm,0)};
 Cue{mood,strength,serious:serious||(distress&&!negated)}
}
impl Mood{
 pub fn label(self,en:bool)->&'static str{match(self,en){
 (Self::Calm,true)=>"Calm",(Self::Calm,false)=>"Спокій",
 (Self::Joy,true)=>"Joy",(Self::Joy,false)=>"Радість",
 (Self::Concern,true)=>"Concern",(Self::Concern,false)=>"Турбота",
 (Self::Surprise,true)=>"Surprise",(Self::Surprise,false)=>"Здивування",
 (Self::Curiosity,true)=>"Curiosity",(Self::Curiosity,false)=>"Зацікавленість",
 }}
}
impl State{
 pub fn decay(&mut self,t:u64){
  self.intensity=self.intensity.min(100);
  if self.updated>t{self.updated=t;}
  let steps=t.saturating_sub(self.updated)/60;
  if steps>0{self.intensity=self.intensity.saturating_sub(steps.saturating_mul(5).min(100) as u8);self.updated=self.updated.saturating_add(steps*60);}
  if self.intensity==0||self.mood==Mood::Calm{self.mood=Mood::Calm;self.intensity=0;}
 }
 pub fn observe(&mut self,input:&str,t:u64,expressiveness:u8){
  self.decay(t);let cap=expressiveness.min(100);let cue=classify(input);
  if cap==0{*self=Self{updated:t,..Default::default()};return;}
  if cue.mood==Mood::Calm{
   // Neutral turns soften prior expression; explicit serious requests immediately clear it.
   self.intensity=if cue.serious||has(&words(input),&["нейтрально","без емоцій","спокійно","neutral tone","no emotion","calm tone","без эмоций"]){0}else{self.intensity.min(cap).saturating_sub(15)};
  }else{
   let target=((u16::from(cap)*u16::from(cue.strength))/100) as u8;
   self.intensity=if self.mood==cue.mood{self.intensity.min(cap).max(target).saturating_add(8).min(cap)}
     else if cue.mood==Mood::Concern||self.mood==Mood::Calm{target}
     else{((u16::from(self.intensity.min(cap))+2*u16::from(target))/3) as u8};
   self.mood=cue.mood;
  }
  if self.intensity==0{self.mood=Mood::Calm;}
  self.updated=t;
 }
 pub fn guidance(&self)->String{
  let tone=match self.mood{Mood::Calm=>"steady",Mood::Joy=>"warm and upbeat",Mood::Concern=>"gentle and supportive",Mood::Surprise=>"mildly surprised",Mood::Curiosity=>"interested and attentive"};
  format!("Simulated expression: {tone}, intensity {}/100. These are heuristic conversation cues, not facts about the user's emotional or mental state. Adapt to actual needs first; do not infer a diagnosis, pressure the user, or claim genuine feelings or consciousness.",self.intensity.min(100))
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn decay(){let mut s=State::default();s.observe("Дякую",100,80);assert_eq!(s.mood,Mood::Joy);s.decay(1060);assert_eq!(s.mood,Mood::Calm);assert_eq!(s.intensity,0);}
 #[test]fn concern_first(){let mut s=State::default();s.observe("Мені сумно?",10,200);assert_eq!(s.mood,Mood::Concern);assert_eq!(s.intensity,100);}
 #[test]fn multilingual_and_word_boundaries(){assert_eq!(classify("Дякую!").mood,Mood::Joy);assert_eq!(classify("wow").mood,Mood::Surprise);assert_eq!(classify("shadow").mood,Mood::Calm);assert_eq!(classify("sadness is a word").mood,Mood::Calm);assert_eq!(classify("мне грустно").mood,Mood::Concern);assert_eq!(classify("not sad").mood,Mood::Calm);}
 #[test]fn serious_overrides_joy(){let c=classify("Дякую, але не працює");assert!(c.serious);assert_eq!(c.mood,Mood::Calm);}
 #[test]fn bounded_transitions(){let mut s=State::default();s.observe("дякую",100,80);let first=s.intensity;s.observe("дякую",101,80);assert!(s.intensity>first&&s.intensity<=80);s.observe("мені важко",102,40);assert_eq!((s.mood,s.intensity),(Mood::Concern,40));s.observe("добре",103,40);assert_eq!(s.intensity,25);s.observe("нейтрально",104,40);assert_eq!(s.mood,Mood::Calm);s.observe("дякую",105,0);assert_eq!(s.intensity,0);}
 #[test]fn decay_preserves_remainder_and_future_time(){let mut s=State{mood:Mood::Joy,intensity:80,updated:100};s.decay(190);assert_eq!((s.intensity,s.updated),(75,160));s.decay(220);assert_eq!(s.intensity,70);s.decay(10);assert_eq!(s.updated,10);}
 #[test]fn legacy_serialization(){let s:State=serde_json::from_str(r#"{"mood":"Joy","intensity":50,"updated":100}"#).unwrap();assert_eq!(s.mood,Mood::Joy);assert_eq!(s.intensity,50);}
}
