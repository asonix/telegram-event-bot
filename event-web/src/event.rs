use std::str::FromStr;

use chrono::{DateTime, Datelike, Timelike};
use chrono::offset::Utc;
use chrono_tz::Tz;
use failure::{Fail, ResultExt};

use error::{FrontendError, FrontendErrorKind, MissingField};

pub struct Event {
    title: String,
    description: String,
    datetime: DateTime<Tz>,
}

impl Event {
    pub fn from_option(option_event: Option<OptionEvent>) -> Result<Self, FrontendError> {
        CreateEvent::from_option(option_event.ok_or(FrontendErrorKind::MissingField)?)?
            .try_to_event()
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn datetime(&self) -> DateTime<Tz> {
        self.datetime
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionEvent {
    title: Option<String>,
    description: Option<String>,
    year: Option<i32>,
    month: Option<u32>,
    day: Option<u32>,
    hour: Option<u32>,
    minute: Option<u32>,
    timezone: Option<String>,
}

impl OptionEvent {
    pub fn new(
        title: Option<String>,
        description: Option<String>,
        year: Option<i32>,
        month: Option<u32>,
        day: Option<u32>,
        hour: Option<u32>,
        minute: Option<u32>,
        timezone: Option<String>,
    ) -> Self {
        OptionEvent {
            title: title.and_then(|title| {
                if title.trim().len() == 0 {
                    None
                } else {
                    Some(title)
                }
            }),
            description: description.and_then(|description| {
                if description.trim().len() == 0 {
                    None
                } else {
                    Some(description)
                }
            }),
            year: year,
            month: month,
            day: day,
            hour: hour,
            minute: minute,
            timezone: timezone.and_then(|tz| if tz.trim().len() == 0 { None } else { Some(tz) }),
        }
    }

    pub fn missing_keys(&self) -> Vec<&'static str> {
        let mut v = Vec::new();

        if self.title.is_none() {
            v.push("title");
        }

        if self.description.is_none() {
            v.push("description");
        }

        if self.year.is_none() {
            v.push("year");
        }

        if self.month.is_none() {
            v.push("month");
        }

        if self.day.is_none() {
            v.push("day");
        }

        if self.hour.is_none() {
            v.push("hour");
        }

        if self.minute.is_none() {
            v.push("minute");
        }

        if self.timezone.is_none() {
            v.push("timezone");
        }

        v
    }
}

pub struct CreateEvent {
    pub title: String,
    pub description: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub timezone: String,
}

impl CreateEvent {
    pub fn default_from(date: DateTime<Tz>) -> Self {
        CreateEvent {
            title: "".to_owned(),
            description: "".to_owned(),
            year: date.year(),
            month: date.month() - 1,
            day: date.day() as u32,
            hour: date.hour() as u32,
            minute: date.minute() as u32,
            timezone: date.timezone().name().to_owned(),
        }
    }

    pub fn merge(&mut self, option_event: &OptionEvent) {
        if let Some(ref title) = option_event.title {
            self.title = title.to_owned();
        }

        if let Some(ref description) = option_event.description {
            self.description = description.to_owned();
        }

        if let Some(year) = option_event.year {
            self.year = year;
        }

        if let Some(month) = option_event.month {
            self.month = month;
        }

        if let Some(day) = option_event.day {
            self.day = day;
        }

        if let Some(hour) = option_event.hour {
            self.hour = hour;
        }

        if let Some(minute) = option_event.minute {
            self.minute = minute;
        }

        if let Some(ref timezone) = option_event.timezone {
            self.timezone = timezone.to_owned();
        }
    }

    fn from_option(option_event: OptionEvent) -> Result<Self, FrontendError> {
        let title = maybe_empty_string(maybe_field(option_event.title, "title")?, "title")?;
        let description = maybe_empty_string(
            maybe_field(option_event.description, "description")?,
            "description",
        )?;
        let year = maybe_field(option_event.year, "year")?;
        let month = maybe_field(option_event.month, "month")?;
        let day = maybe_field(option_event.day, "day")?;
        let hour = maybe_field(option_event.hour, "hour")?;
        let minute = maybe_field(option_event.minute, "minute")?;
        let timezone = maybe_field(option_event.timezone, "timezone")?;

        Ok(CreateEvent {
            title,
            description,
            year,
            month,
            day,
            hour,
            minute,
            timezone,
        })
    }

    fn try_to_event(self) -> Result<Event, FrontendError> {
        let timezone = Tz::from_str(&self.timezone).map_err(|_| FrontendErrorKind::BadTimeZone)?;

        let now = Utc::now();

        let datetime = now.with_timezone(&timezone);
        let datetime = datetime
            .with_year(self.year)
            .ok_or(FrontendErrorKind::BadYear)?
            .with_month0(self.month)
            .ok_or(FrontendErrorKind::BadMonth)?
            .with_day(self.day)
            .ok_or(FrontendErrorKind::BadDay)?
            .with_hour(self.hour)
            .ok_or(FrontendErrorKind::BadHour)?
            .with_minute(self.minute)
            .ok_or(FrontendErrorKind::BadMinute)?
            .with_second(0)
            .ok_or(FrontendErrorKind::BadSecond)?;

        Ok(Event {
            title: self.title,
            description: self.description,
            datetime: datetime,
        })
    }
}

fn maybe_field<T>(maybe: Option<T>, field: &'static str) -> Result<T, FrontendError> {
    Ok(maybe
        .ok_or(MissingField { field })
        .context(FrontendErrorKind::MissingField)?)
}

fn maybe_empty_string(s: String, field: &'static str) -> Result<String, FrontendError> {
    let s = s.trim().to_owned();

    if s.len() == 0 {
        Err(MissingField { field }
            .context(FrontendErrorKind::MissingField)
            .into())
    } else {
        Ok(s)
    }
}
