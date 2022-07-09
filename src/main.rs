use std::fs;

use color_eyre::eyre::Result;
use glib::MainContext;
use gtk::prelude::*;
use gtk::{self, glib, Application, ApplicationWindow, ListBox};
use tokio::process::Command;
use tracing::{debug, info, trace, Level};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    trace!("Starting Pacstall GUI");

    // Prepare the URL to get the package list from
    let url = fs::read_to_string("/usr/share/pacstall/repo/pacstallrepo.txt")? + "/packagelist";
    trace!("URL read from pacstall repos");
    let url = url.trim();
    debug!("Parsed packagelist link: {}", url);

    // Get the packages string from the URL
    let packages = reqwest::get(url).await?.text().await?;
    debug!("Raw packages: {:#?}", packages);

    let app = Application::builder().build();
    trace!("Application built");

    app.connect_activate(move |app| {
        // Split the package list string into a vector
        let packages: Vec<&str> = packages.split('\n').collect();
        debug!("{:#?}", packages);

        let win = ApplicationWindow::builder()
            .application(app)
            .default_width(800)
            .default_height(600)
            .title("Pacstall GUI")
            .border_width(10)
            .build();

        win.set_position(gtk::WindowPosition::Center);

        let list_box = ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let header_bar = gtk::HeaderBar::builder().show_close_button(true).build();
        header_bar.set_title(Some("Pacstall GUI"));

        win.set_titlebar(Some(&header_bar));

        let scrolled_window = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();
        scrolled_window.add(&list_box);

        let search_entry = gtk::SearchEntry::builder().build();
        search_entry.connect_search_changed(
            glib::clone!(@weak list_box, @weak search_entry => move |_| {
                info!("New search entry");

                let text = search_entry.text();
                debug!("Search entry text: {:#?}", text);

                for child in &list_box.children() {
                    let list_box_row = child.downcast_ref::<gtk::ListBoxRow>().unwrap();
                    let list_box_row_child = list_box_row.child().unwrap();
                    let check_button = list_box_row_child.downcast_ref::<gtk::CheckButton>().unwrap();

                    let check_button_label = check_button.label().unwrap();
                    if check_button_label.contains(&text.as_str()) {
                        debug!("Check button shown: {:#?}", check_button_label);
                        child.show();
                    } else {
                        trace!("Check button hidden: {:#?}", check_button_label);
                        child.hide();
                    }
                }
            }),
        );


        let spinner = gtk::Spinner::builder().build();

        let install_button = gtk::Button::builder().label("Install").build();
        install_button.connect_clicked(glib::clone!(@weak list_box, @weak spinner => move |install_button| {
            info!("Install button clicked");

            install_button.set_sensitive(false);
            trace!("Install button desensitized");
            let main_context = MainContext::default();

            main_context.spawn_local(async move {
                trace!("Moved into async main context");

                spinner.start();
                trace!("Spinner started");
                let mut packages_to_install = Vec::new();

                for child in &list_box.children() {
                    let list_box_row = child.downcast_ref::<gtk::ListBoxRow>().unwrap();
                    let list_box_row_child = list_box_row.child().unwrap();
                    let check_button = list_box_row_child.downcast_ref::<gtk::CheckButton>().unwrap();

                    if check_button.is_active() {
                        let label = check_button.label().unwrap();
                        let label_string = label.to_string();

                        debug!("Label pushed to `packages_to_install` vector: {:#?}", label_string);
                        packages_to_install.push(label_string);

                        check_button.set_active(false);
                        trace!("Check button deactivated");
                    }
                }

                debug!("Installing {:#?}", packages_to_install);
                Command::new("pacstall")
                    .args(["-PI".to_owned()].into_iter().chain(packages_to_install))
                    .spawn()
                    .expect("Failed to install")
                    .wait()
                    .await
                    .unwrap();

                spinner.stop();
                trace!("Spinner stopped");
            });
        }
        install_button.set_sensitive(true);
        trace!("Install button resensitized");
    ));

        header_bar.pack_start(&spinner);
        header_bar.pack_start(&install_button);
        header_bar.pack_end(&search_entry);

        let box_layout = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        box_layout.pack_start(&scrolled_window, true, true, 0);

        win.add(&box_layout);

        for package in packages {
            let row = gtk::ListBoxRow::builder().build();
            let label = gtk::CheckButton::builder().label(package).build();
            row.add(&label);
            list_box.add(&row);
        }

        // Don't forget to make all widgets visible.
        win.show_all();
    });

    app.run();
    info!("Application run");
    Ok(())
}
