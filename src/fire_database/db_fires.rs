/*! Methods and types to support querying the fires table of the database. */

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::error::SatFireError;
use chrono::NaiveDateTime;
use geo::{point, Point, Polygon};
use rusqlite::{Connection, ToSql};

impl super::FiresDatabase {
    pub fn next_new_fire_id_state(&self) -> Result<FireDataNextNewFireState, Box<dyn Error>> {
        let next_id_num: i64 = match self.db.query_row(
            "SELECT item_value FROM meta WHERE item_name = 'next fire num'",
            [],
            |row| row.get(0),
        ) {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(1),
            other => other,
        }?;

        assert!(next_id_num > 0);
        Ok(FireDataNextNewFireState {
            conn: &self.db,
            next_id_num: next_id_num as u32,
        })
    }

    pub fn add_fire_handle(&self) -> Result<AddFireTransaction, Box<dyn Error>> {
        Ok(AddFireTransaction {
            db: &self.db,
            buffer: Vec::with_capacity(BUFFER_CAPACITY),
        })
    }
}

pub struct FireQuery<'a>(rusqlite::Statement<'a>);

impl<'a> FireQuery<'a> {
    pub fn records_for(
        &mut self,
        satellite: &str,
    ) -> Result<impl Iterator<Item = FireRecord> + '_, Box<dyn Error>> {
        let rows = self
            .0
            .query_and_then(&[satellite], |row| {
                let id: FireCode = FireCode(row.get(0)?);
                let last_observed: NaiveDateTime =
                    chrono::NaiveDateTime::from_timestamp(row.get::<_, i64>(1)?, 0);
                let lat: f64 = row.get(2)?;
                let lon: f64 = row.get(3)?;
                let origin = point!(x: lat, y: lon);

                let pblob = row.get_ref(4)?.as_blob()?;

                let perimeter: Polygon<f64> =
                    bincode::deserialize(&pblob).map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(FireRecord {
                    id,
                    last_observed,
                    origin,
                    perimeter,
                })
            })?
            .filter_map(|res: Result<_, rusqlite::Error>| res.ok());

        Ok(rows)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FireCode(String);

impl FireCode {
    pub fn make_child_fire(&self, child_num: u32) -> FireCode {
        assert!(child_num < 100);

        FireCode(format!("{}{:02}", self.0, child_num))
    }

    pub fn num_generations(&self) -> usize {
        (self.0.len() - 6) / 2 + 1
    }
}

impl Display for FireCode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

pub struct FireDataNextNewFireState<'a> {
    next_id_num: u32,
    conn: &'a Connection,
}

impl<'a> Drop for FireDataNextNewFireState<'a> {
    fn drop(&mut self) {
        self.conn
            .execute(
                "INSERT OR UPDATE INTO meta (item_name, item_value) VALUES (?, ?)",
                [&"next fire num" as &dyn ToSql, &self.next_id_num],
            )
            .unwrap();
    }
}

impl<'a> FireDataNextNewFireState<'a> {
    pub fn get_next_fire_id(&mut self) -> Result<FireCode, SatFireError> {
        let val_to_return = self.next_id_num;

        self.next_id_num += 1;

        if val_to_return <= 999_999 {
            Ok(FireCode(format!("{:06}", val_to_return)))
        } else {
            Err(SatFireError {
                msg: "Too many fires for satfire",
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct FireRecord {
    /// Row id from the database.
    pub id: FireCode,
    /// The mid-point time of the scan this cluster was detected in.
    pub last_observed: NaiveDateTime,
    /// Perimeter
    pub perimeter: Polygon<f64>,
    /// Point of origin (pixel first detected.
    pub origin: Point<f64>,
}

pub struct AddFireTransaction<'a> {
    buffer: Vec<(
        String,
        &'static str,
        NaiveDateTime,
        Point<f64>,
        Polygon<f64>,
    )>,
    db: &'a rusqlite::Connection,
}

const BUFFER_CAPACITY: usize = 10_000;

impl<'a> AddFireTransaction<'a> {
    pub fn add_fire<S: Into<String>>(
        &mut self,
        fire_id: &str,
        satellite: &'static str,
        last_observed: NaiveDateTime,
        origin: Point<f64>,
        perimeter: Polygon<f64>,
    ) -> Result<(), Box<dyn Error>> {
        let fire_id: String = fire_id.into();

        self.buffer
            .push((fire_id, satellite, last_observed, origin, perimeter));

        if self.buffer.len() >= BUFFER_CAPACITY {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        self.db.execute_batch("BEGIN;")?;
        let mut stmt = self.db.prepare(include_str!("add_fire.sql"))?;

        for (fire_id, satellite, last_observed, origin, perimeter) in self.buffer.drain(..) {
            let lat = origin.x();
            let lon = origin.y();

            let perimeter = bincode::serialize(&perimeter)?;
            let _ = stmt.execute([
                &fire_id as &dyn ToSql,
                &satellite,
                &last_observed.timestamp(),
                &lat,
                &lon,
                &perimeter,
            ])?;
        }

        self.db.execute_batch("COMMIT;")?;

        Ok(())
    }
}

impl<'a> Drop for AddFireTransaction<'a> {
    fn drop(&mut self) {
        self.flush().unwrap()
    }
}
