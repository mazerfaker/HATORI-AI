//! Transient conversational hints; never learn hidden preferences or copy old output into instructions.
use crate::inference::Message;
fn normalize(s:&str)->String{s.chars().take(200).collect::<String>().to_lowercase().split(|c:char|!c.is_alphanumeric()).filter(|w|!w.is_empty()).collect::<Vec<_>>().join(" ")}
fn starts(s:&str,patterns:&[&str])->bool{patterns.iter().any(|p|s==*p||s.strip_prefix(p).is_some_and(|tail|tail.starts_with(' ')))}
pub fn topic_change(query:&str)->bool{starts(&normalize(query),&["інша тема","змінимо тему","нове питання","до речі","другая тема","new topic","different question","by the way"])}
pub fn hint(query:&str,history:&[Message])->String{
 let q=normalize(query);
 let mut hints=vec!["Use natural wording, not a fixed reply template. Avoid routine greetings, self-introductions, unnecessary compliments, repeated summaries and generic closing questions. Match the user's register without mimicking mistakes or abuse."];
 if topic_change(query){hints.push("The user signals a topic change; follow the new topic and do not force the old subject into the answer.");}
 else if crate::memory_core::continuation_query(query,history)!=query.chars().take(1500).collect::<String>()&&!q.is_empty(){hints.push("This is a continuation; answer the next substantive step instead of restarting the explanation. Do not assume permission to act.");}
 if matches!(q.as_str(),"дякую"|"спасибі"|"спасибо"|"thanks"|"thank you"|"супер"|"чудово"|"great"|"awesome"){hints.push("A simple acknowledgment is enough unless the user asks another question.");}
 if starts(&q,&["не так","я мав на увазі","я мала на увазі","не це","i meant","not that","я имел в виду"]){hints.push("Treat this as a correction. Address the changed requirement directly, without defensiveness or repeating the rejected answer.");}
 let replies:Vec<_>=history.iter().rev().take(16).filter(|m|m.role=="assistant").take(4).collect();
 let opening=|s:&str|normalize(s).split_whitespace().take(3).collect::<Vec<_>>().join(" ");
 let openings:Vec<_>=replies.iter().map(|m|opening(&m.content)).collect();
 if openings.iter().enumerate().any(|(i,a)|a.split_whitespace().count()==3&&openings.iter().skip(i+1).any(|b|a==b)){hints.push("Recent replies have repeated openings. Start with the actual answer using different wording; do not repeat the stock preamble.");}
 hints.join(" ")
}
#[cfg(test)]mod tests{
 use super::*;
 fn m(role:&str,content:&str)->Message{Message{role:role.into(),content:content.into()}}
 #[test]fn no_raw_history_in_hint(){let h=vec![m("assistant","private secret phrase one"),m("assistant","private secret phrase two")];let s=hint("hello",&h);assert!(s.contains("repeated openings"));assert!(!s.contains("private secret"));}
 #[test]fn topic_and_correction(){assert!(hint("До речі, гітара",&[]).contains("topic change"));assert!(hint("Я мав на увазі інше",&[]).contains("correction"));assert!(hint("Дякую!",&[]).contains("acknowledgment"));}
 #[test]fn continuation(){let h=vec![m("user","Rust project"),m("user","далі")];assert!(hint("далі",&h).contains("next substantive step"));}
}
