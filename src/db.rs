use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub struct PriceDb {
    conn: Connection,
    _db_path: PathBuf,
}

impl PriceDb {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = db_path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = PriceDb {
            conn,
            _db_path: db_path,
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error>> {
        let version: Option<i32> = self
            .conn
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .ok();

        match version {
            None => {
                self.conn.execute_batch(
                    "
                    CREATE TABLE schema_version (version INTEGER NOT NULL);
                    INSERT INTO schema_version VALUES (1);

                    CREATE TABLE IF NOT EXISTS ohlcv (
                        symbol      TEXT NOT NULL,
                        ts          INTEGER NOT NULL,
                        interval_t  TEXT NOT NULL,
                        open        REAL,
                        high        REAL,
                        low         REAL,
                        close       REAL NOT NULL,
                        volume      REAL,
                        PRIMARY KEY (symbol, interval_t, ts)
                    );

                    CREATE TABLE IF NOT EXISTS indicator_results (
                        symbol       TEXT NOT NULL,
                        indicator_t  TEXT NOT NULL,
                        config_json  TEXT NOT NULL,
                        computed_at  INTEGER NOT NULL,
                        value        REAL,
                        secondary    REAL,
                        triggered    INTEGER NOT NULL DEFAULT 0,
                        PRIMARY KEY (symbol, indicator_t, config_json, computed_at)
                    );
                    ",
                )?;
            }
            Some(1) => {}
            Some(v) => {
                eprintln!("Warning: unknown schema version {v}, proceeding");
            }
        }
        Ok(())
    }

    pub fn insert_ohlcv(
        &self,
        symbol: &str,
        interval: &str,
        rows: &[crate::data::OhlcvRow],
    ) -> Result<(), rusqlite::Error> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO ohlcv (symbol, ts, interval_t, open, high, low, close, volume)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for row in rows {
                stmt.execute(params![
                    symbol, row.ts, interval, row.open, row.high, row.low, row.close, row.volume,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn latest_timestamp(
        &self,
        symbol: &str,
        interval: &str,
    ) -> Result<Option<i64>, rusqlite::Error> {
        let result: Option<i64> = self.conn.query_row(
            "SELECT MAX(ts) FROM ohlcv WHERE symbol = ?1 AND interval_t = ?2",
            params![symbol, interval],
            |row| row.get::<_, Option<i64>>(0),
        )?;
        Ok(result)
    }

    pub fn load_close_prices(
        &self,
        symbol: &str,
        interval: &str,
    ) -> Result<Vec<f64>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT close FROM ohlcv WHERE symbol = ?1 AND interval_t = ?2 ORDER BY ts ASC",
        )?;
        let prices = stmt
            .query_map(params![symbol, interval], |row| row.get::<_, f64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(prices)
    }

    pub fn count_ohlcv(&self, symbol: &str, interval: &str) -> Result<i64, rusqlite::Error> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM ohlcv WHERE symbol = ?1 AND interval_t = ?2",
            params![symbol, interval],
            |row| row.get(0),
        )
    }

