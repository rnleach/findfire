/*! Methods and types to support querying the fires table of the database. */

use std::error::Error;

use chrono::NaiveDateTime;
use geo::{Point, Polygon};
use rusqlite::ToSql;

impl super::FiresDatabase {
    pub fn add_fire_handle(&self) -> Result<AddFireTransaction, Box<dyn Error>> {
        let stmt = self.db.prepare(include_str!("add_fire.sql"))?;

        self.db.execute("BEGIN", [])?;
        Ok(AddFireTransaction(stmt, &self.db))
    }
}

/*
pub struct FireQuery<'a>(rusqlite::Statement<'a>);

impl<'a> FireQuery<'a> {
    pub fn records_for(
        &mut self,
        satellite: &str,
    ) -> Result<impl Iterator<Item = ClusterRecord> + '_, Box<dyn Error>> {
        let rows = self
            .0
            .query_and_then(&[satellite], |row| {
                let id: i64 = row.get(0)?;
                let valid_time: NaiveDateTime =
                    chrono::NaiveDateTime::from_timestamp(row.get::<_, i64>(1)?, 0);
                let lat: f64 = row.get(2)?;
                let lon: f64 = row.get(3)?;
                let centroid = point!(x: lat, y: lon);
                let power: f64 = row.get(4)?;

                let pblob = row.get_ref(5)?.as_blob()?;

                let perimeter: Polygon<f64> =
                    bincode::deserialize(&pblob).map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(ClusterRecord::new(
                    id, valid_time, power, perimeter, centroid,
                ))
            })?
            .filter_map(|res: Result<_, rusqlite::Error>| res.ok());

        Ok(rows)
    }
}
*/

// TODO add firecode!
pub struct AddFireTransaction<'a>(rusqlite::Statement<'a>, &'a rusqlite::Connection);

impl<'a> AddFireTransaction<'a> {
    pub fn add_fire(
        &mut self,
        fire_id: &str,
        satellite: &str,
        last_oberved: NaiveDateTime,
        origin: Point<f64>,
        perimeter: Polygon<f64>,
    ) -> Result<(), Box<dyn Error>> {
        let lat = origin.x();
        let lon = origin.y();

        let perimeter = bincode::serialize(&perimeter)?;

        let _ = self.0.execute([
            &fire_id as &dyn ToSql,
            &satellite,
            &last_oberved.timestamp(),
            &lat,
            &lon,
            &perimeter,
        ])?;

        Ok(())
    }
}

impl<'a> Drop for AddFireTransaction<'a> {
    fn drop(&mut self) {
        self.1.execute("COMMIT", []).unwrap();
    }
}
