
![Media Cutter's logo](./ui/logo.png)

# Media Cutter

Licensed under the MIT license.

![Media Cutter's main screen](https://i.imgur.com/saZrF5e.png)

[ffmpeg](https://www.ffmpeg.org/) is used for most audio and video processing.
[SoX](http://sox.sourceforge.net/) is an optional tool used for noise reduction filter.

I originally made this software to easily cut audio files
using ffmpeg as well as applying some filters quickly.
However, it can also process video files without further modification thanks to `ffmpeg`
hence "Media Cutter" instead of "Audio Cutter".

Also, I wanted to try some graphical libraries in Rust.
In this case, I had a quite good experience using [gtk-rs](https://gtk-rs.org/) despite
the boiler-plate code required to handle ownership and borrowing in the numerous clojures.

## Dependencies

- gtk 3.10
- ffmpeg (tested with v4.1, but older might works perfectly as well. No new fancy feature is used to my knowledge)
- **Optional**: sox (tested with v14.4, but again, older might works perfectly)

Present software was used and tested only on *Linux*.
It might works on Windows too, but since aformentioned tools are directly used
by command line you might get some `command not found` troubles.
Hint: make sure executables are accessibles using the %PATH%.
I might add a way to manually specify path to executables later though.

## Building

Thanks to the Rust package manager, `cargo`, building is as simple as executing

```
$ cargo build --release
```

inside the project directory.

It will build a self-contained executable `media_cutter` inside `target/release/` folder.

If you need to install the rust compiler and cargo, see [rustup](https://github.com/rust-lang/rustup.rs) for an easy install.

*Additional step*: you can use the [strip](https://sourceware.org/binutils/docs/binutils/strip.html)
command to discard object files from the executable and get its size under 1M.

```
$ strip target/release/media_cutter
```

## Pre-built binaries

*Not yet available*.

## Installing

On *Linux*, simply copy/move the executable in a folder present in the `$PATH` environment variable.
`/usr/local/bin` is a good choice for manually installed (installed without package manager) softwares.

## TODO

- open error dialog on error command return status.
- checkbutton to enable forced overidde of existing file
- peak normalization
- noise reduction with sox

