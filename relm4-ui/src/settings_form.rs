use std::{path::PathBuf, sync::Arc};

use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self, FileChooserDialog,
        gio::prelude::FileExt,
        glib::{self, clone},
        prelude::{
            BoxExt, ButtonExt, CheckButtonExt, DialogExt, EditableExt, EntryBufferExtManual,
            EntryExt, FileChooserExt as _, GtkWindowExt, OrientableExt, WidgetExt,
        },
    },
};
use service::{error::Error, settings_service::SettingsSaveModel, view_models::Settings};

use crate::utils::dialog_utils::show_error_dialog;

#[derive(Debug)]
pub struct SettingsForm {
    // settings fields (we are not modifying directly the settings object)
    // once changed settings have been saved, we emit a SettingsChanged message
    // so that the main application can reload settings and update components accordingly
    pub collection_root_dir: Option<PathBuf>,
    pub s3_bucket_name: String,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_sync_enabled: bool,
    pub s3_access_key_id: String,
    pub s3_secret_access_key: String,

    // Credential status indicator
    pub credentials_stored: bool,
    pub stored_access_key_preview: Option<String>,

    pub app_services: Arc<service::app_services::AppServices>,
}

impl SettingsForm {
    /// Helper function to create a preview of the access key ID for display
    fn format_access_key_preview(access_key_id: &str) -> String {
        if access_key_id.len() >= 8 {
            format!(
                "{}...{}",
                &access_key_id[..4],
                &access_key_id[access_key_id.len() - 4..]
            )
        } else {
            "****".to_string()
        }
    }
}

pub struct SettingsFormInit {
    pub app_services: Arc<service::app_services::AppServices>,
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
    ClearCredentials,
    S3FileSyncToggled,
    S3BucketNameChanged(String),
    S3EndpointChanged(String),
    S3RegionChanged(String),
    S3AccessKeyChanged(String),
    S3SecretKeyChanged(String),
    LoadCredentialStatus,
    BrowseCollectionRootDir,
    CollectionRootDirSelected(std::path::PathBuf),
}

