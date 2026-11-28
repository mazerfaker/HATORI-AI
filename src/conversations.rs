use rusqlite::{Connection,Result,params};
use hatori_ai::inference::Message;
pub fn migrate(d:&Connection)->Result<()>{
 d.execute_batch("BEGIN; CREATE TABLE IF NOT EXISTS chats(id INTEGER PRIMARY KEY AUTOINCREMENT,title TEXT NOT NULL); CREATE TABLE IF NOT EXISTS chat_messages(id INTEGER PRIMARY KEY,chat_id INTEGER NOT NULL,role TEXT NOT NULL,content TEXT NOT NULL); CREATE TABLE IF NOT EXISTS chat_migration(done INTEGER PRIMARY KEY); INSERT INTO chats(id,title) SELECT 1,'Previous conversation' WHERE NOT EXISTS(SELECT 1 FROM chat_migration); INSERT INTO chat_messages(chat_id,role,content) SELECT 1,role,content FROM messages WHERE NOT EXISTS(SELECT 1 FROM chat_migration) ORDER BY id; INSERT OR IGNORE INTO chat_migration VALUES(1); COMMIT;").map_err(|e|{let _=d.execute_batch("ROLLBACK;");e})
}
pub fn list(d:&Connection)->Result<Vec<(i64,String)>>{let mut q=d.prepare("SELECT id,title FROM chats ORDER BY id DESC")?;let rows=q.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?;rows.collect()}
pub fn load(d:&Connection,id:i64)->Result<Vec<Message>>{let mut q=d.prepare("SELECT role,content FROM chat_messages WHERE chat_id=?1 ORDER BY id")?;let rows=q.query_map([id],|r|Ok(Message{role:r.get(0)?,content:r.get(1)?}))?;rows.collect()}
pub fn create(d:&Connection)->Result<i64>{d.execute("INSERT INTO chats(title) VALUES('New chat')",[])?;Ok(d.last_insert_rowid())}
pub fn rename(d:&Connection,id:i64,title:&str)->Result<()>{d.execute("UPDATE chats SET title=?1 WHERE id=?2",params![title,id])?;Ok(())}
pub fn append(d:&Connection,id:i64,role:&str,content:&str)->Result<()>{d.execute("INSERT INTO chat_messages(chat_id,role,content) VALUES(?1,?2,?3)",params![id,role,content])?;Ok(())}
pub fn remove(d:&mut Connection,id:i64)->Result<()>{let tx=d.transaction()?;tx.execute("DELETE FROM chat_messages WHERE chat_id=?1",[id])?;tx.execute("DELETE FROM chats WHERE id=?1",[id])?;tx.commit()}
#[cfg(test)]mod tests{use super::*;#[test]fn migration_and_isolation(){let d=Connection::open_in_memory().unwrap();d.execute_batch("CREATE TABLE messages(id INTEGER PRIMARY KEY,role TEXT,content TEXT);INSERT INTO messages VALUES(1,'user','old');").unwrap();migrate(&d).unwrap();migrate(&d).unwrap();let id=create(&d).unwrap();append(&d,id,"user","new").unwrap();assert_eq!(load(&d,1).unwrap().len(),1);assert_eq!(load(&d,id).unwrap()[0].content,"new");}}

#[derive(Clone,Debug)]
pub struct SearchHit{pub chat_id:i64,pub title:String,pub excerpt:String}
/// Local manual search over the latest 2000 messages; no model or network access.
pub fn search(d:&Connection,query:&str)->Result<Vec<SearchHit>>{
 let needle=query.trim().chars().take(200).collect::<String>().to_lowercase();
 if needle.chars().count()<2{return Ok(vec![]);}
 let mut q=d.prepare("SELECT c.id,c.title,substr(m.content,1,16000) FROM chat_messages m JOIN chats c ON c.id=m.chat_id ORDER BY m.id DESC LIMIT 2000")?;
 let rows=q.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?;
 let mut out=Vec::new();
 for row in rows{let(chat_id,title,content)=row?;if content.to_lowercase().contains(&needle)||title.to_lowercase().contains(&needle){
 let excerpt=content.chars().take(240).collect();out.push(SearchHit{chat_id,title,excerpt});if out.len()==20{break;}
 }}
 Ok(out)
}
#[cfg(test)]#[test]fn archive_search_and_delete(){
 let mut d=Connection::open_in_memory().unwrap();d.execute_batch("CREATE TABLE messages(id INTEGER PRIMARY KEY,role TEXT,content TEXT);").unwrap();migrate(&d).unwrap();
 let id=create(&d).unwrap();append(&d,id,"user","Проєкт Hatoriel").unwrap();
 assert_eq!(search(&d,"ПРОЄКТ").unwrap()[0].chat_id,id);assert!(search(&d,"").unwrap().is_empty());remove(&mut d,id).unwrap();assert!(search(&d,"Hatoriel").unwrap().is_empty());
}
