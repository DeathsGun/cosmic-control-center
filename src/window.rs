use crate::fl;
use cosmic::app::{Command, Core};
use cosmic::applet::padded_control;
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::{Alignment, Subscription};
use cosmic::iced_core::window::Id;
use cosmic::iced_style::application;
use cosmic::iced_widget::{row, Column};
use cosmic::widget::{icon, slider, text};
use cosmic::{Element, Theme};
use cosmic_settings_subscriptions::settings_daemon;
use tokio::sync::mpsc::UnboundedSender;

pub const ID: &str = "xyz.deathsgun.CosmicControlCenter";

#[derive(Clone, Default)]
pub struct Window {
    core: Core,
    popup: Option<Id>,
    pub max_screen_brightness: Option<i32>,
    pub screen_brightness: Option<i32>,
    zbus_connection: Option<zbus::Connection>,
    settings_daemon_sender: Option<UnboundedSender<settings_daemon::Request>>,
}

#[derive(Clone, Debug)]
pub enum Message {
    PopupClosed(Id),
    TogglePopup,
    SetScreenBrightness(i32),
    SettingsDaemon(settings_daemon::Event),
    ZbusConnection(zbus::Result<zbus::Connection>),
}

impl cosmic::Application for Window {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                core,
                ..Default::default()
            },
            Command::perform(zbus::Connection::session(), |res| {
                cosmic::app::Message::App(Message::ZbusConnection(res))
            }),
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Self::Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subscriptions = vec![];
        if let Some(conn) = self.zbus_connection.clone() {
            subscriptions.push(settings_daemon::subscription(conn).map(Message::SettingsDaemon))
        }
        Subscription::batch(subscriptions)
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let pop_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::MAIN, new_id, None, None, None);
                    get_popup(pop_settings)
                }
            }
            Message::SetScreenBrightness(value) => {
                self.screen_brightness = Some(value);
                if let Some(tx) = &self.settings_daemon_sender {
                    let _ = tx.send(settings_daemon::Request::SetDisplayBrightness(value));
                }
            }
            Message::ZbusConnection(Err(err)) => {
                tracing::error!("Failed to connect to session dbus: {}", err);
            }
            Message::ZbusConnection(Ok(conn)) => {
                tracing::info!("Got zbus connection");
                self.zbus_connection = Some(conn);
            }
            Message::SettingsDaemon(event) => {
                tracing::info!("Got SettingDaemon-Event");
                match event {
                    settings_daemon::Event::Sender(tx) => {
                        tracing::info!("Got sender");
                        self.settings_daemon_sender = Some(tx);
                    }
                    settings_daemon::Event::MaxDisplayBrightness(max_brightness) => {
                        tracing::info!("Max-Brightness: {:?}", max_brightness);
                        self.max_screen_brightness = Some(max_brightness);
                    }
                    settings_daemon::Event::DisplayBrightness(brightness) => {
                        tracing::info!("Brightness: {:?}", brightness);
                        self.screen_brightness = Some(brightness);
                    }
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        self.core
            .applet
            .icon_button(ID)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _: Id) -> Element<Self::Message> {
        let mut content = vec![];

        // Wi-Fi & bluetooth
        

        // Display brightness control
        if let Some(max_screen_brightness) = self.max_screen_brightness {
            if let Some(screen_brightness) = self.screen_brightness {
                content.push(padded_control(
                    row![
                    icon::from_name("display-brightness-symbolic")
                    .size(24)
                    .symbolic(true),
                    text(fl!("display")),
                    slider(1..=max_screen_brightness, screen_brightness, Message::SetScreenBrightness)
                ].spacing(8)
                        .align_items(Alignment::Center),
                ).into())
            }
        }
        self.core
            .applet
            .popup_container(Column::with_children(content).padding([8, 0]))
            .into()
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
    }
}
