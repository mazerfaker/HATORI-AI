use rusqlite::{Connection,params,Result};
#[derive(Clone,Debug)]
pub struct Memory{pub id:i64,pub content:String,pub category:String,pub importance:u8,pub created:i64}
pub fn migrate(db:&Connection)->Result<()>{
 db.execute_batch("CREATE TABLE IF NOT EXISTS memory_metadata(memory_id INTEGER PRIMARY KEY, category TEXT NOT NULL DEFAULT 'General', importance INTEGER NOT NULL DEFAULT 50, created INTEGER NOT NULL DEFAULT 0); INSERT OR IGNORE INTO memory_metadata(memory_id) SELECT id FROM memories;")
}
pub fn load(db:&Connection)->Result<Vec<Memory>>{
 let mut q=db.prepare("SELECT m.id,m.content,coalesce(x.category,'General'),coalesce(x.importance,50),coalesce(x.created,0) FROM memories m LEFT JOIN memory_metadata x ON x.memory_id=m.id ORDER BY m.id")?;
 let rows=q.query_map([],|r|Ok(Memory{id:r.get(0)?,content:r.get(1)?,category:r.get(2)?,importance:r.get(3)?,created:r.get(4)?}))?;rows.collect()
}
pub fn add(db:&mut Connection,content:&str,category:&str,importance:u8)->Result<Memory>{
 let tx=db.transaction()?;tx.execute("INSERT INTO memories(content) VALUES(?1)",[content])?;let id=tx.last_insert_rowid();let created=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
 tx.execute("INSERT INTO memory_metadata VALUES(?1,?2,?3,?4)",params![id,category,importance.min(100),created])?;tx.commit()?;Ok(Memory{id,content:content.into(),category:category.into(),importance:importance.min(100),created})
}
pub fn update(db:&mut Connection,m:&Memory)->Result<()>{
 let tx=db.transaction()?;tx.execute("UPDATE memories SET content=?1 WHERE id=?2",params![m.content,m.id])?;tx.execute("INSERT OR REPLACE INTO memory_metadata VALUES(?1,?2,?3,?4)",params![m.id,m.category,m.importance.min(100),m.created])?;tx.commit()
}
pub fn remove(db:&mut Connection,id:i64)->Result<()>{
 let tx=db.transaction()?;tx.execute("DELETE FROM memory_metadata WHERE memory_id=?1",[id])?;tx.execute("DELETE FROM memories WHERE id=?1",[id])?;tx.commit()
}
#[cfg(test)]mod tests{use super::*;#[test]fn legacy_and_duplicates(){let mut db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE memories(id INTEGER PRIMARY KEY,content TEXT);INSERT INTO memories(content) VALUES('same');").unwrap();migrate(&db).unwrap();migrate(&db).unwrap();let mut m=add(&mut db,"same","Goals",90).unwrap();m.content="edited".into();update(&mut db,&m).unwrap();remove(&mut db,m.id).unwrap();let rows=load(&db).unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].content,"same");assert_eq!(rows[0].created,0);}}
