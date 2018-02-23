use maud::{html, Markup, DOCTYPE};

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
) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title "EventBot | New Event";
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
                            label for="title" "Title";
                            input type="text" name="title" value=(create_event.title);

                            label for="description" "Description";
                            textarea form="event" name="description" {
                                (create_event.description)
                            }

                            lable for="start_year" "Start Year";
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

                            label for="start_month" "Start Month";
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

                            label for="start_day" "Start Day";
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

                            label for="start_hour" "Start Hour";
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

                            label for="start_minute" "Start Minute";
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

                            lable for="end_year" "End Year";
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

                            label for="end_month" "End Month";
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

                            label for="end_day" "End Day";
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

                            label for="end_hour" "End Hour";
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

                            label for="end_minute" "End Minute";
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

                            label for="timezone" "Timezone";
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

                            input type="hidden" name="secret" value=(id);
                            input type="submit" value="Submit";
                        }
                    }
                }
            }
        }
    }
}

pub fn success(event: Event) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title {
                    "EventBot | Event Created"
                }
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
                            "Start" (event.start_date().to_rfc2822())
                        }
                        p {
                            "End" (event.end_date().to_rfc2822())
                        }
                    }
                }
            }
        }
    }
}
