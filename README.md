# actix-web-rsvp

Actix web server + frontend to gather RSVPs into a CSV file.

This is a self-hosted alternative to popular websites that gathers RSVPs, like
most wedding or e-card websites. Feel free to customize it to your heart's desire!

## Quickstart

* Start up the webserver in testing mode, so no emails are sent:

```console
$ cargo run -- --testing --csv rsvp.csv test@test.com test@test.com
```

* Go to the homepage at `http://127.0.0.1:8080`
* Click "RSVP"
* Put in the name "test" or "test again". This provides simple name-based gating,
similar to most RSVP websites.
* Fill out the form and hit "Submit"

NOTE: Since there is no authentication in the server, someone can easily
circumvent the name-based gating by hitting the API endpoint directly.

## Features

* Homepage with general information about the event or set of events
* Form to fetch an existing RSVP
* Form for guests to respond to all questions
* Photos page to get your guests excited about the event
* Receive an email notification anytime someone RSVPs, allowing you to see all
activity and potentially help guests. The

### Making your guestlist

Once you have the basic site up and running, you can start customizing it by
adding your guests' names, emails, and expected plus-one into the provided
`rsvp.csv` file.

### Changing form fields

To add or change RSVP fields, you must:

* update the rsvp model at `src/model.rs` and relevant tests
* change `rsvp.csv` to reflect the new fields
* show the new fields at `rsvp.html` and `confirm.html`

### HTML Customization

The `templates` directory contains all of the HTML for the website, so you can
update those as you wish. The provided website uses Material Design Lite for
styling and layout, but any other package can be used.

### Sendmail Configuration

The webserver uses the `sendmail` transport provided by lettre to send
notifications from the "FROM_EMAIL" and to all "ADMIN_EMAIL"s, anytime someone
submits the form. It also attaches the current state of the full database, which
can help with debugging any issues.

It is outside the scope of this README to provide information about setting up
a mail server or mail transport agent that works with `sendmail`. There are many
great tutorials that explain how to setup `postfix` or other mail tools.

## Other features

Use the `-h` flag to get enough information about other features:

```console
$ cargo run -- -h
```

## Client bin

There is also a simple client to add guests, rather than modifying the CSV file
directly. In case the site is already active, this allows for adding guests
without clobbering any other inflight guest RSVPs.

```console
$ cargo run --bin client -- "Test Person" tester@example.com "Other Testperson"
```

Use `-h` to see other options.

## Test

The tests mainly cover basic functionality of the "database" and the main
server routes. This can certainly be expanded!

```console
$ cargo test
```

## Potential TODOs

* Customize the email sender from more than just `sendmail`, or even allow
disabling it entirely!
* Create a migration client to go from an old model to a new model

## Security

This is a hobby project, but was used for a real website! As mentioned earlier,
the simple name-based gating can easily be circumvented, and no encryption or
safety exists by default. The sysadmin must setup proper routes, HTTPS, etc.
