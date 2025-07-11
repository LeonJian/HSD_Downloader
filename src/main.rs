use chrono::{NaiveDateTime, Utc};
use std::fmt::Formatter;
use std::{fmt, io};

static DATE_FMT: &str = r#"%Y-%m-%d %H:%M:%S"#;

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "----------  Himawari HSD Data Downloader Version {}  ----------",
        version
    );

    let current_time = Utc::now();
    println!(
        "Current UTC Time: {}",
        current_time.format(DATE_FMT).to_string()
    );

    let download_period = input_time();

    match download_period {
        Some(download_period) => {
            println!("Download Period: {}", download_period);
        }
        None => {
            println!("No download period");
        }
    }
}

struct DownloadTime {
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
}

impl fmt::Display for DownloadTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Download start time: {}\nDownload end time: {}",
            self.start_time, self.end_time
        )
        .expect("Failed to format DownloadTime for display");
        Ok(())
    }
}

fn convert_input_str_to_naive_date_time(
    input_time: &str,
) -> Result<NaiveDateTime, chrono::ParseError> {
    let time = NaiveDateTime::parse_from_str(input_time, DATE_FMT);
    time
}

fn return_naive_date_time() -> Option<NaiveDateTime> {
    let mut download_time = String::new();
    io::stdin().read_line(&mut download_time).unwrap();

    let download_time = download_time.trim();
    if download_time.is_empty() {
        return None;
    }
    let start_end_time = match convert_input_str_to_naive_date_time(download_time) {
        Ok(naive_date_time) => naive_date_time,
        Err(_) => return None,
    };
    Some(start_end_time)
}

fn input_time() -> Option<DownloadTime> {
    println!("Input download start time: ({})", DATE_FMT);
    let start_time = match return_naive_date_time() {
        Some(naive_date_time) => naive_date_time,
        None => return None,
    };

    println!("Input download end time: ({})", DATE_FMT);
    let end_time = return_naive_date_time().unwrap_or_else(|| start_time); // if end_time is nothing, we will use the start time.

    let download_period = DownloadTime {
        start_time,
        end_time,
    };
    Some(download_period)
}
