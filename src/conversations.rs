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
