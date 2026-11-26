use std::collections::HashSet;
fn words(s:&str)->HashSet<String>{s.split(|c:char|!c.is_alphanumeric()).filter(|w|w.chars().count()>=3).map(str::to_lowercase).collect()}
pub fn relevant(query:&str,memories:&[String])->Vec<String>{
 let q=words(query);let mut ranked:Vec<_>=memories.iter().enumerate().map(|(i,m)|(words(m).intersection(&q).count(),i,m)).collect();
 ranked.sort_by(|a,b|b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
 ranked.into_iter().take(12).map(|(_,_,m)|m.clone()).collect()
}
#[cfg(test)] mod tests{use super::*;#[test]fn ranking(){let m=vec!["Люблю гітару".into(),"Маю кота".into()];assert_eq!(relevant("гітару",&m)[0],m[0]);}#[test]fn empty(){assert!(relevant("test",&[]).is_empty());}}
