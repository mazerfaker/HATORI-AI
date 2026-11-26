#[derive(Debug,PartialEq)]
pub struct Profile{pub context:u32,pub threads:u32,pub gpu_layers:u32}
pub fn recommend(ram:Option<u64>,cores:u32)->Profile{
 let context=match ram{Some(n) if n>=32=>8192,Some(n) if n>=16=>4096,Some(n) if n>=8=>2048,_=>1024};
 Profile{context,threads:(cores/2).clamp(1,16),gpu_layers:0}
}
pub fn warning(ram:Option<u64>,bytes:u64)->bool{
 ram.map(|n|bytes>n.saturating_sub(3).saturating_mul(1073741824)).unwrap_or(true)
}
#[cfg(test)]mod tests{use super::*;#[test]fn profiles(){assert_eq!(recommend(None,0).threads,1);assert_eq!(recommend(Some(16),8).context,4096);assert_eq!(recommend(Some(64),128).threads,16);assert!(warning(Some(4),2*1073741824));}}
