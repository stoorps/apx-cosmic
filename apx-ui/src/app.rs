// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use crate::pages::{pkgmanagers, stacks, subsystems, Page, PageModel};
use cosmic::{
    app::{context_drawer, Core, Task},
    cosmic_config::{self, CosmicConfigEntry},
    cosmic_theme,
    iced::{
        alignment::{Horizontal, Vertical},
        Alignment, Length, Subscription,
    },
    theme,
    widget::{
        self, icon, menu, nav_bar,
        segmented_button::{Entity, HorizontalSegmentedButton, VerticalSegmentedButton},
    },
    Application, ApplicationExt, Apply, Element,
};
use futures_util::SinkExt;
use std::collections::HashMap;
use tracing::error;

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

    page_models: HashMap<Page, Box<dyn PageModel>>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    Navigate(Entity),
    SubNavigate(Entity),
    PkgManager(pkgmanagers::PkgManagerMessage),
    Stack(stacks::StackMessage),
    Subsystem(subsystems::SubsystemMessage),
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
    const APP_ID: &'static str = "com.singularityos.apx";

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

        nav.insert()
            //.text(fl!("subsystems"))
            .data::<Page>(Page::Subsystems)
            .icon(icon::from_name("utilities-terminal-symbolic"))
            .activate();

        nav.insert()
            // .text(fl!("pkgmanagers"))
            .data::<Page>(Page::PkgManagers)
            .icon(icon::from_name("applications-system-symbolic"));

        nav.insert()
            //  .text(fl!("stacks"))
            .data::<Page>(Page::Stacks)
            .icon(icon::from_name("network-server-symbolic"));

        let mut page_models: HashMap<Page, Box<dyn PageModel>> = HashMap::new();
        page_models.insert(
            Page::Subsystems,
            Box::new(subsystems::SubSystemsModel::new()),
        );
        page_models.insert(
            Page::PkgManagers,
            Box::new(pkgmanagers::PkgManagerModel::new()),
        );
        page_models.insert(Page::Stacks, Box::new(stacks::StacksModel::new()));

        page_models
            .iter_mut()
            .for_each(|(_, model)| model.update_items());

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            page_models,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((errors, config)) => {
                        for why in errors {
                            tracing::error!(%why, "error loading app config");
                        }

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
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        None //Some(&self.nav)
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
        let page = match self.nav.data::<Page>(self.nav.active()) {
            Some(page) => page,
            None => &Page::Subsystems, //TODO: handle this error better
        };

        //let spacing = cosmic_theme::Spacing::default();

        let page_model = self.page_models.get(page).unwrap(); //TODO: Handle this error better

        cosmic::iced_widget::row![
            widget::Container::new(cosmic::iced_widget::column![
                HorizontalSegmentedButton::new(&self.nav)
                    .button_height(32)
                    .button_padding([8, 16, 8, 16])
                    .button_spacing(8)
                    .minimum_button_width(32)
                    .width(Length::Fill)
                    .button_alignment(Alignment::Center)
                    .on_activate(|id| Message::Navigate(id))
                    .style(theme::SegmentedButton::TabBar),
                VerticalSegmentedButton::new(page_model.current_items())
                    .style(theme::SegmentedButton::TabBar)
                    .button_height(32)
                    .button_padding([8, 16, 8, 16])
                    .button_spacing(8)
                    .width(Length::Fill)
                    .on_activate(|id| Message::SubNavigate(id))
            ])
            .width(Length::Fixed(300.))
            .style(|_| theme::Container::primary(&cosmic_theme::Theme::default())) //TODO: Understand this... Doesn't seem to follow other widgets
            .height(Length::Fill),
            widget::Container::new(
                page_model
                    .view()
                    .apply(widget::container)
                    .width(Length::Fixed(400.))
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
            )
            .width(Length::Fill)
        ]
        .spacing(10)
        .padding(10)
        .into()
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
                    for why in update.errors {
                        tracing::error!(?why, "app config error");
                    }

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
                    error!("failed to open {url:?}: {err}");
                }
            },
            Message::Navigate(entity) => self.nav.activate(entity),
            Message::SubNavigate(entity) => {
                let page = match self.nav.data::<Page>(self.nav.active()) {
                    Some(page) => page,
                    None => &Page::Subsystems, //TODO: handle this error better
                };

                let page_model = self.page_models.get_mut(page).unwrap(); //TODO: Handle this error better

                page_model.on_select(entity);
            }
            Message::PkgManager(_) => self
                .page_models
                .get_mut(&Page::PkgManagers)
                .unwrap()
                .on_message(message),
            Message::Stack(_) => self
                .page_models
                .get_mut(&Page::Stacks)
                .unwrap()
                .on_message(message),
            Message::Subsystem(_) => self
                .page_models
                .get_mut(&Page::Subsystems)
                .unwrap()
                .on_message(message),
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
