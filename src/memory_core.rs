use std::collections::HashSet;
fn words(s:&str)->HashSet<String>{s.split(|c:char|!c.is_alphanumeric()).filter(|w|w.chars().count()>=3).map(str::to_lowercase).collect()}
pub fn relevant(query:&str,memories:&[String])->Vec<String>{
 let q=words(query);let mut ranked:Vec<_>=memories.iter().enumerate().map(|(i,m)|(words(m).intersection(&q).count(),i,m)).collect();
 ranked.sort_by(|a,b|b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
 ranked.into_iter().take(12).map(|(_,_,m)|m.clone()).collect()
}
#[cfg(test)] mod tests{use super::*;#[test]fn ranking(){let m=vec!["Люблю гітару".into(),"Маю кота".into()];assert_eq!(relevant("гітару",&m)[0],m[0]);}#[test]fn empty(){assert!(relevant("test",&[]).is_empty());}}

pub const CATEGORIES:[&str;8]=["General","Interests","Preferences","Skills","Goals","Projects","Relationships","Other"];
pub fn ranked(query:&str,memories:&[(String,u8,i64)])->Vec<String>{
 let q=words(query);
 let mut hits:Vec<_>=memories.iter().filter_map(|(content,importance,id)|{
 let overlap=words(content).intersection(&q).count();
 if overlap==0{return None;}
 Some((overlap,*importance,*id,content))
 }).collect();
 hits.sort_by(|a,b|b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(b.2.cmp(&a.2)));
 hits.into_iter().take(12).map(|(_,_,_,s)|s.clone()).collect()
}
#[cfg(test)]#[test]fn priority_and_relevance(){let m=vec![("guitar rock".into(),20,1),("guitar jazz".into(),90,2),("unrelated".into(),100,3)];assert_eq!(ranked("guitar",&m),vec!["guitar jazz","guitar rock"]);assert!(ranked("unknown",&m).is_empty());}
