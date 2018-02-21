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

                            lable for="year" "Year";
                            select name="year" {
                                @for year in &years {
                                    @if year == &create_event.year {
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

                            label for="month" "Month";
                            select name="month" {
                                @for &(i, month) in &months {
                                    @if i == create_event.month {
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

                            label for="day" "Day";
                            select name="day" {
                                @for day in &days {
                                    @if day == &create_event.day {
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

                            label for="hour" "Hour";
                            select name="hour" {
                                @for hour in &hours {
                                    @if hour == &create_event.hour {
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

                            label for="minute" "Minute";
                            select name="minute" {
                                @for minute in &minutes {
                                    @if minute == &create_event.minute {
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
                            (event.datetime().to_rfc2822())
                        }
                    }
                }
            }
        }
    }
}
