use std::{
    env,
    fs::create_dir_all,
    path::PathBuf,
    process::{Command, Output, Stdio},
    str::from_utf8,
};

use lazy_static::lazy_static;
use regex::Regex;

use crate::{build_args_string, duration_to_string, Config};

pub type Result<T> = std::result::Result<T, String>;

lazy_static! {
    static ref MAX_VOLUME_RE: Regex =
        Regex::new(r#"max_volume:\s*(?P<max>-?[0-9\.]+)\s*dB"#).unwrap();
}

const FFMPEG_COMMAND: &'static str = "ffmpeg";
const FFPLAY_COMMAND: &'static str = "ffplay";
const SOX_COMMAND: &'static str = "sox";

pub fn run(conf: &Config) -> Result<()> {
    let mut state = State::default();

    if conf.noise_profile_file.is_some() && conf.noise_reduction_amount.is_some() {
        let sox_noise_profile_args = make_sox_generate_noiseprof_args(conf)?;
        let sox_clean_noise_args = make_sox_clean_noise_args(conf, &mut state)?;
        if let Some(ref tmp_file_output) = state.sox_temporary_file_output {
            create_dir_all(tmp_file_output.parent().ok_or_else(|| {
                String::from("Unexpected error: couldn't create temporary directory")
            })?)
            .map_err(|e| format!("Could not create temporary directory.\nError: {}", e))?;

            let child = command_map_error(
                Command::new(SOX_COMMAND)
                    .args(&sox_noise_profile_args[..])
                    .stdout(Stdio::piped())
                    .spawn(),
                SOX_COMMAND,
                &sox_noise_profile_args,
            )?;

            let sox_output = command_map_error(
                Command::new(SOX_COMMAND)
                    .args(&sox_clean_noise_args[..])
                    .stdin(child.stdout.unwrap())
                    .output(),
                SOX_COMMAND,
                &sox_clean_noise_args,
            )?;

            output_map_error(&sox_output, SOX_COMMAND, &sox_clean_noise_args)?;
        }
    }

    if conf.peak_normalization {
        let args = make_ffmpeg_detect_max_volume_args(conf);
        let output = run_command_and_get_output(FFMPEG_COMMAND, &args)?;
        output_map_error(&output, FFMPEG_COMMAND, &args)?;
        if let Some(caps) = MAX_VOLUME_RE.captures(from_utf8(&output.stderr).unwrap()) {
            // pattern matched by the regex should be parsable into f64, hence unwrap.
            state.max_volume_db = Some(caps["max"].parse::<f64>().unwrap());
        }
    }

    let command_name = if conf.preview { FFPLAY_COMMAND } else { FFMPEG_COMMAND };
    let args = make_ffmpeg_processing_args(conf, &state);
    let output = run_command_and_get_output(command_name, &args)?;
    output_map_error(&output, command_name, &args)
}

struct State {
    max_volume_db: Option<f64>,
    sox_temporary_file_output: Option<PathBuf>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            max_volume_db: None,
            sox_temporary_file_output: None,
        }
    }
}

fn command_map_error<T, E>(
    result: std::result::Result<T, E>,
    command_name: &str,
    args: &Vec<String>,
) -> Result<T>
where
    E: std::fmt::Display,
{
    result.map_err(|err| {
        format!(
            "Failed to start: {}\nCommand was: {} {}",
            err,
            command_name,
            build_args_string(&args[..])
        )
    })
}

fn output_map_error(output: &Output, command_name: &str, args: &Vec<String>) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        match output.status.code() {
            Some(code) => Err(format!(
                "⚠ {} exited with non-zero status code: {}\n\nArguments were: {}\n\nError output: {}",
                command_name,
                code,
                build_args_string(args),
                String::from_utf8_lossy(&output.stderr)
            )),
            None => Err(format!("⚠ {} terminated by signal", command_name)),
        }
    }
}

fn run_command_and_get_output(command_name: &str, args: &Vec<String>) -> Result<Output> {
    command_map_error(
        Command::new(command_name).args(&args[..]).output(),
        command_name,
        args,
    )
}

fn make_sox_generate_noiseprof_args(conf: &Config) -> Result<Vec<String>> {
    let mut args = Vec::with_capacity(3);
    match conf.noise_profile_file {
        Some(ref filename) => {
            if filename == "" {
                return Err(String::from("Error: no noise file provided."));
            } else {
                args.push(filename.clone()); // input noise file
            }
        }
        None => {
            return Err(String::from(
                "Unexpected error: could not build noise profile generation command.",
            ));
        }
    }
    args.push(String::from("-n"));
    args.push(String::from("noiseprof"));
    Ok(args)
}

fn make_sox_clean_noise_args(conf: &Config, state: &mut State) -> Result<Vec<String>> {
    let mut dir = env::temp_dir();
    dir.push("media_cutter");
    let input_file_pathbuf = PathBuf::from(conf.input_file.clone());
    match input_file_pathbuf.file_name() {
        Some(filename) => dir.push(filename),
        None => {
            return Err(String::from("Error: no input file provided."));
        }
    }
    let output_file = dir.to_string_lossy().into_owned();

    state.sox_temporary_file_output = Some(dir);

    let mut args = Vec::with_capacity(5);
    args.push(conf.input_file.clone()); // input file
    args.push(output_file); // output file
    args.push(String::from("noisered"));
    args.push(String::from("-")); // take noise profile from stdin
    match conf.noise_reduction_amount {
        Some(amount) => args.push(amount.to_string()),
        None => {
            return Err(String::from(
                "Unexpected error: could not build noise cleaning command.",
            ));
        }
    }
    Ok(args)
}

fn make_ffmpeg_detect_max_volume_args(conf: &Config) -> Vec<String> {
    let mut args = Vec::with_capacity(15);

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

fn make_ffmpeg_processing_args(conf: &Config, state: &State) -> Vec<String> {
    let mut args = Vec::with_capacity(15);

    if !conf.preview {
        if conf.allow_overidde {
            args.push(String::from("-y"));
        } else {
            args.push(String::from("-nostdin"));
        }
    }

    args.push(String::from("-i"));
    if let Some(ref sox_output_file) = state.sox_temporary_file_output {
        // use sox output file if applicable
        args.push(sox_output_file.to_string_lossy().into_owned());
    } else {
        args.push(conf.input_file.clone());
    }

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
