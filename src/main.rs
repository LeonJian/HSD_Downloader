use chrono::{Duration, NaiveDateTime, Timelike, Utc};
use std::fmt::Formatter;
use std::{fmt, io};

static DATE_FMT: &str = r#"%Y-%m-%d %H:%M:%S"#;
static TIME_STEP: i64 = 10;

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

    let current_time = current_time.naive_utc();

    let download_period = input_time();

    let download_period = match download_period {
        Some(download_period) => {
            if download_period.start_time <= download_period.end_time {
                if download_period.start_time > current_time
                    || download_period.end_time > current_time
                {
                    panic!("Input time is greater than current time");
                }

                println!("Download Period: {}", download_period);
                download_period
            } else {
                panic!("End time is earlier than start time");
            }
        }
        None => {
            // println!("No download period");
            panic!("No download period")
        }
    };

    let download_time_list = match generate_download_time_list(&download_period) {
        Ok(download_time_list) => download_time_list,
        Err(e) => {
            panic!("Error generating download time list: {}", e);
        }
    };

    println!("Download Time List: {:?}", download_time_list);
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
    println!("Input download start time(UTC Time): ({})", DATE_FMT);
    let start_time = match return_naive_date_time() {
        Some(naive_date_time) => naive_date_time,
        None => return None,
    };

    println!(
        "Input download end time(UTC Time): ({})(Use start time instead if input nothing.)",
        DATE_FMT
    );
    let end_time = return_naive_date_time().unwrap_or_else(|| start_time); // if end_time is nothing, we will use the start time.

    let download_period = DownloadTime {
        start_time,
        end_time,
    };
    Some(download_period)
}

fn generate_download_time_list(
    original_time_period: &DownloadTime,
) -> Result<Vec<NaiveDateTime>, &str> {
    let mut start_time = original_time_period.start_time;
    let mut start_min = start_time
        .format("%M")
        .to_string()
        .parse::<u8>()
        .expect("Failed to parse start time.");

    const MAX_COUNT: u8 = 10;
    let mut count: u8 = 0;
    let correct_start_min = loop {
        if count >= MAX_COUNT {
            break None;
        } else if start_min % 10 == 0 {
            break Some(strip_seconds(start_time));
        } else {
            start_time = start_time + Duration::minutes(1);
            start_min += 1;
            count += 1
        }
    };

    match correct_start_min {
        Some(correct_start_min) => {
            let download_time_list: Vec<NaiveDateTime> =
                generate_ten_minute_intervals(correct_start_min, original_time_period.end_time);
            Ok(download_time_list)
        }
        None => Err("Failed to generate download time list."),
    }
}

fn strip_seconds(dt: NaiveDateTime) -> NaiveDateTime {
    dt.with_second(0)
        .and_then(|dt| dt.with_nanosecond(0))
        .unwrap()
}

fn generate_ten_minute_intervals(start: NaiveDateTime, end: NaiveDateTime) -> Vec<NaiveDateTime> {
    // 每 10 分钟一个间隔
    let step = Duration::minutes(TIME_STEP);
    // 计算总间隔数
    let total_minutes = (end - start).num_minutes();
    let count = (total_minutes / TIME_STEP) + 1; // 包含两端

    let mut times = Vec::with_capacity(count as usize);

    let mut current = start;
    while current <= end {
        times.push(current);
        current += step;
    }

    times
}
