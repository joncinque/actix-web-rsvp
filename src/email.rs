use {
    crate::{error::Error, model::RsvpParams},
    lettre::{
        message::{Attachment, Message, MultiPart, SinglePart},
        transport::stub::StubTransport,
        AsyncSendmailTransport, AsyncTransport, Tokio1Executor,
    },
    log::info,
};

#[derive(Default)]
pub struct Email {
    pub from: String,
    pub admin: String,
}
impl Email {
    pub fn new(from: &str, admin: &str) -> Self {
        Self {
            from: from.to_string(),
            admin: admin.to_string(),
        }
    }

    async fn send_message(&self, message: Message, test: bool) -> Result<(), Error> {
        // types are gross, we can probably do this better some other time
        if test {
            info!("Sending message: {:?}", message);
            let sender = StubTransport::new_ok();
            sender.send(message).await.map_err(Error::from)
        } else {
            let sender = AsyncSendmailTransport::<Tokio1Executor>::new();
            sender.send(message).await.map_err(Error::from)
        }
    }

    fn csv_email(&self, rsvp: &RsvpParams, csv_contents: String) -> Result<Message, Error> {
        Message::builder()
            .from(self.from.parse().map_err(Error::from)?)
            .reply_to(self.from.parse().map_err(Error::from)?)
            .to(self.admin.parse().map_err(Error::from)?)
            .subject("New RSVP!")
            .multipart(
                MultiPart::mixed()
                    .singlepart(SinglePart::plain(format!(
                        "Success on new RSVP!\n{}",
                        serde_json::to_string(rsvp).map_err(Error::from)?
                    )))
                    .singlepart(
                        Attachment::new("rsvp.csv".to_string())
                            .body(csv_contents, "text/csv".parse().unwrap()),
                    ),
            )
            .map_err(Error::from)
    }

    fn error_email(&self, error: &Error, rsvp: &RsvpParams) -> Result<Message, Error> {
        Message::builder()
            .from(self.from.parse().map_err(Error::from)?)
            .reply_to(self.from.parse().map_err(Error::from)?)
            .to(self.admin.parse().map_err(Error::from)?)
            .subject("Error on RSVP")
            .multipart(
                MultiPart::mixed()
                    .singlepart(SinglePart::plain(format!("Error on new RSVP, try to get in touch with them or put it in yourself.\nError: {}\nRSVP: {}", error, serde_json::to_string(rsvp).map_err(Error::from)?)))
            ).map_err(Error::from)
    }

    pub async fn send_csv(
        &self,
        rsvp: &RsvpParams,
        csv_contents: String,
        test: bool,
    ) -> Result<(), Error> {
        let message = self.csv_email(rsvp, csv_contents)?;
        self.send_message(message, test).await?;
        Ok(())
    }

    pub async fn send_rsvp_error(
        &self,
        error: &Error,
        rsvp: &RsvpParams,
        test: bool,
    ) -> Result<(), Error> {
        let message = self.error_email(error, rsvp)?;
        self.send_message(message, test).await?;
        Ok(())
    }
}
