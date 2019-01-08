use std::process::{Command, Output, Stdio};
use std::str::from_utf8;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{build_args_string, duration_to_string, Config};

pub type Result<T> = std::result::Result<T, String>;

lazy_static! {
    static ref MAX_VOLUME_RE: Regex =
        Regex::new(r#"max_volume:\s*(?P<max>-?[0-9\.]+)\s*dB"#).unwrap();
}

pub fn run(conf: &Config) -> Result<()> {
    let mut state = State::default();
    if conf.peak_normalization {
        let args = make_detect_max_volume_args(conf);
        let output = run_command_and_get_output("ffmpeg", &args)?;
        if let Some(caps) = MAX_VOLUME_RE.captures(from_utf8(&output.stderr).unwrap()) {
            // pattern matched by the regex should be parsable into f64, hence unwrap.
            state.max_volume_db = Some(caps["max"].parse::<f64>().unwrap());
        }
    }

    let command_name = if conf.preview { "ffplay" } else { "ffmpeg" };
    let args = make_processing_args(conf, &state);
    let output = run_command_and_get_output(command_name, &args)?;
    if output.status.success() {
        Ok(())
    } else {
        match output.status.code() {
            Some(code) => Err(format!(
                "⚠ {} exited with non-zero status code: {}\nArguments were: {}\n\nError output: {}",
                command_name,
                code,
                build_args_string(&args),
                String::from_utf8_lossy(&output.stderr)
            )),
            None => Err(format!("⚠ {} terminated by signal", command_name)),
        }
    }
}

struct State {
    max_volume_db: Option<f64>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            max_volume_db: None,
        }
    }
}

fn run_command_and_get_output(command_name: &str, args: &Vec<String>) -> Result<Output> {
    Command::new(command_name)
        .args(&args[..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            format!(
                "Failed to start: {}\nCommand was: {} {}",
                err,
                command_name,
                build_args_string(&args[..])
            )
        })
}

fn make_detect_max_volume_args(conf: &Config) -> Vec<String> {
    let mut args = Vec::new();

    args.push(String::from("-nostdin"));

    args.push(String::from("-i"));
    args.push(conf.input_file.clone());

    args.push(String::from("-vn"));

    let duration = conf.to_time - conf.from_time;
    args.push(String::from("-ss"));
    args.push(duration_to_string(conf.from_time));
    args.push(String::from("-t"));
    args.push(duration_to_string(duration));

    args.push(String::from("-filter:a"));
    args.push(String::from("volumedetect"));

    // no output file
    args.push(String::from("-f"));
    args.push(String::from("null"));
    args.push(String::from("-"));

    args
}

fn make_processing_args(conf: &Config, state: &State) -> Vec<String> {
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

    let duration = conf.to_time - conf.from_time;
    args.push(String::from("-ss"));
    args.push(duration_to_string(conf.from_time));
    args.push(String::from("-t"));
    args.push(duration_to_string(duration));

    // == filters
    args.push(String::from("-af")); // alias of -filter:a with ffmpeg but not with ffplay.

    let mut filters = Vec::with_capacity(3);
    if let Some(high) = conf.high_pass_filter {
        filters.push(format!("highpass=f={}", high));
    }
    if let Some(low) = conf.low_pass_filter {
        filters.push(format!("lowpass=f={}", low));
    }

    let volume_filter = if let Some(max_volume_db) = state.max_volume_db {
        // peak normalization
        conf.volume_change - max_volume_db
    } else {
        conf.volume_change
    };
    filters.push(format!("volume={}dB", volume_filter));

    args.push(filters.join(","));
    // == end filters

    if !conf.preview {
        args.push(conf.output_file.clone());
    }

    args
}
