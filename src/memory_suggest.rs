#[derive(Clone,Debug)]
pub struct Suggestion{pub content:String,pub category:&'static str,pub importance:u8}
pub fn suggest(input:&str)->Option<Suggestion>{
 let input=input.trim();if input.chars().count()>500{return None;}
 let lower=input.to_lowercase();
 let rules=[("мій проєкт ","Projects",90),("мій проект ","Projects",90),("наш проєкт ","Projects",90),("наш проект ","Projects",90),("я працюю над ","Projects",85),("my project ","Projects",90),("our project ","Projects",90),("i am working on ","Projects",85),("мені подобається ","Interests",70),("мені зручніше ","Preferences",80),("я надаю перевагу ","Preferences",80),("мне удобнее ","Preferences",80),("запам'ятай","General",85),("запам’ятай","General",85),("remember that","General",85),("я люблю ","Interests",70),("i like ","Interests",70),("моя ціль","Goals",85),("my goal","Goals",85),("я віддаю перевагу","Preferences",80),("i prefer ","Preferences",80)];
 for (prefix,category,importance) in rules{if lower.starts_with(prefix)&&input.chars().count()>prefix.chars().count()+2{return Some(Suggestion{content:input.into(),category,importance});}}None
}
pub fn duplicate(a:&str,b:&str)->bool{a.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()==b.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()}
#[cfg(test)]mod tests{use super::*;#[test]fn explicit(){assert_eq!(suggest("Я люблю гітару").unwrap().category,"Interests");assert!(suggest("Привіт").is_none());assert!(duplicate(" A  B ","a b"));}}

#[cfg(test)]#[test]fn project_and_preference_suggestions(){
 assert_eq!(suggest("Наш проєкт Hatoriel").unwrap().category,"Projects");
 assert_eq!(suggest("Мені зручніше короткі відповіді").unwrap().category,"Preferences");
 assert!(suggest("Чи подобається тобі Rust?").is_none());
}
