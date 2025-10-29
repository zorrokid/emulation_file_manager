use std::sync::Arc;

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
use service::{
    error::Error,
    settings_service::{SettingsSaveModel, SettingsService},
    view_models::Settings,
};

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
    pub s3_access_key_id: String,
    pub s3_secret_access_key: String,

    pub settings_service: Arc<SettingsService>,
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
    S3AccessKeyChanged(String),
    S3SecretKeyChanged(String),
}

#[derive(Debug)]
pub enum SettingsFormCommandMsg {
    SettingsSaved(Result<(), Error>),
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

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    gtk::Label {
                        set_label: "S3 Access Key ID",
                    },

                    #[name = "s3_acces_key_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("S3 Access Key ID"),
                        set_text: &model.s3_access_key_id,
                        connect_changed[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(SettingsFormMsg::S3AccessKeyChanged(buffer.text().into()));
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    gtk::Label {
                        set_label: "S3 Secret Access Key",
                    },

                    #[name = "s3_secret_key_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("S3 Secret Access Key"),
                        set_text: &model.s3_secret_access_key,
                        connect_changed[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(SettingsFormMsg::S3SecretKeyChanged(buffer.text().into()));
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
        let settings_service = Arc::new(SettingsService::new(Arc::clone(&init.repository_manager)));
        let model = Self {
            s3_bucket_name: s3_settings.bucket.clone(),
            s3_endpoint: s3_settings.endpoint.clone(),
            s3_region: s3_settings.region.clone(),
            s3_access_key_id: String::new(),
            s3_secret_access_key: String::new(),
            s3_sync_enabled: init.settings.s3_sync_enabled,
            repository_manager: init.repository_manager,
            settings: init.settings,
            settings_service,
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
            SettingsFormMsg::S3AccessKeyChanged(access_key) => {
                self.s3_access_key_id = access_key;
            }
            SettingsFormMsg::S3SecretKeyChanged(secret_key) => {
                self.s3_secret_access_key = secret_key;
            }
            SettingsFormMsg::Submit => {
                let settings_service = Arc::clone(&self.settings_service);

                let settings = SettingsSaveModel {
                    bucket: self.s3_bucket_name.clone(),
                    endpoint: self.s3_endpoint.clone(),
                    region: self.s3_region.clone(),
                    sync_enabled: self.s3_sync_enabled,
                    access_key_id: self.s3_access_key_id.clone(),
                    secret_access_key: self.s3_secret_access_key.clone(),
                };

                sender.oneshot_command(async move {
                    let res = settings_service.save_settings(settings).await;
                    SettingsFormCommandMsg::SettingsSaved(res)
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
            SettingsFormCommandMsg::SettingsSaved(Ok(())) => {
                // notify main application that settings have changed
                let res = sender.output(SettingsFormOutputMsg::SettingsChanged);
                if res.is_err() {
                    eprintln!("Error sending SettingsChanged message");
                }
                root.hide();
            }
            SettingsFormCommandMsg::SettingsSaved(Err(e)) => {
                eprintln!("Error saving settings: {}", e);
            }
        }
    }
}
