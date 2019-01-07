use std::process::Command;
use std::process::Stdio;

use chrono::Duration;

use crate::{build_args_string, duration_to_string};

pub type Result<T> = std::result::Result<T, String>;

pub struct Config {
    pub input_file: String,
    pub output_file: String,
    pub from_time: Duration,
    pub to_time: Duration,
    pub preview: bool,
    pub allow_overidde: bool,
    pub ignore_video: bool,
    pub ignore_audio: bool,
    pub high_pass_filter: Option<u32>,
    pub low_pass_filter: Option<u32>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_file: String::from(""),
            output_file: String::from(""),
            from_time: Duration::seconds(0),
            to_time: Duration::seconds(0),
            preview: false,
            allow_overidde: false,
            ignore_video: false,
            ignore_audio: false,
            high_pass_filter: None,
            low_pass_filter: None,
        }
    }
}

pub fn run(conf: &Config) -> Result<()> {
    let args = make_args(conf)?;

    let command_name = if conf.preview { "ffplay" } else { "ffmpeg" };

    let output = Command::new(command_name)
        .args(&args[..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            format!(
                "Failed to start: {}\nCommand was: {} {}",
                err,
                command_name,
                build_args_string(&args[..])
            )
        })?;

    if output.status.success() {
        Ok(())
    } else {
        match output.status.code() {
            Some(code) => Err(format!(
                "⚠ {} exited with non-zero status code: {}\nArguments were: {}\n\nError output: {}",
                command_name,
                code,
                build_args_string(&args[..]),
                String::from_utf8_lossy(&output.stderr)
            )),
            None => Err(format!("⚠ {} terminated by signal", command_name)),
        }
    }
}

pub fn make_args(conf: &Config) -> Result<Vec<String>> {
    let duration = conf.to_time - conf.from_time;

    let mut args = Vec::new();

    if !conf.preview {
        if conf.allow_overidde {
            args.push(String::from("-y"));
        } else {
            args.push(String::from("-nostdin"));
        }
    }

    args.push(String::from("-i"));
    args.push(conf.input_file.clone());

    if conf.ignore_video {
        args.push(String::from("-vn"));
    }

    if conf.ignore_audio {
        args.push(String::from("-an"));
    }

    args.push(String::from("-ss"));
    args.push(duration_to_string(conf.from_time));
    args.push(String::from("-t"));
    args.push(duration_to_string(duration));

    if conf.high_pass_filter.is_some() || conf.low_pass_filter.is_some() {
        args.push(String::from("-filter:a"));

        match (conf.high_pass_filter, conf.low_pass_filter) {
            (Some(high), Some(low)) => args.push(format!("highpass=f={}, lowpass=f={}", high, low)),
            (Some(high), None) => args.push(format!("highpass=f={}", high)),
            (None, Some(low)) => args.push(format!("lowpass=f={}", low)),
            (None, None) => (),
        }
    }

    if !conf.preview {
        args.push(conf.output_file.clone());
    }

    Ok(args)
}
