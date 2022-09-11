use {
    crate::{
        error::Error,
        model::{AddParams, Attendance, RsvpModel, RsvpParams},
    },
    chrono::{DateTime, Utc},
    csv::{ReaderBuilder, WriterBuilder},
    log::error,
    std::{
        fs::File,
        io::{BufReader, Read, Seek, SeekFrom, Write},
    },
    tempfile::tempfile,
};

const HEADER_LINE: &str = "name,email,attending,attending_secondary,attending_tertiary,meal_choice,dietary_restrictions,plus_one_attending,plus_one_name,plus_one_meal_choice,plus_one_dietary_restrictions,comments,created_at,updated_at";

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

    /// Inserts a new record just based on names
    pub fn insert(&mut self, params: &AddParams) -> Result<RsvpModel, Error> {
        if let Some(model) = self.get(&params.name)? {
            self.file.seek(SeekFrom::End(0))?;
            error!(
                "Attempted to add {:?}, but {:?} exists already",
                params, model
            );
            Err(Error::Add(params.clone()))
        } else {
            self.file.seek(SeekFrom::End(0))?;
            let record_to_insert = RsvpModel::new_with_add(params, self.datetime);
            let mut wtr = WriterBuilder::new()
                .has_headers(false)
                .from_writer(&self.file);
            wtr.serialize(record_to_insert.clone())
                .map_err(Error::from)?;
            wtr.flush()?;
            Ok(record_to_insert)
        }
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
            RsvpModel::new_with_rsvp(params, self.datetime)
        };
        let mut wtr = WriterBuilder::new()
            .has_headers(false)
            .from_writer(&self.file);
        wtr.serialize(record_to_insert.clone())
            .map_err(Error::from)?;
        wtr.flush()?;
        Ok(record_to_insert)
    }

    /// Removes a record by name if found, rewriting the whole file
    ///
    /// Ideally, we could use an memmap, clear just the bytes of the entry,
    /// and append at the end, with some regular compaction.  This is good enough
    /// for v1 and small enough sets.
    pub fn remove(&mut self, name: &str) -> Result<Option<RsvpModel>, Error> {
        let records = self.get_all()?;
        let name = name.trim().to_lowercase();
        if let Some(record) = records.iter().find(|r| r.name.to_lowercase() == name) {
            self.file.set_len(0)?;
            let record = record.clone();
            self.file.seek(SeekFrom::Start(0))?;
            let mut wtr = WriterBuilder::new()
                .has_headers(true)
                .from_writer(&self.file);
            for record in records {
                if record.name.to_lowercase() != name {
                    wtr.serialize(record).map_err(Error::from)?;
                }
            }
            wtr.flush()?;
            Ok(Some(record))
        } else {
            self.file.seek(SeekFrom::End(0))?;
            Ok(None)
        }
    }

    /// Get a specific record
    pub fn get(&mut self, name: &str) -> Result<Option<RsvpModel>, Error> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(&self.file);
        for result in reader.deserialize() {
            let rsvp: RsvpModel = result?;
            for name in name.split('&') {
                let name = name.trim().to_lowercase();
                if rsvp.name.to_lowercase() == name || rsvp.plus_one_name.to_lowercase() == name {
                    return Ok(Some(rsvp));
                }
            }
        }
        Ok(None)
    }

    /// Get all records
    pub fn get_all(&mut self) -> Result<Vec<RsvpModel>, Error> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(&self.file);
        let mut records = vec![];
        for result in reader.deserialize() {
            let rsvp: RsvpModel = result?;
            records.push(rsvp);
        }
        Ok(records)
    }

    /// Get the current attendance numbers
    pub fn attendance(&mut self) -> Result<Attendance, Error> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(&self.file);
        let mut attendance = Attendance::default();
        for result in reader.deserialize() {
            let rsvp: RsvpModel = result?;
            let number_attending = if rsvp.plus_one_attending { 2 } else { 1 };
            if rsvp.attending {
                attendance.attending += number_attending;
            }
            if rsvp.attending_secondary {
                attendance.attending_secondary += number_attending;
            }
            if rsvp.attending_tertiary {
                attendance.attending_tertiary += number_attending;
            }
        }
        Ok(attendance)
    }

    /// Doesn't implement ToString because it requires a `&mut self`
    pub fn dump(&mut self) -> String {
        self.file.seek(SeekFrom::Start(0)).unwrap();
        let mut contents = String::new();
        let mut buf_reader = BufReader::new(&self.file);
        buf_reader.read_to_string(&mut contents).unwrap();
        contents
    }

    /// Add just the header row, useful for testing
    pub fn add_header(&mut self) {
        self.file.seek(SeekFrom::Start(0)).unwrap();
        writeln!(self.file, "{}", HEADER_LINE).unwrap();
    }
}
impl Default for CsvDb {
    fn default() -> Self {
        let mut db = CsvDb::new(tempfile().unwrap());
        db.add_header();
        db
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    pub fn test_db(num: usize) -> CsvDb {
        let mut db = CsvDb::new(tempfile().unwrap());
        db.add_header();
        let rsvps = test_rsvps(num);
        for rsvp in rsvps {
            db.upsert(&rsvp).unwrap();
        }
        db
    }

    pub fn test_add() -> AddParams {
        AddParams {
            name: "John".to_string(),
            email: "john@john.john".to_string(),
            plus_one_name: "Johnson".to_string(),
        }
    }

    pub fn test_rsvp() -> RsvpParams {
        RsvpParams {
            name: "John".to_string(),
            email: "john@john.john".to_string(),
            attending: true,
            attending_secondary: true,
            attending_tertiary: false,
            meal_choice: "Fish".to_string(),
            dietary_restrictions: "Yes".to_string(),
            plus_one_attending: true,
            plus_one_name: "Johnson".to_string(),
            plus_one_meal_choice: "Veggies".to_string(),
            plus_one_dietary_restrictions: "No".to_string(),
            comments: "Can't wait!".to_string(),
        }
    }

    fn test_rsvps(num: usize) -> Vec<RsvpParams> {
        (0..num)
            .map(|n| RsvpParams {
                name: format!("John-{}", n),
                email: format!("john{}@john.john", n),
                attending: n % 2 == 0,
                attending_secondary: n % 3 == 0,
                attending_tertiary: n % 5 == 0,
                meal_choice: "Meat".to_string(),
                dietary_restrictions: "".to_string(),
                plus_one_attending: n % 2 == 0,
                plus_one_name: format!("Johnson-{}", n),
                plus_one_meal_choice: "Veggie".to_string(),
                plus_one_dietary_restrictions: "Vegetarian".to_string(),
                comments: format!("{} comments!", n),
            })
            .collect()
    }

    #[test]
    fn insert() {
        let datetime = Utc::now();
        let mut db = CsvDb::new_with_time(tempfile().unwrap(), datetime);
        db.add_header();
        let add = test_add();
        let model = db.insert(&add).unwrap();

        let contents = db.dump();
        assert_eq!(
            format!(
                "{}\n{},{},{},{},{},{},{},{},{},{},{},{},{:?},{:?}\n",
                HEADER_LINE,
                model.name,
                model.email,
                model.attending,
                model.attending_secondary,
                model.attending_tertiary,
                model.meal_choice,
                model.dietary_restrictions,
                model.plus_one_attending,
                model.plus_one_name,
                model.plus_one_meal_choice,
                model.plus_one_dietary_restrictions,
                model.comments,
                datetime,
                datetime
            ),
            contents
        );

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), 1);
        let test_record = RsvpModel::new_with_add(&add, datetime);
        assert_eq!(all_records[0], test_record);
        assert!(db.remove(&add.name).unwrap().is_some());
        assert!(db.remove("Blah").unwrap().is_none());
        assert_eq!(db.attendance().unwrap(), Attendance::default());
    }

    #[test]
    fn upsert_one() {
        let datetime = Utc::now();
        let mut db = CsvDb::new_with_time(tempfile().unwrap(), datetime);
        db.add_header();
        let rsvp = test_rsvp();
        db.upsert(&rsvp).unwrap();

        let contents = db.dump();
        assert_eq!(
            format!(
                "{}\n{},{},{},{},{},{},{},{},{},{},{},{},{:?},{:?}\n",
                HEADER_LINE,
                rsvp.name,
                rsvp.email,
                rsvp.attending,
                rsvp.attending_secondary,
                rsvp.attending_tertiary,
                rsvp.meal_choice,
                rsvp.dietary_restrictions,
                rsvp.plus_one_attending,
                rsvp.plus_one_name,
                rsvp.plus_one_meal_choice,
                rsvp.plus_one_dietary_restrictions,
                rsvp.comments,
                datetime,
                datetime
            ),
            contents
        );

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), 1);
        let test_record = RsvpModel::new_with_rsvp(&test_rsvp(), datetime);
        assert_eq!(all_records[0], test_record);
        assert!(db.remove(&test_rsvp().name).unwrap().is_some());
        assert!(db.remove("Blah").unwrap().is_none());
        assert_eq!(db.attendance().unwrap(), Attendance::default());
    }

    #[test]
    fn upsert() {
        let mut db = CsvDb::default();
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
            email: "".to_string(),
            attending: true,
            attending_secondary: true,
            attending_tertiary: true,
            meal_choice: "".to_string(),
            dietary_restrictions: "".to_string(),
            plus_one_attending: false,
            plus_one_name: "".to_string(),
            plus_one_meal_choice: "".to_string(),
            plus_one_dietary_restrictions: "".to_string(),
            comments: "No comment.".to_string(),
        };
        db.upsert(&updated).unwrap();

        let all_records = db.get_all().unwrap();
        assert_eq!(all_records.len(), num_test);
        assert_eq!(all_records[num_test - 1].name, updated.name);
        assert_eq!(all_records[num_test - 1].attending, updated.attending);

        let mut attendance = Attendance::default();
        for record in all_records {
            let number_attending = if record.plus_one_attending { 2 } else { 1 };
            if record.attending {
                attendance.attending += number_attending;
            }
            if record.attending_secondary {
                attendance.attending_secondary += number_attending;
            }
            if record.attending_tertiary {
                attendance.attending_tertiary += number_attending;
            }
        }
        assert_eq!(db.attendance().unwrap(), attendance);
    }

    fn check_name(name: &str) {
        let mut db = CsvDb::default();
        db.upsert(&RsvpParams {
            name: name.to_string(),
            email: name.to_string(),
            attending: false,
            attending_secondary: true,
            attending_tertiary: true,
            meal_choice: "".to_string(),
            dietary_restrictions: "".to_string(),
            plus_one_attending: false,
            plus_one_name: "".to_string(),
            plus_one_meal_choice: "".to_string(),
            plus_one_dietary_restrictions: "".to_string(),
            comments: "No comment.".to_string(),
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

    #[test]
    fn get() {
        let datetime = Utc::now();
        let mut db = CsvDb::new_with_time(tempfile().unwrap(), datetime);
        db.add_header();
        let rsvp = test_rsvp();
        db.upsert(&rsvp).unwrap();

        db.get(&rsvp.name.to_uppercase()).unwrap().unwrap();
        db.get(&format!(" {} ", rsvp.name)).unwrap().unwrap();
        db.get(&format!(" {} ", rsvp.plus_one_name))
            .unwrap()
            .unwrap();
        db.get(&format!(" {} & {} ", rsvp.name, rsvp.plus_one_name))
            .unwrap()
            .unwrap();
    }
}