    pub fn store_indicator_result(
        &self,
        symbol: &str,
        indicator_type: &str,
        config_json: &str,
        computed_at: i64,
        value: Option<f64>,
        secondary: Option<f64>,
        triggered: bool,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO indicator_results
             (symbol, indicator_t, config_json, computed_at, value, secondary, triggered)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                symbol,
                indicator_type,
                config_json,
                computed_at,
                value,
                secondary,
                triggered as i32,
            ],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn last_indicator_result(
        &self,
        symbol: &str,
        indicator_type: &str,
        config_json: &str,
    ) -> Result<Option<(Option<f64>, Option<f64>, bool)>, rusqlite::Error> {
        let result = self.conn.query_row(
            "SELECT value, secondary, triggered FROM indicator_results
             WHERE symbol = ?1 AND indicator_t = ?2 AND config_json = ?3
             ORDER BY computed_at DESC LIMIT 1",
            params![symbol, indicator_type, config_json],
            |row| {
                let value: Option<f64> = row.get(0)?;
                let secondary: Option<f64> = row.get(1)?;
                let triggered: i32 = row.get(2)?;
                Ok((value, secondary, triggered != 0))
            },
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn purge_old_data(&self, retain_count: i64) -> Result<(), rusqlite::Error> {
        if retain_count <= 0 {
            return Ok(());
        }
        let mut stmt = self.conn.prepare(
            "DELETE FROM ohlcv WHERE rowid IN (
                SELECT o1.rowid FROM ohlcv o1
                WHERE (
                    SELECT COUNT(*) FROM ohlcv o2
                    WHERE o2.symbol = o1.symbol
                      AND o2.interval_t = o1.interval_t
                      AND o2.ts > o1.ts
                ) >= ?1
            )",
        )?;
        stmt.execute(params![retain_count])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::OhlcvRow;

    fn test_db() -> PriceDb {
        PriceDb::new(":memory:").expect("Failed to create in-memory DB")
    }

    #[test]
    fn test_schema_creation() {
        let db = test_db();
        let count: i32 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3); // schema_version, ohlcv, indicator_results
    }

    #[test]
    fn test_insert_and_query_ohlcv() {
        let db = test_db();
        let rows = vec![
            OhlcvRow {
                ts: 100,
                open: Some(10.0),
                high: Some(11.0),
                low: Some(9.0),
                close: 10.5,
                volume: Some(1000.0),
            },
            OhlcvRow {
                ts: 200,
                open: Some(10.5),
                high: Some(12.0),
                low: Some(10.0),
                close: 11.0,
                volume: Some(1500.0),
            },
            OhlcvRow {
                ts: 300,
                open: Some(11.0),
                high: Some(13.0),
                low: Some(10.5),
                close: 12.0,
                volume: Some(2000.0),
            },
        ];

        db.insert_ohlcv("TEST", "1d", &rows).unwrap();
        assert_eq!(db.count_ohlcv("TEST", "1d").unwrap(), 3);

        let prices = db.load_close_prices("TEST", "1d").unwrap();
        assert_eq!(prices, vec![10.5, 11.0, 12.0]);
    }

    #[test]
    fn test_latest_timestamp() {
        let db = test_db();
        assert!(db.latest_timestamp("TEST", "1d").unwrap().is_none());

        let rows = vec![
            OhlcvRow {
                ts: 100,
                open: None,
                high: None,
                low: None,
                close: 10.0,
                volume: None,
            },
            OhlcvRow {
                ts: 200,
                open: None,
                high: None,
                low: None,
                close: 11.0,
                volume: None,
            },
        ];
        db.insert_ohlcv("TEST", "1d", &rows).unwrap();
        assert_eq!(db.latest_timestamp("TEST", "1d").unwrap(), Some(200));
    }

    #[test]
    fn test_upsert_replaces_duplicate() {
        let db = test_db();
        let row = OhlcvRow {
            ts: 100,
            open: Some(10.0),
            high: Some(11.0),
            low: Some(9.0),
            close: 10.5,
            volume: Some(1000.0),
        };
        db.insert_ohlcv("TEST", "1d", &[row]).unwrap();

        let updated = OhlcvRow {
            ts: 100,
            open: Some(11.0),
            high: Some(12.0),
            low: Some(10.0),
            close: 11.5,
            volume: Some(2000.0),
        };
        db.insert_ohlcv("TEST", "1d", &[updated]).unwrap();

        assert_eq!(db.count_ohlcv("TEST", "1d").unwrap(), 1);
        let prices = db.load_close_prices("TEST", "1d").unwrap();
        assert_eq!(prices, vec![11.5]);
    }

    #[test]
    fn test_purge_keeps_correct_count() {
        let db = test_db();
        let mut rows = Vec::new();
        for i in 0..100i64 {
            rows.push(OhlcvRow {
                ts: i * 10,
                open: None,
                high: None,
                low: None,
                close: i as f64,
                volume: None,
            });
        }
        db.insert_ohlcv("TEST", "1d", &rows).unwrap();
        assert_eq!(db.count_ohlcv("TEST", "1d").unwrap(), 100);

        db.purge_old_data(10).unwrap();
        assert_eq!(db.count_ohlcv("TEST", "1d").unwrap(), 10);

        let prices = db.load_close_prices("TEST", "1d").unwrap();
        assert_eq!(
            prices,
            vec![90.0, 91.0, 92.0, 93.0, 94.0, 95.0, 96.0, 97.0, 98.0, 99.0]
        );
    }

    #[test]
    fn test_purge_with_zero_retain() {
        let db = test_db();
        let rows = vec![OhlcvRow {
            ts: 100,
            open: None,
            high: None,
            low: None,
            close: 10.0,
            volume: None,
        }];
        db.insert_ohlcv("TEST", "1d", &rows).unwrap();
        db.purge_old_data(0).unwrap();
        assert_eq!(db.count_ohlcv("TEST", "1d").unwrap(), 1);
    }

    #[test]
    fn test_purge_respects_symbol_separation() {
        let db = test_db();
        for sym in &["A", "B"] {
            let rows = (0..20i64)
                .map(|i| OhlcvRow {
                    ts: i * 10,
                    open: None,
                    high: None,
                    low: None,
                    close: i as f64,
                    volume: None,
                })
                .collect::<Vec<_>>();
            db.insert_ohlcv(sym, "1d", &rows).unwrap();
        }

        db.purge_old_data(5).unwrap();
        assert_eq!(db.count_ohlcv("A", "1d").unwrap(), 5);
        assert_eq!(db.count_ohlcv("B", "1d").unwrap(), 5);
    }

    #[test]
    fn test_store_and_query_indicator_result() {
        let db = test_db();
        let cfg = r#"{"type":"rsi","threshold":35.0,"period":14}"#;

        db.store_indicator_result("TEST", "rsi", cfg, 100, Some(30.5), None, true)
            .unwrap();

        let result = db.last_indicator_result("TEST", "rsi", cfg).unwrap();
        assert!(result.is_some());
        let (val, sec, trig) = result.unwrap();
        assert!((val.unwrap() - 30.5).abs() < 0.001);
        assert!(sec.is_none());
        assert!(trig);
    }

    #[test]
    fn test_last_indicator_result_returns_most_recent() {
        let db = test_db();
        let cfg = r#"{"type":"rsi","threshold":35.0,"period":14}"#;

        db.store_indicator_result("TEST", "rsi", cfg, 100, Some(40.0), None, false)
            .unwrap();
        db.store_indicator_result("TEST", "rsi", cfg, 200, Some(30.0), None, true)
            .unwrap();

        let result = db.last_indicator_result("TEST", "rsi", cfg).unwrap();
        let (val, _, trig) = result.unwrap();
        assert!((val.unwrap() - 30.0).abs() < 0.001);
        assert!(trig);
    }

    #[test]
    fn test_last_indicator_result_no_data() {
        let db = test_db();
        let result = db.last_indicator_result("NONE", "rsi", "{}").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_ohlcv_separate_intervals() {
        let db = test_db();
        let rows_d = vec![OhlcvRow {
            ts: 100,
            open: None,
            high: None,
            low: None,
            close: 1.0,
            volume: None,
        }];
        let rows_w = vec![OhlcvRow {
            ts: 100,
            open: None,
            high: None,
            low: None,
            close: 100.0,
            volume: None,
        }];

        db.insert_ohlcv("TEST", "1d", &rows_d).unwrap();
        db.insert_ohlcv("TEST", "1wk", &rows_w).unwrap();

        assert_eq!(db.count_ohlcv("TEST", "1d").unwrap(), 1);
        assert_eq!(db.count_ohlcv("TEST", "1wk").unwrap(), 1);
    }
}
