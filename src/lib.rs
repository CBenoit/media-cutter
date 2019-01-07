use chrono::Duration;

pub mod processing;

#[macro_export]
macro_rules! message_dialog {
    ($win:ident, $type:path, $msg:expr) => {{
        let dialog = MessageDialog::new(
            Some(&$win),
            gtk::DialogFlags::MODAL,
            $type,
            gtk::ButtonsType::Ok,
            $msg,
        );
        dialog.run();
        dialog.destroy();
    }};
}

// upgrade weak reference or return
#[macro_export]
macro_rules! upgrade_weak {
    ($x:ident, $r:expr) => {{
        match $x.upgrade() {
            Some(o) => o,
            None => return $r,
        }
    }};
    ($x:ident) => {
        upgrade_weak!($x, ())
    };
}

// gtk utility: clone variables before moving them inside a clojure
#[macro_export]
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

#[macro_export]
macro_rules! get_widget {
    ($builder:ident, $name:expr) => {
        $builder
            .get_object($name)
            .expect(&format!("failed to get {} from builder", $name))
    };
}

fn duration_to_string(time: Duration) -> String {
    format!(
        "{}:{}:{}.{}",
        time.num_hours(),
        time.num_minutes() - time.num_hours() * 60,
        time.num_seconds() - time.num_minutes() * 60,
        time.num_milliseconds() - time.num_seconds() * 1000
    )
}

fn build_args_string<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|arg| format!(r#""{}""#, arg.as_ref()))
        .collect::<Vec<String>>()
        .join(" ")
}

pub struct Config {
    pub preview: bool,
    pub input_file: String,
    pub output_file: String,
    pub from_time: Duration,
    pub to_time: Duration,
    pub high_pass_filter: Option<u32>,
    pub low_pass_filter: Option<u32>,
    pub allow_overidde: bool,
    pub ignore_video: bool,
    pub ignore_audio: bool,
    pub peak_normalization: bool,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preview: false,
            input_file: String::from(""),
            output_file: String::from(""),
            from_time: Duration::seconds(0),
            to_time: Duration::seconds(0),
            high_pass_filter: None,
            low_pass_filter: None,
            allow_overidde: false,
            ignore_video: false,
            ignore_audio: false,
            peak_normalization: false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn convert_duration_to_string() {
        assert_eq!(duration_to_string(Duration::milliseconds(1002)), "0:0:1.2");
        assert_eq!(
            duration_to_string(Duration::milliseconds(65125)),
            "0:1:5.125"
        );
        assert_eq!(
            duration_to_string(Duration::milliseconds(6065125)),
            "1:41:5.125"
        );
        assert_eq!(duration_to_string(Duration::seconds(128)), "0:2:8.0");
    }

    #[test]
    fn build_args() {
        assert_eq!(
            build_args_string(&["-l", "-h", "a/path"]),
            r#""-l" "-h" "a/path""#
        );
    }
}
