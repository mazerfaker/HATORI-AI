use crate::emotion::{Mood,State};
pub fn effective(base:[u8;7],query:&str,mood:Option<&State>,mode:&str)->([u8;7],&'static str){
 let mut t=base.map(|v|v.min(100));let s=query.to_lowercase();
 let serious=crate::emotion::classify(query).serious||matches!(mood.map(|m|m.mood),Some(Mood::Concern));
 let playful=["пожартуй","жарт","мем","joke","funny"].iter().any(|w|s.contains(*w));
 if serious{t[3]=t[3].min(15);t[6]=0;t[1]=t[1].max(80);}else if playful{t[3]=t[3].saturating_add(10).min(100);}
 if mode=="Developer"||mode=="Researcher"{t[0]=t[0].max(85);t[6]=t[6].min(25);}
 let length=if ["детально","докладно","step by step","in detail"].iter().any(|w|s.contains(*w)){"Detailed when useful"}else if ["коротко","стисло","briefly","concise"].iter().any(|w|s.contains(*w)){"Concise"}else{"Match the complexity of the request"};
 (t,length)
}
#[cfg(test)]mod tests{use super::*;#[test]fn serious(){let base=[90,80,90,90,90,80,70];let(t,_)=effective(base,"серйозно",None,"Companion");assert_eq!(t[6],0);assert_eq!(base[6],70);}#[test]fn technical_context(){assert_eq!(effective([90;7],"дякую, але не працює",None,"Companion").0[6],0);assert_eq!(effective([90;7],"errorless",None,"Companion").0[6],90);}#[test]fn length(){assert_eq!(effective([50;7],"коротко",None,"Companion").1,"Concise");}}
