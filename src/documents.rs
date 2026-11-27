//! Local document extraction; attachment content is untrusted reference data.
use std::{io::{Cursor,Read},path::Path};
pub const MAX_CHARS:usize=200_000;
pub const CHUNK:usize=5_000;
const MAX_XML:u64=8*1024*1024;
#[derive(Clone)]
pub struct Section{pub name:String,pub number:usize,pub total:usize,pub text:String}
pub struct Extracted{pub sections:Vec<Section>,pub notes:Vec<String>}
fn normalize(text:&str)->String{
 text.chars().filter(|c|!c.is_control()||matches!(c,'\n'|'\t'|'\r'|'\u{c}')).collect()
}
fn extension(name:&str)->String{Path::new(name).extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase()}
pub fn supported(name:&str)->bool{
 matches!(extension(name).as_str(),"pdf"|"docx"|"txt"|"md"|"markdown"|"csv"|"tsv"|"json"|"jsonl"|"xml"|"yaml"|"yml"|"toml"|"ini"|"log"|"rs"|"py"|"js"|"ts"|"tsx"|"jsx"|"html"|"css"|"java"|"kt"|"swift"|"c"|"h"|"cpp"|"hpp"|"cs"|"gd"|"lua"|"sql"|"sh"|"ps1"|"bat")
}
fn plain(bytes:&[u8])->Result<String,String>{
 if bytes.starts_with(&[0xff,0xfe])||bytes.starts_with(&[0xfe,0xff]){
  if bytes.len()%2!=0{return Err("Invalid UTF-16".into());}
  let little=bytes[0]==0xff;
  let units:Vec<u16>=bytes[2..].chunks_exact(2).map(|b|if little{u16::from_le_bytes([b[0],b[1]])}else{u16::from_be_bytes([b[0],b[1]])}).collect();
  return String::from_utf16(&units).map_err(|_|"Invalid UTF-16".into());
 }
 let s=std::str::from_utf8(bytes.strip_prefix(&[0xef,0xbb,0xbf]).unwrap_or(bytes)).map_err(|_|"Only UTF-8 or BOM-marked UTF-16 text is supported")?;
 if s.contains('\0'){return Err("Binary text is not supported".into());}
 Ok(s.to_owned())
}
fn docx(bytes:&[u8])->Result<String,String>{
 use quick_xml::{Reader,events::Event};
 let mut archive=zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e|e.to_string())?;
 if archive.len()>4096{return Err("DOCX archive has too many entries".into());}
 let entry=archive.by_name("word/document.xml").map_err(|_|"DOCX main document missing")?;
 if entry.size()>MAX_XML{return Err("DOCX XML exceeds 8 MiB".into());}
 let mut xml=Vec::new();entry.take(MAX_XML+1).read_to_end(&mut xml).map_err(|e|e.to_string())?;
 if xml.len() as u64>MAX_XML{return Err("DOCX XML exceeds 8 MiB".into());}
 let xml=std::str::from_utf8(&xml).map_err(|_|"DOCX XML must be UTF-8")?;
 let mut reader=Reader::from_str(xml);let mut out=String::new();let mut inside_text=false;let mut depth=0usize;
 loop{
  match reader.read_event().map_err(|e|e.to_string())?{
   Event::Start(e)=>{depth+=1;if depth>128{return Err("DOCX XML nesting too deep".into());}match e.local_name().as_ref(){b"t"=>inside_text=true,b"tab"=>out.push('\t'),b"br"|b"cr"=>out.push('\n'),_=>{}}},
   Event::Empty(e)=>match e.local_name().as_ref(){b"tab"=>out.push('\t'),b"br"|b"cr"=>out.push('\n'),_=>{}},
   Event::End(e)=>{depth=depth.saturating_sub(1);match e.local_name().as_ref(){b"t"=>inside_text=false,b"p"|b"tr"=>out.push('\n'),b"tc"=>out.push('\t'),_=>{}}},
   Event::Text(t) if inside_text=>out.push_str(&t.unescape().map_err(|e|e.to_string())?),
   Event::DocType(_)=>return Err("DOCX DTD is not allowed".into()),
   Event::Eof=>break,
   _=>{}
  }
  if out.len()>MAX_XML as usize{return Err("DOCX extracted text exceeds limit".into());}
 }
 Ok(out)
}
fn pdf(bytes:&[u8],program:&str)->Result<String,String>{
 use std::{io::Write,process::{Command,Stdio},time::{Duration,Instant}};
 if !bytes.starts_with(b"%PDF-"){return Err("Invalid PDF header".into());}
 if program.trim().is_empty(){return Err("Configure a trusted local pdftotext executable for PDF extraction".into());}
 let exe=std::fs::canonicalize(crate::storage::resolve(program)).map_err(|_|"pdftotext executable not found")?;
 if !exe.is_file(){return Err("pdftotext must be a regular executable".into());}
 #[cfg(windows)]
 if !exe.extension().is_some_and(|e|e.eq_ignore_ascii_case("exe")){return Err("Choose pdftotext.exe".into());}
 let mut cmd=Command::new(exe);
 cmd.args(["-f","1","-l","50","-enc","UTF-8","-","-"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
 #[cfg(windows)] {use std::os::windows::process::CommandExt;cmd.creation_flags(0x08000000);}
 let mut child=cmd.spawn().map_err(|e|e.to_string())?;
 let (Some(mut input),Some(output))=(child.stdin.take(),child.stdout.take())else{let _=child.kill();let _=child.wait();return Err("PDF process pipes unavailable".into());};
 let data=bytes.to_vec();
 let writer=std::thread::spawn(move||input.write_all(&data));
 let reader=std::thread::spawn(move||{let mut out=Vec::new();output.take(1024*1024+1).read_to_end(&mut out).map(|_|out)});
 let start=Instant::now();
 let status=loop{match child.try_wait(){Ok(Some(status))=>break Ok(status),Ok(None) if start.elapsed()<Duration::from_secs(20)=>std::thread::sleep(Duration::from_millis(40)),Ok(None)=>{let _=child.kill();let _=child.wait();break Err("PDF extraction exceeded 20 seconds".to_string());},Err(e)=>{let _=child.kill();let _=child.wait();break Err(e.to_string());}}};
 let _=writer.join();let output=reader.join().map_err(|_|"PDF reader failed")?.map_err(|e|e.to_string())?;
 if !status?.success(){return Err("PDF extraction failed (encrypted, damaged or unsupported PDF)".into());}
 if output.len()>1024*1024{return Err("PDF output exceeds 1 MiB".into());}
 String::from_utf8(output).map_err(|_|"Invalid UTF-8 PDF output".into())
}
fn split(name:&str,text:&str)->Vec<Section>{
 let chars:Vec<char>=text.chars().take(MAX_CHARS).collect();
 let total=chars.len().div_ceil(CHUNK);
 chars.chunks(CHUNK).enumerate().map(|(i,c)|Section{name:name.to_owned(),number:i+1,total,text:c.iter().collect()}).collect()
}
pub fn extract(files:Vec<(String,Vec<u8>)>,pdftotext:&str)->Result<Extracted,String>{
 if files.is_empty()||files.len()>4{return Err("Select one to four supported documents".into());}
 let mut result=Extracted{sections:vec![],notes:vec![]};
 for (name,bytes) in files{
  if bytes.is_empty()||bytes.len()>8*1024*1024||!supported(&name){return Err(format!("{name}: unsupported file or size"));}
  let ext=extension(&name);
  let raw=match ext.as_str(){"pdf"=>pdf(&bytes,pdftotext),"docx"=>docx(&bytes),_=>plain(&bytes)}.map_err(|e|format!("{name}: {e}"))?;
  let text=normalize(&raw);
  if text.trim().is_empty(){return Err(format!("{name}: no extractable text; scanned PDFs need OCR (not implemented)"));}
  if ext=="pdf"{result.notes.push(format!("{name}: PDF extraction limited to the first 50 pages; no OCR."));}
  if ext=="docx"{result.notes.push(format!("{name}: main document text only; no embedded objects, headers or footnotes."));}
  if text.chars().count()>MAX_CHARS{result.notes.push(format!("{name}: text truncated at {MAX_CHARS} characters."));}
  result.sections.extend(split(&name,&text));
 }
 Ok(result)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn unicode_chunks(){let x=split("test.txt",&"ї".repeat(CHUNK+1));assert_eq!(x.len(),2);assert_eq!(x[0].text.chars().count(),CHUNK);assert_eq!(x[1].text,"ї");}
 #[test]fn encodings(){assert_eq!(plain(&[0xff,0xfe,0x16,0x04]).unwrap(),"Ж");assert_eq!(plain(b"\xef\xbb\xbfabc").unwrap(),"abc");assert!(plain(&[0xff]).is_err());}
 #[test]fn docx_text(){use std::io::Write;let mut zip=zip::ZipWriter::new(Cursor::new(Vec::new()));zip.start_file("word/document.xml",zip::write::SimpleFileOptions::default()).unwrap();zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>A &amp; B</w:t></w:r></w:p></w:body></w:document>"#).unwrap();let bytes=zip.finish().unwrap().into_inner();assert_eq!(docx(&bytes).unwrap(),"A & B\n");}
}