#[derive(Debug)]
pub enum SettingsFormCommandMsg {
    SettingsSaved {
        result: Result<(), Error>,
        credentials_stored: bool,
        stored_key_preview: Option<String>,
    },
    CredentialStatusLoaded {
        credentials_stored: bool,
        stored_key_preview: Option<String>,
    },
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
                    set_label: "Core settings",
                    set_xalign: 0.0,
                    set_margin_bottom: 10,
                },

                #[name = "collection_root_dir_entry"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    set_margin_bottom: 10,
                    gtk::Label {
                        set_label: "Collection Root Directory",
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &model.collection_root_dir.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "Not set".to_string()),
                    },
                    gtk::Button {
                        set_label: "Browse",
                        connect_clicked => SettingsFormMsg::BrowseCollectionRootDir,
                    },
                },


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
                    set_label: "Credentials are stored securely in your system keyring.\nLeave fields empty to use AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables.",
                    set_xalign: 0.0,
                    set_margin_bottom: 10,
                    add_css_class: "dim-label",
                },

                #[name = "credentials_status_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    set_margin_bottom: 10,

                    #[name = "credentials_status_icon"]
                    gtk::Label {
                        #[watch]
                        set_label: if model.credentials_stored { "✓" } else { "⚠" },
                        #[watch]
                        add_css_class: if model.credentials_stored { "success" } else { "warning" },
                    },

                    #[name = "credentials_status_label"]
                    gtk::Label {
                        #[watch]
                        set_label: &if model.credentials_stored {
                            if let Some(ref preview) = model.stored_access_key_preview {
                                format!("Credentials stored ({})", preview)
                            } else {
                                "Credentials stored".to_string()
                            }
                        } else {
                            "No credentials stored".to_string()
                        },
                        set_xalign: 0.0,
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "S3 Bucket name",
                    },

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

                    #[name = "s3_access_key_entry"]
                    gtk::PasswordEntry {
                        set_placeholder_text: Some("S3 Access Key ID"),
                        set_text: &model.s3_access_key_id,
                        set_show_peek_icon: true,  // Allow user to peek at the value if needed
                        connect_changed[sender] => move |entry| {
                            sender.input(SettingsFormMsg::S3AccessKeyChanged(entry.text().into()));
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
                    gtk::PasswordEntry {
                        set_placeholder_text: Some("S3 Secret Access Key"),
                        set_text: &model.s3_secret_access_key,
                        set_show_peek_icon: true,  // Allow user to peek at the value if needed
                        connect_changed[sender] => move |entry| {
                            sender.input(SettingsFormMsg::S3SecretKeyChanged(entry.text().into()));
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Button {
                        set_label: "Save Settings",
                        connect_clicked => SettingsFormMsg::Submit,
                    },

                    gtk::Button {
                        set_label: "Clear Stored Credentials",
                        add_css_class: "destructive-action",
                        connect_clicked => SettingsFormMsg::ClearCredentials,
                    },
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
            s3_access_key_id: String::new(),
            s3_secret_access_key: String::new(),
            s3_sync_enabled: init.settings.s3_sync_enabled,
            credentials_stored: false,
            stored_access_key_preview: None,
            collection_root_dir: Some(init.settings.collection_root_dir.clone()),
            app_services: init.app_services,
        };
        let widgets = view_output!();

        // Load credential status on initialization
        sender.input(SettingsFormMsg::LoadCredentialStatus);

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
            SettingsFormMsg::BrowseCollectionRootDir => {
                self.select_collection_root_dir(root, &sender);
            }
            SettingsFormMsg::CollectionRootDirSelected(path) => {
                self.collection_root_dir = Some(path);
            }
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
            SettingsFormMsg::ClearCredentials => {
                // Clear the form fields in model
                self.s3_access_key_id.clear();
                self.s3_secret_access_key.clear();

                // Clear the UI widgets
                widgets.s3_access_key_entry.set_text("");
                widgets.s3_secret_key_entry.set_text("");

                // Delete from keyring
                let app_services = self.app_services.clone();
                sender.oneshot_command(async move {
                    let result = app_services.settings().delete_credentials().await;
                    if let Err(ref e) = result {
                        tracing::error!(error = ?e, "Error deleting credentials");
                    }
                    // Return status update: no credentials stored after deletion
                    SettingsFormCommandMsg::SettingsSaved {
                        result,
                        credentials_stored: false,
                        stored_key_preview: None,
                    }
                });
            }
            SettingsFormMsg::Submit => {
                let app_services = Arc::clone(&self.app_services);

                let settings = SettingsSaveModel {
                    bucket: self.s3_bucket_name.clone(),
                    endpoint: self.s3_endpoint.clone(),
                    region: self.s3_region.clone(),
                    sync_enabled: self.s3_sync_enabled,
                    access_key_id: self.s3_access_key_id.clone(),
                    secret_access_key: self.s3_secret_access_key.clone(),
                    collection_root_dir: self.collection_root_dir.clone(),
                };

                sender.oneshot_command(async move {
                    let save_result = app_services.settings().save_settings(settings).await;

                    // Check credential status after save attempt
                    let (credentials_stored, stored_key_preview) =
                        match app_services.settings().load_credentials().await {
                            Ok(Some(creds)) => {
                                let preview = Self::format_access_key_preview(&creds.access_key_id);
                                (true, Some(preview))
                            }
                            _ => (false, None),
                        };

                    SettingsFormCommandMsg::SettingsSaved {
                        result: save_result,
                        credentials_stored,
                        stored_key_preview,
                    }
                });
            }
            SettingsFormMsg::LoadCredentialStatus => {
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    match app_services.settings().load_credentials().await {
                        Ok(Some(creds)) => {
                            let preview = Self::format_access_key_preview(&creds.access_key_id);
                            SettingsFormCommandMsg::CredentialStatusLoaded {
                                credentials_stored: true,
                                stored_key_preview: Some(preview),
                            }
                        }
                        _ => SettingsFormCommandMsg::CredentialStatusLoaded {
                            credentials_stored: false,
                            stored_key_preview: None,
                        },
                    }
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
            SettingsFormCommandMsg::SettingsSaved {
                result,
                credentials_stored,
                stored_key_preview,
            } => {
                // Update credential status
                self.credentials_stored = credentials_stored;
                self.stored_access_key_preview = stored_key_preview;

                match result {
                    Ok(()) => {
                        // notify main application that settings have changed
                        sender
                            .output(SettingsFormOutputMsg::SettingsChanged)
                            .unwrap_or_else(|e| {
                                tracing::error!(error = ?e,
                                "Error sending SettingsChanged message")
                            });
                        root.hide();
                    }
                    Err(e) => {
                        tracing::error!(error = ?e, "Error saving settings");
                        show_error_dialog(format!("Error saving settings: {}", e), root);
                    }
                }
            }
            SettingsFormCommandMsg::CredentialStatusLoaded {
                credentials_stored,
                stored_key_preview,
            } => {
                // Update credential status display
                self.credentials_stored = credentials_stored;
                self.stored_access_key_preview = stored_key_preview;
            }
        }
    }
}

impl SettingsForm {
    fn select_collection_root_dir(&mut self, root: &gtk::Window, sender: &ComponentSender<Self>) {
        let dialog = FileChooserDialog::builder()
            .title("Select Collection Root Directory")
            .action(gtk::FileChooserAction::SelectFolder)
            .modal(true)
            .transient_for(root)
            .build();

        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        dialog.add_button("Select", gtk::ResponseType::Accept);

        dialog.connect_response(clone!(
            #[strong]
            sender,
            move |dialog, response| {
                tracing::info!("Directory selection dialog response: {:?}", response);
                if response == gtk::ResponseType::Accept
                    && let Some(path) = dialog.file().and_then(|f| f.path())
                {
                    tracing::info!("Selected directory path: {:?}", path);
                    sender.input(SettingsFormMsg::CollectionRootDirSelected(path));
                }
                dialog.close();
            }
        ));

        dialog.present();
    }
}
