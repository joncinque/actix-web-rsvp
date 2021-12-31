use {
    crate::{error::Error, model::RsvpParams},
    csv::Writer,
    std::fs::File,
};

pub struct CsvDb {
    file: File,
}

impl CsvDb {
    pub fn new(file: File) -> Self {
        Self { file }
    }

    pub fn insert(&mut self, params: RsvpParams) -> Result<(), Error> {
        let mut wtr = Writer::from_writer(&self.file);
        wtr.serialize(params).map_err(Error::from)
    }
}
