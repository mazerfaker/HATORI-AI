#[derive(Clone,Debug)]
pub struct Suggestion{pub content:String,pub category:&'static str,pub importance:u8}
pub fn suggest(input:&str)->Option<Suggestion>{
 let input=input.trim();if input.chars().count()>500{return None;}
 let lower=input.to_lowercase();
 let rules=[("запам'ятай","General",85),("запам’ятай","General",85),("remember that","General",85),("я люблю ","Interests",70),("i like ","Interests",70),("моя ціль","Goals",85),("my goal","Goals",85),("я віддаю перевагу","Preferences",80),("i prefer ","Preferences",80)];
 for (prefix,category,importance) in rules{if lower.starts_with(prefix)&&input.chars().count()>prefix.chars().count()+2{return Some(Suggestion{content:input.into(),category,importance});}}None
}
pub fn duplicate(a:&str,b:&str)->bool{a.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()==b.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()}
#[cfg(test)]mod tests{use super::*;#[test]fn explicit(){assert_eq!(suggest("Я люблю гітару").unwrap().category,"Interests");assert!(suggest("Привіт").is_none());assert!(duplicate(" A  B ","a b"));}}
