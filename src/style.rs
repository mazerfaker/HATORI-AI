//! Per-request adaptation, not automatic changes to stored personality.
use crate::{adaptive::Profile,emotion::{Mood,State},inference::Message};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Task{General,Technical,Research,Creative,Practical,Support}
impl Task{
 pub fn label(self)->&'static str{match self{Self::General=>"General",Self::Technical=>"Technical",Self::Research=>"Research",Self::Creative=>"Creative",Self::Practical=>"Practical",Self::Support=>"Support"}}
 pub fn guidance(self)->&'static str{match self{
 Self::General=>"Answer the actual question directly.",
 Self::Technical=>"Preserve code and constraints; distinguish proposed changes from verified results.",
 Self::Research=>"Distinguish sources, evidence, estimates and uncertainty; never fabricate references.",
 Self::Creative=>"Offer relevant creative options within requested constraints.",
 Self::Practical=>"Focus on usable results; never claim unperformed actions.",
 Self::Support=>"Be respectful and nonjudgmental; avoid diagnoses and assumptions.",
 }}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Plan{pub traits:[u8;7],pub length:&'static str,pub task:Task,pub inherited:bool}
fn words(s:&str,max:usize)->Vec<String>{s.chars().take(max).collect::<String>().to_lowercase().split(|c:char|!c.is_alphanumeric()).filter(|s|!s.is_empty()).map(str::to_owned).collect()}
fn has(w:&[String],phrases:&[&str])->bool{phrases.iter().any(|p|{let parts:Vec<_>=p.split_whitespace().collect();!parts.is_empty()&&w.windows(parts.len()).any(|s|s.iter().map(String::as_str).eq(parts.iter().copied()))})}
fn lengths(w:&[String])->(bool,bool){(has(w,&["коротко","стисло","лаконічно","без води","briefly","concise","short answer","кратко"]),has(w,&["детально","докладно","покроково","step by step","in detail","detailed","подробно"]))}
fn task(w:&[String])->Task{
 if has(w,&["мені сумно","мені страшно","мені боляче","мені важко","я боюся","i feel sad","i feel awful","i am scared","мне грустно"]){Task::Support}
 else if has(w,&["код","коду","компіляція","збірка","збірки","помилка","помилки","github","rust","python","java","javascript","debug","build","compile","bug","error","не працює","failed","crash","ошибка"]){Task::Technical}
 else if has(w,&["дослідження","порівняй","порівняти","джерела","докази","статистика","research","compare","sources","evidence","statistics","сравни"]){Task::Research}
 else if has(w,&["намалюй","дизайн","ідеї","сюжет","історію","вірш","пісню","арт","design","brainstorm","story","poem","draw","concept","нарисуй"]){Task::Creative}
 else if has(w,&["план","список","напиши","організуй","підготуй","зроби","розклад","plan","schedule","draft","organize","checklist"]){Task::Practical}
 else{Task::General}
}
fn continuation(w:&[String])->bool{matches!(w.join(" ").as_str(),"далі"|"давай"|"продовжуй"|"продовжити"|"перевір"|"перевір і далі"|"роби"|"ок"|"так"|"continue"|"go ahead"|"next"|"check"|"check and continue"|"yes"|"дальше"|"продолжай")}
pub fn plan(base:[u8;7],query:&str,mood:Option<&State>,mode:&str,profile:&Profile,history:&[Message])->Plan{
 let mut t=base.map(|v|v.min(100));let w=words(query,12000);let prefs=words(&profile.communication,1000);
 let mut kind=task(&w);let mut inherited=false;
 if kind==Task::General&&continuation(&w){
  for m in history.iter().rev().filter(|m|m.role=="user").skip(1).take(3){
   let candidate=task(&words(&m.content,12000));if candidate!=Task::General{kind=candidate;inherited=true;break;}
  }
 }
 let serious=crate::emotion::classify(query).serious||kind==Task::Support||matches!(mood.map(|m|m.mood),Some(Mood::Concern));
 let no_humor=has(&w,&["без гумору","без сарказму","без жартів","no humor","no sarcasm","no jokes","без шуток"]);
 let playful=has(&w,&["пожартуй","жарт","мем","joke","funny","humor","пошути"]);
 let pref_no_humor=has(&prefs,&["без гумору","без сарказму","без жартів","no humor","no sarcasm","no jokes","без шуток"]);
 if kind==Task::Technical||kind==Task::Research||mode=="Developer"||mode=="Researcher"{t[0]=t[0].max(85);t[6]=t[6].min(25);t[3]=t[3].min(45);}
 if kind==Task::Support{t[1]=t[1].max(80);t[4]=t[4].min(45);}
 if serious||no_humor{t[3]=t[3].min(15);t[6]=0;if serious{t[1]=t[1].max(80);}}
 else if playful&&kind!=Task::Technical&&kind!=Task::Research{t[3]=t[3].saturating_add(10).min(100);}
 else if pref_no_humor{t[3]=0;t[6]=0;}
 let explicit=lengths(&w);let choice=if explicit==(false,false){lengths(&prefs)}else{explicit};
 let length=match choice{(true,false)=>"Concise",(false,true)=>"Detailed when useful",_=>"Match the complexity of the request"};
 Plan{traits:t,length,task:kind,inherited}
}
pub fn effective(base:[u8;7],query:&str,mood:Option<&State>,mode:&str)->([u8;7],&'static str){let p=plan(base,query,mood,mode,&Profile::default(),&[]);(p.traits,p.length)}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn serious(){let base=[90,80,90,90,90,80,70];let(t,_)=effective(base,"серйозно",None,"Companion");assert_eq!(t[6],0);assert_eq!(base[6],70);}
 #[test]fn technical_context(){assert_eq!(effective([90;7],"дякую, але не працює",None,"Companion").0[6],0);assert_eq!(effective([90;7],"errorless",None,"Companion").0[6],90);}
 #[test]fn length(){assert_eq!(effective([50;7],"коротко",None,"Companion").1,"Concise");}
 #[test]fn explicit_length_overrides_profile(){let p=Profile{communication:"коротко без жартів".into(),..Default::default()};let a=plan([90;7],"покроково поясни",None,"Companion",&p,&[]);assert_eq!(a.length,"Detailed when useful");assert_eq!(a.traits[6],0);assert_eq!(plan([90;7],"привіт",None,"Companion",&p,&[]).length,"Concise");assert_eq!(plan([90;7],"коротко і детально",None,"Companion",&p,&[]).length,"Match the complexity of the request");}
 #[test]fn continuation_uses_user_turns(){let h=vec![Message{role:"user".into(),content:"виправ код Rust".into()},Message{role:"assistant".into(),content:"story".into()},Message{role:"user".into(),content:"далі".into()}];let p=plan([50;7],"далі",None,"Companion",&Profile::default(),&h);assert_eq!(p.task,Task::Technical);assert!(p.inherited);assert_eq!(p.traits[0],85);assert_eq!(plan([50;7],"намалюй арт",None,"Companion",&Profile::default(),&h).task,Task::Creative);}
 #[test]fn serious_precedes_playful(){let p=plan([90;7],"мені сумно, пожартуй",None,"Companion",&Profile::default(),&[]);assert_eq!(p.task,Task::Support);assert_eq!(p.traits[6],0);}
 #[test]fn other_profile_fields_do_not_control_style(){let p=Profile{goals:"коротко".into(),interests:"без жартів".into(),..Default::default()};let a=plan([90;7],"hello",None,"Companion",&p,&[]);assert_eq!(a.traits,[90;7]);assert_eq!(a.length,"Match the complexity of the request");}
}
