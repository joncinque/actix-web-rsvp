use {
    crate::{error::Error, model::RsvpParams},
    csv::{ReaderBuilder, WriterBuilder},
    std::io::{Seek, SeekFrom},
    std::fs::File,
};

pub struct CsvDb {
    pub file: File,
}

impl CsvDb {
    pub fn new(file: File) -> Self {
        Self { file }
    }

    pub fn insert(&mut self, params: RsvpParams) -> Result<(), Error> {
        self.file.seek(SeekFrom::End(0))?;
        let mut wtr = WriterBuilder::new().has_headers(false).from_writer(&self.file);
        wtr.serialize(params).map_err(Error::from)
    }

    #[allow(dead_code)]
    pub fn find(&mut self, name: &str) -> Result<bool, Error> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut reader = ReaderBuilder::new().has_headers(false).from_reader(&self.file);
        for result in reader.deserialize() {
            let rsvp: RsvpParams = result?;
            if rsvp.name == name {
                return Ok(true);
            }
        }
        Ok(false)
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

    #[test]
    fn insert() {
        let mut db = CsvDb::new(tempfile().unwrap());
        db.insert(test_rsvp()).unwrap();
        db.file.seek(SeekFrom::Start(0)).unwrap();
        let mut contents = String::new();
        let mut buf_reader = BufReader::new(&db.file);
        buf_reader.read_to_string(&mut contents).unwrap();
        assert_eq!("John,true\n", contents);
        assert!(db.find(&test_rsvp().name).unwrap());
        assert!(!db.find("Blah").unwrap());
    }
}
