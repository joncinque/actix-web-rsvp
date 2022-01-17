use {
    crate::{
        error::RsvpError,
        model::{RsvpModel, RsvpParams},
    },
    chrono::{DateTime, Utc},
    csv::{ReaderBuilder, WriterBuilder},
    std::{
        fs::File,
        io::{Seek, SeekFrom},
    },
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

    pub fn update_time(&mut self, new_datetime: DateTime<Utc>) {
        self.datetime = new_datetime;
    }

    /// Upsert a new record at the end.
    ///
    /// Search for a record. If not found, insert a new record at the end. If found,
    /// erase the previous record and insert a new one.
    pub fn upsert(&mut self, params: RsvpParams) -> Result<(), RsvpError> {
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
        wtr.serialize(record_to_insert).map_err(RsvpError::from)
    }

    /// Removes a record by name if found, rewriting the whole file
    ///
    /// Ideally, we could use an memmap, clear just the bytes of the entry,
    /// and append at the end, with some regular compaction.  This is good enough
    /// for v1 and small enough sets.
    pub fn remove(&mut self, name: &str) -> Result<Option<RsvpModel>, RsvpError> {
        let records = self.get_all()?;
        if let Some(record) = records.iter().find(|r| r.name == name) {
            let record = record.clone();
            self.file.seek(SeekFrom::Start(0))?;
            let mut wtr = WriterBuilder::new()
                .has_headers(false)
                .from_writer(&self.file);
            for record in records {
                if record.name != name {
                    wtr.serialize(record).map_err(RsvpError::from)?;
                }
            }
            Ok(Some(record))
        } else {
            self.file.seek(SeekFrom::End(0))?;
            Ok(None)
        }
    }

    /// Get all records
    pub fn get_all(&mut self) -> Result<Vec<RsvpModel>, RsvpError> {
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
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::io::{BufReader, Read, Seek, SeekFrom},
        tempfile::tempfile,
    };

    fn test_rsvp() -> RsvpParams {
        RsvpParams {
            name: "John".to_string(),
            attending: true,
        }
    }

    fn test_rsvps(num: usize) -> Vec<RsvpParams> {
        (0..num)
            .map(|n| RsvpParams {
                name: format!("John-{}", n),
                attending: n % 2 == 0,
            })
            .collect()
    }

    #[test]
    fn insert() {
        let datetime = Utc::now();
        let mut db = CsvDb::new_with_time(tempfile().unwrap(), datetime);
        db.upsert(test_rsvp()).unwrap();

        db.file.seek(SeekFrom::Start(0)).unwrap();
        let mut contents = String::new();
        let mut buf_reader = BufReader::new(&db.file);
        buf_reader.read_to_string(&mut contents).unwrap();
        assert_eq!(
            format!("John,true,{:?},{:?}\n", datetime, datetime),
            contents
        );

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), 1);
        let test_record = RsvpModel::new_with_params(test_rsvp(), datetime);
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
            db.upsert(rsvp).unwrap();
        }

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), num_test);

        let test_index = num_test / 2;
        assert!(!all_records[test_index].attending);

        let updated = RsvpParams {
            name: format!("John-{}", test_index),
            attending: true,
        };
        db.upsert(updated.clone()).unwrap();

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), num_test);
        assert_eq!(all_records[num_test - 1].name, updated.name);
        assert_eq!(all_records[num_test - 1].attending, updated.attending);
    }
}
