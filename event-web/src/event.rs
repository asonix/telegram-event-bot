/*
 * This file is part of Event Web
 *
 * Event Web is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Event Web is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with Event Web.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::str::FromStr;

use chrono::{DateTime, Datelike, Timelike};
use chrono::offset::Utc;
use chrono_tz::Tz;
use failure::{Fail, ResultExt};

use error::{FrontendError, FrontendErrorKind, MissingField};

#[derive(Clone, Debug)]
pub struct Event {
    title: String,
    description: String,
    start_date: DateTime<Tz>,
    end_date: DateTime<Tz>,
}

impl Event {
    pub fn from_parts(
        title: String,
        description: String,
        start_date: DateTime<Tz>,
        end_date: DateTime<Tz>,
    ) -> Self {
        Event {
            title,
            description,
            start_date,
            end_date,
        }
    }
    pub fn from_option(option_event: OptionEvent) -> Result<Self, FrontendError> {
        CreateEvent::from_option(option_event)?.try_to_event()
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn start_date(&self) -> DateTime<Tz> {
        self.start_date
    }

    pub fn end_date(&self) -> DateTime<Tz> {
        self.end_date
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionEvent {
    title: Option<String>,
    description: Option<String>,
    start_year: Option<i32>,
    start_month: Option<u32>,
    start_day: Option<u32>,
    start_hour: Option<u32>,
    start_minute: Option<u32>,
    end_year: Option<i32>,
    end_month: Option<u32>,
    end_day: Option<u32>,
    end_hour: Option<u32>,
    end_minute: Option<u32>,
    timezone: Option<String>,
}

impl OptionEvent {
    pub fn new(
        title: Option<String>,
        description: Option<String>,
        start_year: Option<i32>,
        start_month: Option<u32>,
        start_day: Option<u32>,
        start_hour: Option<u32>,
        start_minute: Option<u32>,
        end_year: Option<i32>,
        end_month: Option<u32>,
        end_day: Option<u32>,
        end_hour: Option<u32>,
        end_minute: Option<u32>,
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
            start_year: start_year,
            start_month: start_month,
            start_day: start_day,
            start_hour: start_hour,
            start_minute: start_minute,
            end_year: end_year,
            end_month: end_month,
            end_day: end_day,
            end_hour: end_hour,
            end_minute: end_minute,
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

        if self.start_year.is_none() {
            v.push("start year");
        }

        if self.start_month.is_none() {
            v.push("start month");
        }

        if self.start_day.is_none() {
            v.push("start day");
        }

        if self.start_hour.is_none() {
            v.push("start hour");
        }

        if self.start_minute.is_none() {
            v.push("start minute");
        }

        if self.end_year.is_none() {
            v.push("start year");
        }

        if self.end_month.is_none() {
            v.push("start month");
        }

        if self.end_day.is_none() {
            v.push("start day");
        }

        if self.end_hour.is_none() {
            v.push("start hour");
        }

        if self.end_minute.is_none() {
            v.push("start minute");
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
    pub start_year: i32,
    pub start_month: u32,
    pub start_day: u32,
    pub start_hour: u32,
    pub start_minute: u32,
    pub end_year: i32,
    pub end_month: u32,
    pub end_day: u32,
    pub end_hour: u32,
    pub end_minute: u32,
    pub timezone: String,
}

impl CreateEvent {
    pub fn default_from(date: DateTime<Tz>) -> Self {
        CreateEvent {
            title: "".to_owned(),
            description: "".to_owned(),
            start_year: date.year(),
            start_month: date.month() - 1,
            start_day: date.day() as u32,
            start_hour: date.hour() as u32,
            start_minute: date.minute() as u32,
            end_year: date.year(),
            end_month: date.month() - 1,
            end_day: date.day() as u32,
            end_hour: date.hour() as u32,
            end_minute: date.minute() as u32,
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

        if let Some(start_year) = option_event.start_year {
            self.start_year = start_year;
        }

        if let Some(start_month) = option_event.start_month {
            self.start_month = start_month;
        }

        if let Some(start_day) = option_event.start_day {
            self.start_day = start_day;
        }

        if let Some(start_hour) = option_event.start_hour {
            self.start_hour = start_hour;
        }

        if let Some(start_minute) = option_event.start_minute {
            self.start_minute = start_minute;
        }

        if let Some(end_year) = option_event.end_year {
            self.end_year = end_year;
        }

        if let Some(end_month) = option_event.end_month {
            self.end_month = end_month;
        }

        if let Some(end_day) = option_event.end_day {
            self.end_day = end_day;
        }

        if let Some(end_hour) = option_event.end_hour {
            self.end_hour = end_hour;
        }

        if let Some(end_minute) = option_event.end_minute {
            self.end_minute = end_minute;
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
        let start_year = maybe_field(option_event.start_year, "start_year")?;
        let start_month = maybe_field(option_event.start_month, "start_month")?;
        let start_day = maybe_field(option_event.start_day, "start_day")?;
        let start_hour = maybe_field(option_event.start_hour, "start_hour")?;
        let start_minute = maybe_field(option_event.start_minute, "start_minute")?;
        let end_year = maybe_field(option_event.end_year, "end_year")?;
        let end_month = maybe_field(option_event.end_month, "end_month")?;
        let end_day = maybe_field(option_event.end_day, "end_day")?;
        let end_hour = maybe_field(option_event.end_hour, "end_hour")?;
        let end_minute = maybe_field(option_event.end_minute, "end_minute")?;
        let timezone = maybe_field(option_event.timezone, "timezone")?;

        Ok(CreateEvent {
            title,
            description,
            start_year,
            start_month,
            start_day,
            start_hour,
            start_minute,
            end_year,
            end_month,
            end_day,
            end_hour,
            end_minute,
            timezone,
        })
    }

    fn try_to_event(self) -> Result<Event, FrontendError> {
        let timezone = Tz::from_str(&self.timezone).map_err(|_| FrontendErrorKind::BadTimeZone)?;

        let now = Utc::now();

        let datetime = now.with_timezone(&timezone);
        let start_datetime = datetime
            .with_year(self.start_year)
            .ok_or(FrontendErrorKind::BadYear)?
            .with_month0(self.start_month)
            .ok_or(FrontendErrorKind::BadMonth)?
            .with_day(self.start_day)
            .ok_or(FrontendErrorKind::BadDay)?
            .with_hour(self.start_hour)
            .ok_or(FrontendErrorKind::BadHour)?
            .with_minute(self.start_minute)
            .ok_or(FrontendErrorKind::BadMinute)?
            .with_second(0)
            .ok_or(FrontendErrorKind::BadSecond)?;

        let end_datetime = datetime
            .with_year(self.end_year)
            .ok_or(FrontendErrorKind::BadYear)?
            .with_month0(self.end_month)
            .ok_or(FrontendErrorKind::BadMonth)?
            .with_day(self.end_day)
            .ok_or(FrontendErrorKind::BadDay)?
            .with_hour(self.end_hour)
            .ok_or(FrontendErrorKind::BadHour)?
            .with_minute(self.end_minute)
            .ok_or(FrontendErrorKind::BadMinute)?
            .with_second(0)
            .ok_or(FrontendErrorKind::BadSecond)?;

        Ok(Event {
            title: self.title,
            description: self.description,
            start_date: start_datetime,
            end_date: end_datetime,
        })
    }
}

impl From<Event> for CreateEvent {
    fn from(e: Event) -> Self {
        CreateEvent {
            title: e.title,
            description: e.description,
            start_year: e.start_date.year(),
            start_month: e.start_date.month(),
            start_day: e.start_date.day(),
            start_hour: e.start_date.hour(),
            start_minute: e.start_date.minute(),
            end_year: e.end_date.year(),
            end_month: e.end_date.month(),
            end_day: e.end_date.day(),
            end_hour: e.end_date.hour(),
            end_minute: e.end_date.minute(),
            timezone: e.end_date.timezone().name().to_owned(),
        }
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
