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

use failure::Fail;
use maud::{html, Markup, DOCTYPE};

use error::FrontendError;
use event::{CreateEvent, Event, OptionEvent};

pub fn form(
    create_event: CreateEvent,
    option_event: Option<OptionEvent>,
    submit_url: String,
    years: Vec<i32>,
    months: Vec<(u32, &&str)>,
    days: Vec<u32>,
    hours: Vec<u32>,
    minutes: Vec<u32>,
    timezones: Vec<&'static str>,
    id: String,
    heading_text: &str,
) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title (heading_text);
                meta charset="utf-8";
                link href="/assets/styles.css" rel="stylesheet" type="text/css";
            }
            body {
                section {
                    @if let Some(o) = option_event {
                        article.missing-keys {
                            h1 {
                                "Please provide the following keys"
                            }
                            ul {
                                @for key in &o.missing_keys() {
                                    li {
                                        (key)
                                    }
                                }
                            }
                        }
                    }
                    article {
                        form#event action=(submit_url) method="POST" {
                            fieldset {
                                legend {
                                    h1 { "New Event" }
                                }
                                div {
                                    label for="title" "Title:";
                                    input type="text" name="title" value=(create_event.title);

                                    label for="description" "Description:";
                                    textarea form="event" name="description" {
                                        (create_event.description)
                                    }

                                    fieldset#first {
                                        legend {
                                            h3 { "Start Date" }
                                        }
                                        div {
                                            label for="start_year" "Year:";
                                            select name="start_year" {
                                                @for year in &years {
                                                    @if year == &create_event.start_year {
                                                        option value=(year) selected="true" {
                                                            (year)
                                                        }
                                                    } @else {
                                                        option value=(year) {
                                                            (year)
                                                        }
                                                    }
                                                }
                                            }

                                            label for="start_month" "Month:";
                                            select name="start_month" {
                                                @for &(i, month) in &months {
                                                    @if i == create_event.start_month {
                                                        option value=(i) selected="true" {
                                                            (month)
                                                        }
                                                    } @else {
                                                        option value=(i) {
                                                            (month)
                                                        }
                                                    }
                                                }
                                            }

                                            label for="start_day" "Day:";
                                            select name="start_day" {
                                                @for day in &days {
                                                    @if day == &create_event.start_day {
                                                        option value=(day) selected="true" {
                                                            (day)
                                                        }
                                                    } @else {
                                                        option value=(day) {
                                                            (day)
                                                        }
                                                    }
                                                }
                                            }

                                            label for="start_hour" "Hour:";
                                            select name="start_hour" {
                                                @for hour in &hours {
                                                    @if hour == &create_event.start_hour {
                                                        option value=(hour) selected="true" {
                                                            (hour)
                                                        }
                                                    } @else {
                                                        option value=(hour) {
                                                            (hour)
                                                        }
                                                    }
                                                }
                                            }

                                            label for="start_minute" "Minute:";
                                            select name="start_minute" {
                                                @for minute in &minutes {
                                                    @if minute == &create_event.start_minute {
                                                        option value=(minute) selected="true" {
                                                            @if *minute < 10 {
                                                                (format!("0{}", minute))
                                                            } @else {
                                                                (minute)
                                                            }
                                                        }
                                                    } @else {
                                                        option value=(minute) {
                                                            @if *minute < 10 {
                                                                (format!("0{}", minute))
                                                            } @else {
                                                                (minute)
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    fieldset#second {
                                        legend {
                                            h3 { "End Date" }
                                        }
                                        div {
                                            label for="end_year" "Year:";
                                            select name="end_year" {
                                                @for year in &years {
                                                    @if year == &create_event.end_year {
                                                        option value=(year) selected="true" {
                                                            (year)
                                                        }
                                                    } @else {
                                                        option value=(year) {
                                                            (year)
                                                        }
                                                    }
                                                }
                                            }

                                            label for="end_month" "Month:";
                                            select name="end_month" {
                                                @for &(i, month) in &months {
                                                    @if i == create_event.end_month {
                                                        option value=(i) selected="true" {
                                                            (month)
                                                        }
                                                    } @else {
                                                        option value=(i) {
                                                            (month)
                                                        }
                                                    }
                                                }
                                            }

                                            label for="end_day" "Day:";
                                            select name="end_day" {
                                                @for day in &days {
                                                    @if day == &create_event.end_day {
                                                        option value=(day) selected="true" {
                                                            (day)
                                                        }
                                                    } @else {
                                                        option value=(day) {
                                                            (day)
                                                        }
                                                    }
                                                }
                                            }

                                            label for="end_hour" "Hour:";
                                            select name="end_hour" {
                                                @for hour in &hours {
                                                    @if hour == &create_event.end_hour {
                                                        option value=(hour) selected="true" {
                                                            (hour)
                                                        }
                                                    } @else {
                                                        option value=(hour) {
                                                            (hour)
                                                        }
                                                    }
                                                }
                                            }

                                            label for="end_minute" "Minute:";
                                            select name="end_minute" {
                                                @for minute in &minutes {
                                                    @if minute == &create_event.end_minute {
                                                        option value=(minute) selected="true" {
                                                            @if *minute < 10 {
                                                                (format!("0{}", minute))
                                                            } @else {
                                                                (minute)
                                                            }
                                                        }
                                                    } @else {
                                                        option value=(minute) {
                                                            @if *minute < 10 {
                                                                (format!("0{}", minute))
                                                            } @else {
                                                                (minute)
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    label for="timezone" "Timezone:";
                                    select name="timezone" {
                                        @for tz in &timezones {
                                            @if tz == &create_event.timezone {
                                                option value=(tz) selected="true" {
                                                    (tz)
                                                }
                                            } @else {
                                                option value=(tz) {
                                                    (tz)
                                                }
                                            }
                                        }
                                    }
                                }

                                input type="hidden" name="secret" value=(id);
                            }
                            input type="submit" value="Submit";
                        }
                    }
                }
            }
        }
    }
}

pub fn success(event: Event, title: &str) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title (title);
                link href="/assets/styles.css" rel="stylesheet" type="text/css";
            }
            body {
                section {
                    article {
                        h1 {
                            "Thanks for creating an event!"
                        }
                        h3 {
                            (event.title())
                        }
                        p {
                            (event.description())
                        }
                        p {
                            "Start: " (event.start_date().to_rfc2822())
                        }
                        p {
                            "End: " (event.end_date().to_rfc2822())
                        }
                    }
                }
            }
        }
    }
}

pub fn error(error: &FrontendError) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title "Event Bot | Error";
                link href="/assets/styles.css" rel="stylesheet" type="text/css";
            }
            body {
                section {
                    article {
                        h1 {
                            "Oops, there was an error processing your request"
                        }
                        @if let Some(cause) = error.cause() {
                            p {
                                (cause)
                            }
                        }
                    }
                }
            }
        }
    }
}
