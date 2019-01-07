use std::cell::RefCell;
use std::env::args;
use std::rc::Rc;

use chrono::Duration;
use gdk_pixbuf::Pixbuf;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{MenuItemExt, MessageDialog};

use media_cutter::*;

fn main() {
    let application = gtk::Application::new(
        "eu.fried-world.media_cutter",
        gio::ApplicationFlags::empty(),
    )
    .expect("Initialization failed...");

    application.connect_startup(|app| {
        build_ui(app);
    });
    application.connect_activate(|_| {});

    application.run(&args().collect::<Vec<_>>());
}

macro_rules! message_dialog {
    ($win:ident, $type:path, $msg:expr) => {{
        let dialog = MessageDialog::new(
            Some(&$win),
            gtk::DialogFlags::MODAL,
            $type,
            gtk::ButtonsType::Ok,
            &$msg,
        );
        dialog.run();
        dialog.destroy();
    }};
}

pub fn build_ui(application: &gtk::Application) {
    println!(
        "=> Gtk {}.{} detected",
        gtk::get_major_version(),
        gtk::get_minor_version()
    );

    let glade_src = include_str!("../ui/main.glade");
    let builder = gtk::Builder::new_from_string(glade_src);

    let conf = Rc::new(RefCell::new(ffmpeg::Config::default()));

    let window: gtk::ApplicationWindow = builder
        .get_object("main_window")
        .expect("failed to get main_window from builder");
    window.set_application(application);

    let dialog: gtk::AboutDialog = builder
        .get_object("about_dialog")
        .expect("failed to get about_dialog from builder");

    let logo_buf = Pixbuf::new_from_inline(include_bytes!("../ui/logo.inline"), false).unwrap();
    dialog.set_logo(Some(&logo_buf));

    let quit_menu_item: gtk::MenuItem = builder
        .get_object("quit_menu_item")
        .expect("failed to get quit_menu_item from builder");
    let about_menu_item: gtk::MenuItem = builder
        .get_object("about_menu_item")
        .expect("failed to get about_menu_item from builder");

    let select_input_button: gtk::Button = builder
        .get_object("select_input_button")
        .expect("failed to get select_input_button from builder");
    let input_file_entry: gtk::Entry = builder
        .get_object("input_file_entry")
        .expect("failed to get input_file_entry from builder");
    let select_output_button: gtk::Button = builder
        .get_object("select_output_button")
        .expect("failed to get select_output_button from builder");
    let output_file_entry: gtk::Entry = builder
        .get_object("output_file_entry")
        .expect("failed to get output_file_entry from builder");

    let ignore_video_check: gtk::CheckButton = builder
        .get_object("ignore_video_check")
        .expect("failed to get ignore_video_check from builder");
    let ignore_audio_check: gtk::CheckButton = builder
        .get_object("ignore_audio_check")
        .expect("failed to get ignore_audio_check from builder");
    let peak_normalization_check: gtk::CheckButton = builder
        .get_object("peak_normalization_check")
        .expect("failed to get peak_normalization_check from builder");
    let overidde_existing_check: gtk::CheckButton = builder
        .get_object("overidde_check")
        .expect("failed to get overidde_check from builder");
    let high_pass_check: gtk::CheckButton = builder
        .get_object("high_pass_check")
        .expect("failed to get high_pass_check from builder");
    let low_pass_check: gtk::CheckButton = builder
        .get_object("low_pass_check")
        .expect("failed to get low_pass_check from builder");
    let noise_reduc_check: gtk::CheckButton = builder
        .get_object("noise_reduc_check")
        .expect("failed to get noise_reduc_check from builder");

    let noise_file_entry: gtk::Entry = builder
        .get_object("noise_file_entry")
        .expect("failed to get noise_file_entry from builder");
    let select_noise_button: gtk::Button = builder
        .get_object("select_noise_button")
        .expect("failed to get select_noise_button from builder");

    let high_pass_freq_adj: gtk::Adjustment = builder
        .get_object("high_pass_freq_adj")
        .expect("failed to get high_pass_freq_adj from builder");
    let low_pass_freq_adj: gtk::Adjustment = builder
        .get_object("low_pass_freq_adj")
        .expect("failed to get low_pass_freq_adj from builder");
    let start_secs_adj: gtk::Adjustment = builder
        .get_object("start_secs_adj")
        .expect("failed to get start_secs_adj from builder");
    let end_secs_adj: gtk::Adjustment = builder
        .get_object("end_secs_adj")
        .expect("failed to get end_secs_adj from builder");

    let process_button: gtk::Button = builder
        .get_object("process_button")
        .expect("failed to get process_button from builder");
    let preview_button: gtk::Button = builder
        .get_object("preview_button")
        .expect("failed to get preview_button from builder");

    let update_conf = Rc::new(clone!(conf,
                             input_file_entry,
                             output_file_entry,
                             start_secs_adj,
                             end_secs_adj,
                             ignore_audio_check,
                             ignore_video_check,
                             overidde_existing_check,
                             high_pass_check,
                             low_pass_check,
                             high_pass_freq_adj,
                             low_pass_freq_adj => move || {
        conf.borrow_mut().input_file = input_file_entry.get_text().unwrap();
        conf.borrow_mut().output_file = output_file_entry.get_text().unwrap();
        conf.borrow_mut().from_time = Duration::seconds(start_secs_adj.get_value() as i64);
        conf.borrow_mut().to_time = Duration::seconds(end_secs_adj.get_value() as i64);
        conf.borrow_mut().ignore_video = ignore_video_check.get_active();
        conf.borrow_mut().ignore_audio = ignore_audio_check.get_active();
        conf.borrow_mut().allow_overidde = overidde_existing_check.get_active();

        conf.borrow_mut().low_pass_filter = if low_pass_check.get_active() {
            Some(low_pass_freq_adj.get_value() as u32)
        } else {
            None
        };

        conf.borrow_mut().high_pass_filter = if high_pass_check.get_active() {
            Some(high_pass_freq_adj.get_value() as u32)
        } else {
            None
        };
    }));

    let window_weak = window.downgrade();
    let input_file_entry_weak = input_file_entry.downgrade();
    select_input_button.connect_clicked(move |_| {
        let window = upgrade_weak!(window_weak);
        let input_file_entry = upgrade_weak!(input_file_entry_weak);
        handle_select_file(window, input_file_entry, gtk::FileChooserAction::Open);
    });

    let window_weak = window.downgrade();
    let output_file_entry_weak = output_file_entry.downgrade();
    select_output_button.connect_clicked(move |_| {
        let window = upgrade_weak!(window_weak);
        let output_file_entry = upgrade_weak!(output_file_entry_weak);
        handle_select_file(window, output_file_entry, gtk::FileChooserAction::Save);
    });

    process_button.connect_clicked(
        clone!(input_file_entry, output_file_entry, window, conf, update_conf => move |_| {
            let mut errors: Vec<&str> = Vec::new();
            if input_file_entry.get_text().unwrap() == "" {
                errors.push("No input file specified");
            }
            if output_file_entry.get_text().unwrap() == "" {
                errors.push("No output file specified");
            }

            if !errors.is_empty() {
                message_dialog!(window, gtk::MessageType::Error, &errors.join("\n"));
                return;
            }

            update_conf();
            conf.borrow_mut().preview = false;

            match ffmpeg::run(&conf.borrow()) {
                Ok(_) => message_dialog!(window, gtk::MessageType::Info, "Operation suceeded!"),
                Err(e) => message_dialog!(window, gtk::MessageType::Error, &e),
            }
        }),
    );

    preview_button.connect_clicked(clone!(input_file_entry, window, conf => move |_| {
        let mut errors: Vec<&str> = Vec::new();
        if input_file_entry.get_text().unwrap() == "" {
            errors.push("No input file specified");
        }

        if !errors.is_empty() {
            message_dialog!(window, gtk::MessageType::Error, &errors.join("\n"));
            return;
        }

        update_conf();
        conf.borrow_mut().preview = true;

        match ffmpeg::run(&conf.borrow()) {
            Ok(_) => (),
            Err(e) => message_dialog!(window, gtk::MessageType::Error, &e),
        }
    }));

    quit_menu_item.connect_activate(clone!(window => move |_| {
        window.close();
    }));

    about_menu_item.connect_activate(move |_| {
        dialog.run();
        dialog.hide();
    });

    window.connect_delete_event(|win, _| {
        win.destroy();
        Inhibit(false)
    });

    window.show_all();
}

fn handle_select_file(
    window: gtk::ApplicationWindow,
    entry: gtk::Entry,
    dialog_action: gtk::FileChooserAction,
) {
    let file_chooser =
        gtk::FileChooserDialog::new(Some("Select File"), Some(&window), dialog_action);

    file_chooser.add_buttons(&[
        ("Select", gtk::ResponseType::Ok.into()),
        ("Cancel", gtk::ResponseType::Cancel.into()),
    ]);

    if file_chooser.run() == gtk::ResponseType::Ok.into() {
        let filename = file_chooser.get_filename().expect("couldn't get filename");
        entry.set_text(&filename.to_string_lossy());
    }

    file_chooser.destroy();
}
