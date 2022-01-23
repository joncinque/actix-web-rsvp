use {
    crate::{
        error::Error,
        model::{RsvpModel, RsvpParams},
    },
    chrono::{DateTime, Utc},
    csv::{ReaderBuilder, WriterBuilder},
    std::{
        fs::File,
        io::{BufReader, Read, Seek, SeekFrom},
    },
    tempfile::tempfile,
};

pub struct CsvDb {
    pub file: File,
    pub datetime: DateTime<Utc>,
}
impl CsvDb {
    pub fn new(file: File) -> Self {
        Self::new_with_time(file, Utc::now())
    }

    pub fn new_with_time(file: File, datetime: DateTime<Utc>) -> Self {
        Self { file, datetime }
    }

    /// Update the time in the CSV file to the given time, useful for testing
    pub fn update_time(&mut self, new_datetime: DateTime<Utc>) {
        self.datetime = new_datetime;
    }

    /// Upsert a new record at the end.
    ///
    /// Search for a record. If not found, insert a new record at the end. If found,
    /// erase the previous record and insert a new one.
    pub fn upsert(&mut self, params: &RsvpParams) -> Result<RsvpModel, Error> {
        let maybe_record = self.remove(&params.name)?; // remove keeps the file in the right place for writing
        let record_to_insert = if let Some(mut record) = maybe_record {
            record.update(params, self.datetime)?;
            record
        } else {
            RsvpModel::new_with_params(params, self.datetime)
        };
        let mut wtr = WriterBuilder::new()
            .has_headers(false)
            .from_writer(&self.file);
        wtr.serialize(record_to_insert.clone())
            .map_err(Error::from)?;
        Ok(record_to_insert)
    }

    /// Removes a record by name if found, rewriting the whole file
    ///
    /// Ideally, we could use an memmap, clear just the bytes of the entry,
    /// and append at the end, with some regular compaction.  This is good enough
    /// for v1 and small enough sets.
    pub fn remove(&mut self, name: &str) -> Result<Option<RsvpModel>, Error> {
        let records = self.get_all()?;
        let name = name.to_lowercase();
        if let Some(record) = records.iter().find(|r| r.name.to_lowercase() == name) {
            self.file.set_len(0)?;
            let record = record.clone();
            self.file.seek(SeekFrom::Start(0))?;
            let mut wtr = WriterBuilder::new()
                .has_headers(false)
                .from_writer(&self.file);
            for record in records {
                if record.name.to_lowercase() != name {
                    wtr.serialize(record).map_err(Error::from)?;
                }
            }
            Ok(Some(record))
        } else {
            self.file.seek(SeekFrom::End(0))?;
            Ok(None)
        }
    }

    /// Get a specific record
    pub fn get(&mut self, name: &str) -> Result<Option<RsvpModel>, Error> {
        self.file.seek(SeekFrom::Start(0))?;
        let name = name.to_lowercase();
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(&self.file);
        for result in reader.deserialize() {
            let rsvp: RsvpModel = result?;
            if rsvp.name.to_lowercase() == name {
                return Ok(Some(rsvp));
            }
        }
        Ok(None)
    }

    /// Get all records
    pub fn get_all(&mut self) -> Result<Vec<RsvpModel>, Error> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(&self.file);
        let mut records = vec![];
        for result in reader.deserialize() {
            let rsvp: RsvpModel = result?;
            records.push(rsvp);
        }
        Ok(records)
    }

    /// Doesn't implement ToString because it requires a `&mut self`
    pub fn dump(&mut self) -> String {
        self.file.seek(SeekFrom::Start(0)).unwrap();
        let mut contents = String::new();
        let mut buf_reader = BufReader::new(&self.file);
        buf_reader.read_to_string(&mut contents).unwrap();
        contents
    }
}
impl Default for CsvDb {
    fn default() -> Self {
        CsvDb::new(tempfile().unwrap())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    pub fn test_db(num: usize) -> CsvDb {
        let mut db = CsvDb::new(tempfile().unwrap());
        let rsvps = test_rsvps(num);
        for rsvp in rsvps {
            db.upsert(&rsvp).unwrap();
        }
        db
    }

    fn test_rsvp() -> RsvpParams {
        RsvpParams {
            name: "John".to_string(),
            attending: true,
            email: "john@john.john".to_string(),
        }
    }

    fn test_rsvps(num: usize) -> Vec<RsvpParams> {
        (0..num)
            .map(|n| RsvpParams {
                name: format!("John-{}", n),
                attending: n % 2 == 0,
                email: format!("john{}@john.john", n),
            })
            .collect()
    }

    #[test]
    fn insert() {
        let datetime = Utc::now();
        let mut db = CsvDb::new_with_time(tempfile().unwrap(), datetime);
        let rsvp = test_rsvp();
        db.upsert(&rsvp).unwrap();

        let contents = db.dump();
        assert_eq!(
            format!(
                "{},{},{},{:?},{:?}\n",
                rsvp.name, rsvp.attending, rsvp.email, datetime, datetime
            ),
            contents
        );

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), 1);
        let test_record = RsvpModel::new_with_params(&test_rsvp(), datetime);
        assert_eq!(all_records[0], test_record);
        assert!(db.remove(&test_rsvp().name).unwrap().is_some());
        assert!(db.remove("Blah").unwrap().is_none());
    }

    #[test]
    fn upsert() {
        let mut db = CsvDb::new(tempfile().unwrap());
        let num_test = 50;
        let rsvps = test_rsvps(50);
        for rsvp in rsvps {
            db.upsert(&rsvp).unwrap();
        }

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), num_test);

        let test_index = num_test / 2;
        assert!(!all_records[test_index].attending);

        let updated = RsvpParams {
            name: format!("John-{}", test_index),
            attending: true,
            email: "".to_string(),
        };
        db.upsert(&updated).unwrap();

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), num_test);
        assert_eq!(all_records[num_test - 1].name, updated.name);
        assert_eq!(all_records[num_test - 1].attending, updated.attending);
    }

    fn check_name(name: &str) {
        let mut db = CsvDb::new(tempfile().unwrap());
        db.upsert(&RsvpParams {
            name: name.to_string(),
            attending: false,
            email: name.to_string(),
        })
        .unwrap();
        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), 1);
        assert_eq!(all_records[0].name, name);
    }

    #[test]
    fn weird_chars() {
        check_name("comma,");
        check_name("newline\n");
        check_name("newline,and comma\n");
    }
}
