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

pub fn build_ui(application: &gtk::Application) {
    println!(
        "=> Gtk {}.{} detected",
        gtk::get_major_version(),
        gtk::get_minor_version()
    );

    let glade_src = include_str!("../ui/main.glade");
    let builder = gtk::Builder::new_from_string(glade_src);

    let conf = Rc::new(RefCell::new(Config::default()));

    let window: gtk::ApplicationWindow = get_widget!(builder, "main_window");
    window.set_application(application);

    let dialog: gtk::AboutDialog = get_widget!(builder, "about_dialog");

    let logo_buf = Pixbuf::new_from_inline(include_bytes!("../ui/logo.inline"), false).unwrap();
    dialog.set_logo(Some(&logo_buf));

    let quit_menu_item: gtk::MenuItem = get_widget!(builder, "quit_menu_item");
    let about_menu_item: gtk::MenuItem = get_widget!(builder, "about_menu_item");

    let select_input_button: gtk::Button = get_widget!(builder, "select_input_button");
    let input_file_entry: gtk::Entry = get_widget!(builder, "input_file_entry");
    let select_output_button: gtk::Button = get_widget!(builder, "select_output_button");
    let output_file_entry: gtk::Entry = get_widget!(builder, "output_file_entry");

    let ignore_video_check: gtk::CheckButton = get_widget!(builder, "ignore_video_check");
    let ignore_audio_check: gtk::CheckButton = get_widget!(builder, "ignore_audio_check");
    let peak_normalization_check: gtk::CheckButton =
        get_widget!(builder, "peak_normalization_check");
    let overidde_existing_check: gtk::CheckButton = get_widget!(builder, "overidde_check");
    let high_pass_check: gtk::CheckButton = get_widget!(builder, "high_pass_check");
    let low_pass_check: gtk::CheckButton = get_widget!(builder, "low_pass_check");
    let noise_reduc_check: gtk::CheckButton = get_widget!(builder, "noise_reduc_check");

    let noise_file_entry: gtk::Entry = get_widget!(builder, "noise_file_entry");
    let select_noise_button: gtk::Button = get_widget!(builder, "select_noise_button");

    let high_pass_freq_adj: gtk::Adjustment = get_widget!(builder, "high_pass_freq_adj");
    let low_pass_freq_adj: gtk::Adjustment = get_widget!(builder, "low_pass_freq_adj");
    let start_secs_adj: gtk::Adjustment = get_widget!(builder, "start_secs_adj");
    let end_secs_adj: gtk::Adjustment = get_widget!(builder, "end_secs_adj");
    let volume_adj: gtk::Adjustment = get_widget!(builder, "volume_adj");
    let sox_amount_adj: gtk::Adjustment = get_widget!(builder, "sox_amount_adj");

    let process_button: gtk::Button = get_widget!(builder, "process_button");
    let preview_button: gtk::Button = get_widget!(builder, "preview_button");

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
                             low_pass_freq_adj,
                             peak_normalization_check,
                             noise_file_entry => move || {
        conf.borrow_mut().input_file = input_file_entry.get_text().unwrap();
        conf.borrow_mut().output_file = output_file_entry.get_text().unwrap();
        conf.borrow_mut().from_time = Duration::milliseconds((start_secs_adj.get_value() * 1000.0) as i64);
        conf.borrow_mut().to_time = Duration::milliseconds((end_secs_adj.get_value() * 1000.0) as i64);
        conf.borrow_mut().ignore_video = ignore_video_check.get_active();
        conf.borrow_mut().ignore_audio = ignore_audio_check.get_active();
        conf.borrow_mut().allow_overidde = overidde_existing_check.get_active();
        conf.borrow_mut().peak_normalization = peak_normalization_check.get_active();
        conf.borrow_mut().volume_change = volume_adj.get_value();

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

        if noise_reduc_check.get_active() {
            conf.borrow_mut().noise_profile_file = Some(noise_file_entry.get_text().unwrap());
            conf.borrow_mut().noise_reduction_amount = Some(sox_amount_adj.get_value());
        } else {
            conf.borrow_mut().noise_profile_file = None;
            conf.borrow_mut().noise_reduction_amount = None;
        }
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

    let window_weak = window.downgrade();
    let noise_file_entry_weak = noise_file_entry.downgrade();
    select_noise_button.connect_clicked(move |_| {
        let window = upgrade_weak!(window_weak);
        let noise_file_entry = upgrade_weak!(noise_file_entry_weak);
        handle_select_file(window, noise_file_entry, gtk::FileChooserAction::Open);
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

            match processing::run(&conf.borrow()) {
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

        match processing::run(&conf.borrow()) {
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
