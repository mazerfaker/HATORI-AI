use std::time::Duration;
use serde_json::Value;
pub fn cosine(a:&[f64],b:&[f64])->Option<f64>{
 if a.is_empty()||a.len()!=b.len(){return None;}
 let (mut dot,mut aa,mut bb)=(0.,0.,0.);
 for (&x,&y) in a.iter().zip(b){if !x.is_finite()||!y.is_finite(){return None;}dot+=x*y;aa+=x*x;bb+=y*y;}
 if aa<=0.||bb<=0.{return None;}let s=dot/(aa.sqrt()*bb.sqrt());if s.is_finite(){Some(s.clamp(-1.,1.))}else{None}
}
pub fn retrieve(query:&str,memories:&[(String,u8,i64)],model:&str)->Result<Vec<String>,String>{
 if memories.is_empty(){return Ok(vec![]);}
 let candidates=select_candidates(memories);
 let mut inputs=vec![query.chars().take(1500).collect::<String>()];inputs.extend(candidates.iter().map(|m|m.0.chars().take(1500).collect::<String>()));
 let client=reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(2)).timeout(Duration::from_secs(20)).build().map_err(|e|e.to_string())?;
 let v:Value=client.post("http://127.0.0.1:11434/api/embed").json(&serde_json::json!({"model":model,"input":inputs})).send().and_then(|r|r.error_for_status()).and_then(|r|r.json()).map_err(|e|e.to_string())?;
 let vectors:Vec<Vec<f64>>=serde_json::from_value(v["embeddings"].clone()).map_err(|e|e.to_string())?;
 if vectors.len()!=candidates.len()+1{return Err("Embedding count mismatch".into());}
 let mut hits:Vec<_>=candidates.iter().zip(vectors.iter().skip(1)).filter_map(|(m,v)|cosine(&vectors[0],v).filter(|s|*s>=0.55).map(|s|(s+f64::from(m.1)*0.0005,m))).collect();
 hits.sort_by(|a,b|b.0.total_cmp(&a.0).then(b.1.2.cmp(&a.1.2)));
 Ok(hits.into_iter().take(12).map(|(_,m)|m.0.clone()).collect())
}
#[cfg(test)]mod tests{use super::*;#[test]fn similarity(){assert_eq!(cosine(&[1.,0.],&[1.,0.]),Some(1.));assert_eq!(cosine(&[1.,0.],&[0.,1.]),Some(0.));assert_eq!(cosine(&[0.],&[1.]),None);assert_eq!(cosine(&[1.],&[1.,2.]),None);}}

/// Keep both recent and important records in the bounded local embedding batch.
fn select_candidates(memories:&[(String,u8,i64)])->Vec<&(String,u8,i64)>{
 let mut recent:Vec<_>=memories.iter().enumerate().collect();recent.sort_by(|a,b|b.1.2.cmp(&a.1.2));
 let mut important=recent.clone();important.sort_by(|a,b|b.1.1.cmp(&a.1.1).then(b.1.2.cmp(&a.1.2)));
 let mut seen=std::collections::HashSet::new();let mut out=Vec::new();
 for (i,m) in recent.iter().take(32).chain(important.iter()).chain(recent.iter()){
  if seen.insert(*i){out.push(*m);if out.len()==64{break;}}
 }out
}
#[cfg(test)]#[test]fn old_important_record_survives(){let mut m:Vec<_>=(0..100).map(|i|(format!("record {i}"),1,i)).collect();m[0].1=100;let c=select_candidates(&m);assert_eq!(c.len(),64);assert!(c.iter().any(|r|r.2==0));}
