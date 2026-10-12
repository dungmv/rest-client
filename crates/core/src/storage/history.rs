use crate::models::HttpResponseRecord;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct HistoryStorage {
    conn: Arc<Mutex<Connection>>,
}

impl HistoryStorage {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        storage.init_tables()?;
        Ok(storage)
    }

    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        storage.init_tables()?;
        Ok(storage)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS response_history (
                id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                status_text TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL,
                headers_json TEXT NOT NULL,
                cookies_json TEXT NOT NULL,
                body BLOB NOT NULL,
                timing_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_response_history_req ON response_history(request_id, created_at DESC);",
            [],
        )?;
        Ok(())
    }

    pub fn save_response(&self, record: &HttpResponseRecord, max_keep: usize) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let headers_json = serde_json::to_string(&record.headers)?;
        let cookies_json = serde_json::to_string(&record.cookies)?;
        let timing_json = serde_json::to_string(&record.timing)?;
        let created_at_str = record.created_at.to_rfc3339();

        conn.execute(
            "INSERT INTO response_history (
                id, request_id, status_code, status_text, duration_ms, size_bytes,
                headers_json, cookies_json, body, timing_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id,
                record.request_id,
                record.status_code,
                record.status_text,
                record.duration_ms,
                record.size_bytes,
                headers_json,
                cookies_json,
                record.body,
                timing_json,
                created_at_str,
            ],
        )?;

        // Prune older records beyond max_keep
        conn.execute(
            "DELETE FROM response_history 
             WHERE request_id = ?1 
               AND id NOT IN (
                   SELECT id FROM response_history 
                   WHERE request_id = ?1 
                   ORDER BY created_at DESC 
                   LIMIT ?2
               )",
            params![record.request_id, max_keep as i64],
        )?;

        Ok(())
    }

    pub fn get_responses(&self, request_id: &str, limit: usize) -> Result<Vec<HttpResponseRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, request_id, status_code, status_text, duration_ms, size_bytes,
                    headers_json, cookies_json, body, timing_json, created_at
             FROM response_history
             WHERE request_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![request_id, limit as i64], |row| {
            let id: String = row.get(0)?;
            let request_id: String = row.get(1)?;
            let status_code: u16 = row.get(2)?;
            let status_text: String = row.get(3)?;
            let duration_ms: u64 = row.get(4)?;
            let size_bytes: usize = row.get(5)?;
            let headers_json: String = row.get(6)?;
            let cookies_json: String = row.get(7)?;
            let body: Vec<u8> = row.get(8)?;
            let timing_json: String = row.get(9)?;
            let created_at_str: String = row.get(10)?;

            let headers = serde_json::from_str(&headers_json).unwrap_or_default();
            let cookies = serde_json::from_str(&cookies_json).unwrap_or_default();
            let timing = serde_json::from_str(&timing_json).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(HttpResponseRecord {
                id,
                request_id,
                status_code,
                status_text,
                duration_ms,
                size_bytes,
                headers,
                cookies,
                body,
                timing,
                created_at,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_latest_response(&self, request_id: &str) -> Result<Option<HttpResponseRecord>> {
        let list = self.get_responses(request_id, 1)?;
        Ok(list.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TimingBreakdown;

    #[test]
    fn test_history_save_and_prune() {
        let storage = HistoryStorage::new_in_memory().unwrap();
        let req_id = "req-1";

        for i in 1..=5 {
            let record = HttpResponseRecord {
                id: format!("resp-{}", i),
                request_id: req_id.to_string(),
                status_code: 200,
                status_text: "OK".to_string(),
                duration_ms: 10 * i as u64,
                size_bytes: 100 * i,
                headers: vec![("content-type".to_string(), "application/json".to_string())],
                cookies: vec![],
                body: format!("body {}", i).into_bytes(),
                timing: TimingBreakdown::default(),
                created_at: Utc::now(),
            };
            storage.save_response(&record, 3).unwrap();
        }

        let kept = storage.get_responses(req_id, 10).unwrap();
        assert_eq!(kept.len(), 3);
        assert_eq!(kept[0].id, "resp-5");
        assert_eq!(kept[1].id, "resp-4");
        assert_eq!(kept[2].id, "resp-3");
    }
}
