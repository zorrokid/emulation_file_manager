use std::{collections::HashMap, sync::Arc};

use core_types::SettingName;
use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self, glib,
        prelude::{
            BoxExt, ButtonExt, CheckButtonExt, EditableExt, EntryBufferExtManual, EntryExt,
            GtkWindowExt, OrientableExt, WidgetExt,
        },
    },
};
use service::view_models::Settings;

#[derive(Debug)]
pub struct SettingsForm {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,

    // settings fields (we are not modifying directly the settings object)
    // once changed settings have been saved, we emit a SettingsChanged message
    // so that the main application can reload settings and update components accordingly
    pub s3_bucket_name: String,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_sync_enabled: bool,
}

pub struct SettingsFormInit {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub enum SettingsFormOutputMsg {
    SettingsChanged,
}

#[derive(Debug)]
pub enum SettingsFormMsg {
    Submit,
    Show,
    Hide,
    S3FileSyncToggled,
    S3BucketNameChanged(String),
    S3EndpointChanged(String),
    S3RegionChanged(String),
}

#[derive(Debug)]
pub enum SettingsFormCommandMsg {
    SettingsSaved,
}

#[relm4::component(pub)]
impl Component for SettingsForm {
    type Init = SettingsFormInit;
    type Input = SettingsFormMsg;
    type Output = SettingsFormOutputMsg;
    type CommandOutput = SettingsFormCommandMsg;

    view! {
        gtk::Window {
            set_title: Some("Settings"),

            connect_close_request[sender] => move|_| {
                sender.input(SettingsFormMsg::Hide);
                glib::Propagation::Stop
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Label {
                    set_label: "Cloud Storage Settings",
                    set_xalign: 0.0,
                    set_margin_bottom: 10,
                },

                gtk::CheckButton {
                    set_label: Some("Enable S3 File Sync"),
                    #[watch]
                    #[block_signal(extract_files_toggled)]
                    set_active: model.s3_sync_enabled,
                    connect_toggled[sender] => move |_| {
                        sender.input(SettingsFormMsg::S3FileSyncToggled);
                    } @extract_files_toggled,
                },

                gtk::Label {
                    set_label: "In addition to these settings, export the following environment variables:\n- AWS_ACCESS_KEY_ID\n- AWS_SECRET_ACCESS for optional cloud storage access",
                    set_xalign: 0.0,
                    set_margin_bottom: 10,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "S3 Bucket name",
                    },

                    #[name = "s3_bucket_name_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("S3 Bucket Name"),
                        set_text: &model.s3_bucket_name,
                        connect_changed[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(SettingsFormMsg::S3BucketNameChanged(buffer.text().into()));
                        },
                    },

                 },


                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "S3 endpoint URL",
                    },

                    #[name = "s3_endpoint_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("S3 Endpoint"),
                        set_text: &model.s3_endpoint,
                        connect_changed[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(SettingsFormMsg::S3EndpointChanged(buffer.text().into()));
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    gtk::Label {
                        set_label: "S3 region",
                    },

                    #[name = "s3_region_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("S3 Region"),
                        set_text: &model.s3_region,
                        connect_changed[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(SettingsFormMsg::S3RegionChanged(buffer.text().into()));
                        },
                    },
                },
                gtk::Button {
                    set_label: "Save Settings",
                    connect_clicked => SettingsFormMsg::Submit,
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let s3_settings = init.settings.s3_settings.clone().unwrap_or_default();
        let model = Self {
            s3_bucket_name: s3_settings.bucket.clone(),
            s3_endpoint: s3_settings.endpoint.clone(),
            s3_region: s3_settings.region.clone(),
            s3_sync_enabled: init.settings.s3_sync_enabled,
            repository_manager: init.repository_manager,
            settings: init.settings,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            SettingsFormMsg::S3FileSyncToggled => {
                self.s3_sync_enabled = !self.s3_sync_enabled;
            }
            SettingsFormMsg::S3BucketNameChanged(name) => {
                self.s3_bucket_name = name;
            }
            SettingsFormMsg::S3EndpointChanged(endpoint) => {
                self.s3_endpoint = endpoint;
            }
            SettingsFormMsg::S3RegionChanged(region) => {
                self.s3_region = region;
            }
            SettingsFormMsg::Submit => {
                let repo = Arc::clone(&self.repository_manager);
                let settings_map = HashMap::from([
                    (SettingName::S3Bucket, self.s3_bucket_name.clone()),
                    (SettingName::S3EndPoint, self.s3_endpoint.clone()),
                    (SettingName::S3Region, self.s3_region.clone()),
                    (
                        SettingName::S3FileSyncEnabled,
                        if self.s3_sync_enabled {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        },
                    ),
                ]);
                sender.oneshot_command(async move {
                    if let Err(e) = repo
                        .get_settings_repository()
                        .add_or_update_settings(&settings_map)
                        .await
                    {
                        eprintln!("Error saving S3 bucket name: {}", e);
                    }
                    SettingsFormCommandMsg::SettingsSaved
                });
            }
            SettingsFormMsg::Show => {
                root.show();
            }
            SettingsFormMsg::Hide => {
                root.hide();
            }
        }

        // This is essential:
        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            SettingsFormCommandMsg::SettingsSaved => {
                // notify main application that settings have changed
                let res = sender.output(SettingsFormOutputMsg::SettingsChanged);
                if res.is_err() {
                    eprintln!("Error sending SettingsChanged message");
                }
                root.hide();
            }
        }
    }
}
