// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use crate::utils::{VolumesControl, VolumesControlMessage, VolumesModel};
use cosmic::app::{context_drawer, Core, Task};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::Horizontal::Left;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Executor, Length, Subscription};
use cosmic::iced_core::Layout;
use cosmic::iced_widget::{Column, Row};
use cosmic::widget::menu::menu_button;
use cosmic::widget::text::heading;
use cosmic::widget::{self, container, flex_row, horizontal_space, icon, menu, nav_bar, Space};
use cosmic::{cosmic_theme, iced_widget, theme, Application, ApplicationExt, Apply, Element};
use cosmos_common::{bytes_to_pretty, labelled_info};
use cosmos_dbus::udisks::DriveModel;
use futures_util::SinkExt;
use std::collections::HashMap;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: Config,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    VolumesMessage(VolumesControlMessage),
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.cosmos.Disks";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        // Create a nav bar with three page items.
        let mut nav = nav_bar::Model::default();

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let drives = match DriveModel::get_drives().await {
                    Ok(drives) => drives,
                    Err(_) => return,
                };

                for drive in drives {
                    let icon = match drive.removable {
                        true => "drive-removable-media-symbolic",
                        false => "disks-symbolic",
                    };

                    nav.insert()
                        .text(drive.pretty_name())
                        .data::<VolumesControl>(VolumesControl::new(drive.clone()))
                        .data::<DriveModel>(drive)
                        .icon(icon::from_name(icon));
                }
            });

        // nav.insert()
        //     .text(fl!("page-id", num = 1))
        //     .data::<Page>(Page::Page1)
        //     .icon(icon::from_name("applications-science-symbolic"))
        //     .activate();

        // nav.insert()
        //     .text(fl!("page-id", num = 2))
        //     .data::<Page>(Page::Page2)
        //     .icon(icon::from_name("applications-system-symbolic"));

        // nav.insert()
        //     .text(fl!("page-id", num = 3))
        //     .data::<Page>(Page::Page3)
        //     .icon(icon::from_name("applications-games-symbolic"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    }
                })
                .unwrap_or_default(),
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![
            menu::Tree::with_children(
                menu::root("Image"),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button("New Disk Image", None, MenuAction::About),
                        menu::Item::Button("Attach Disk Image", None, MenuAction::About),
                        menu::Item::Button("Create Disk From Drive", None, MenuAction::About),
                        menu::Item::Button("Restore Image to Drive", None, MenuAction::About),
                    ],
                ),
            ),
            menu::Tree::with_children(
                menu::root("Disk"),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button("Eject", None, MenuAction::About),
                        menu::Item::Button("Power Off", None, MenuAction::About),
                        menu::Item::Button("Format Disk", None, MenuAction::About),
                        menu::Item::Button("Benchmark Disk", None, MenuAction::About),
                        menu::Item::Button("SMART Data & Self-Tests", None, MenuAction::About),
                        menu::Item::Button("Drive Settings", None, MenuAction::About),
                        menu::Item::Button("Standby Now", None, MenuAction::About),
                        menu::Item::Button("Wake-up From Standby", None, MenuAction::About),
                    ],
                ),
            ),
            menu::Tree::with_children(
                menu::root(fl!("view")),
                menu::items(
                    &self.key_binds,
                    vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
                ),
            ),
        ]);

        // let end_bar = menu::bar(vec![menu::Tree::with_children(
        //     menu_button(vec![widget::icon::from_name("open-menu-symbolic").into()])
        //         .width(Length::Shrink),
        //     menu::items(
        //         &self.key_binds,
        //         vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
        //     ),
        // )])
        // .width(Length::Fixed(50.));

        vec![menu_bar.into()] //, horizontal_space().into(), end_bar.into()]
    }

    /// Allows overriding the default nav bar widget.
    fn nav_bar(&self) -> Option<Element<cosmic::app::Message<Self::Message>>> {
        if !self.core().nav_bar_active() {
            return None;
        }

        let nav_model = self.nav_model()?;

        let mut nav = widget::nav_bar(nav_model, |id| {
            cosmic::app::Message::Cosmic(cosmic::app::cosmic::Message::NavBar(id))
        })
        .on_context(|id| {
            cosmic::app::Message::Cosmic(cosmic::app::cosmic::Message::NavBarContext(id))
        })
        // .context_menu(self.nav_context_menu(self.nav_bar()))
        .into_container()
        // XXX both must be shrink to avoid flex layout from ignoring it
        .width(cosmic::iced::Length::Shrink)
        .height(cosmic::iced::Length::Shrink);

        if !self.core().is_condensed() {
            nav = nav.max_width(280);
        }

        Some(Element::from(nav))
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(
                self.about(),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        match self.nav.active_data::<DriveModel>() {
            None => widget::text::title1("No disk selected")
                .apply(widget::container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),

            Some(drive) => {
                let volumes_control = self.nav.active_data::<VolumesControl>().unwrap(); //TODO: Handle unwrap.

                let segment = volumes_control
                    .segments
                    .get(volumes_control.selected_segment)
                    .unwrap(); //TODO: Handle unwrap.
                let info = match segment.partition.clone() {
                    Some(p) => {
                        let mut name = p.name.clone();
                        if name.len() == 0 {
                            name = format!("Partition {}", &p.number);
                        } else {
                            name = format!("Partition {}: {}", &p.number, name);
                        }

                        match &p.usage {
                            Some(usage) => iced_widget::column![
                                heading(name),
                                Space::new(0, 10),
                                labelled_info("Size", bytes_to_pretty(&p.size, true)),
                                labelled_info("Usage", bytes_to_pretty(&usage.used, false)),
                                labelled_info("Mounted at", &usage.mount_point),
                                labelled_info("Contents", &p.partition_type),
                                labelled_info("Device", &p.device_path),
                                labelled_info("UUID", &p.uuid),
                            ]
                            .spacing(5),

                            None => iced_widget::column![
                                heading(name),
                                Space::new(0, 10),
                                labelled_info("Size", bytes_to_pretty(&p.size, true)),
                                labelled_info("Contents", &p.partition_type),
                                labelled_info("Device", &p.device_path),
                                labelled_info("UUID", &p.uuid),
                            ]
                            .spacing(5),
                        }
                    }
                    None => {
                        iced_widget::column![
                            heading(&segment.label),
                            labelled_info("Size", bytes_to_pretty(&segment.size, true)),
                            //labelled_info("Contents", &segment.partition_type),
                            // labelled_info("Device", &segment.device_path),
                            // labelled_info("UUID", &p.uuid),
                        ]
                        .spacing(5)
                    }
                };

                iced_widget::column![
                    iced_widget::column![
                        heading(drive.pretty_name()),
                        Space::new(0, 10),
                        labelled_info("Model", &drive.model),
                        labelled_info("Serial", &drive.serial),
                        labelled_info("Size", bytes_to_pretty(&drive.size, true)),
                    ]
                    .spacing(5)
                    .width(Length::Fill),
                    iced_widget::column![
                        heading("Volumes"),
                        Space::new(0, 10),
                        volumes_control.view()
                    ]
                    .spacing(5)
                    .width(Length::Fill),
                    info
                ]
                .spacing(60)
                .padding(20)
                .width(Length::Fill)
                .into()
            }
        }
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }

            Message::SubscriptionChannel => {
                // For example purposes only.
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },
            Message::VolumesMessage(volumes_control_message) => {
                let volumes_control = self.nav.active_data_mut::<VolumesControl>().unwrap(); //TODO: HANDLE UNWRAP.
                volumes_control.update(volumes_control_message);
            }
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        // Activate the page in the model.
        self.nav.activate(id);
        self.update_title()
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

        let title = widget::text::title3(fl!("app-title"));

        let hash = env!("VERGEN_GIT_SHA");
        let short_hash: String = hash.chars().take(7).collect();
        let date = env!("VERGEN_GIT_COMMIT_DATE");

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .push(
                widget::button::link(fl!(
                    "git-description",
                    hash = short_hash.as_str(),
                    date = date
                ))
                .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
                .padding(0),
            )
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<Message> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

/// The page to display in the application.
pub enum Page {
    Page1,
    Page2,
    Page3,
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}
