use iced::{Alignment, Application, Command, Element, executor, Length, subscription, Subscription, Theme, window};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, checkbox, column, container, text, Text};
use iced::window::{Event, Mode, UserAttention};
use tracing::{instrument, trace};
use tray_icon::TrayEvent;

use crate::ui::Message::{Backend, Ui, User};
use crate::windows_ops;

pub(crate) const APPLICATION_DISPLAY_NAME: &str = "no-hidden-extensions";

// Notification of user input
#[derive(Debug, Clone)]
pub(crate) enum UserMessage {
    RunAtStartup,
    DontRunAtStartup,
    HideFileExtensions,
}

// Notification of change in system state
#[derive(Debug, Clone)]
pub(crate) enum BackendMessage {
    FileExtensionsAreNowHidden,
    FileExtensionsAreNoLongerHidden,
}

// Notification of change in UI windowing
#[derive(Debug, Clone)]
pub(crate) enum UiMessage {
    MinimizeToTray,
    RestoreFromTray
}

// Used for communication between components
#[derive(Debug, Clone)]
pub(crate) enum Message {
    User(UserMessage),
    Backend(BackendMessage),
    Ui(UiMessage),
}

#[derive(Debug, Clone)]
pub(crate) struct UiOptions {
    pub(crate) start_minimized: bool,
    pub(crate) theme: Theme,
}

// primary application state
#[derive(Debug, Clone)]
pub(crate) struct NoHiddenExtensionsState {
    run_at_startup: bool,
    file_extensions_hidden: bool,
    system_theme: Theme,
}

impl Application for NoHiddenExtensionsState {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = UiOptions;

    #[instrument]
    fn new(ui_options: UiOptions) -> (NoHiddenExtensionsState, Command<Message>) {
        let file_extensions_hidden: &bool = &windows_ops::are_file_extensions_hidden()
            .expect("Could not determine whether file extensions are hidden");

        let run_at_startup: &bool = &windows_ops::will_app_run_at_startup()
            .expect("Could not determine whether app will run at startup");

        let no_hidden_extensions_state = NoHiddenExtensionsState {
            run_at_startup: *run_at_startup,
            file_extensions_hidden: *file_extensions_hidden,
            system_theme: ui_options.theme,
        };

        let commands: Command<Message> = if *file_extensions_hidden {
            // file extensions are already hidden, so we need to tell the user regardless of
            // whether we're supposed to start minimized
            get_commands_which_notify_user()
        } else if ui_options.start_minimized {
            window::change_mode(Mode::Hidden)
        } else {
            Command::none()
        };

        return (no_hidden_extensions_state, commands);
    }

    fn title(&self) -> String {
        String::from(APPLICATION_DISPLAY_NAME)
    }

    #[instrument]
    fn update(&mut self, message: Message) -> Command<Message> {
        return match message {
            User(user_message) => {
                match user_message {
                    UserMessage::RunAtStartup => {
                        windows_ops::run_this_program_at_startup()
                            .expect("Unable to make this program run at startup");
                        self.run_at_startup = true;
                        Command::none()
                    },
                    UserMessage::DontRunAtStartup => {
                        windows_ops::dont_run_this_program_at_startup()
                            .expect("Unable to stop making this program run at startup");
                        self.run_at_startup = false;
                        Command::none()
                    },
                    UserMessage::HideFileExtensions => {
                        windows_ops::turn_off_file_extension_hiding()
                            .expect("Unable to turn off file extension hiding");
                        Command::none()
                    },
                }
            },
            Backend(backend_message) => {
                match backend_message {
                    BackendMessage::FileExtensionsAreNowHidden => {
                        self.file_extensions_hidden = true;
                        get_commands_which_notify_user()
                    },
                    BackendMessage::FileExtensionsAreNoLongerHidden => {
                        self.file_extensions_hidden = false;
                        Command::none()
                    },
                }
            },
            Ui(ui_message) => {
                match ui_message {
                    UiMessage::RestoreFromTray => {
                        Command::batch(vec![
                            window::change_mode(Mode::Windowed),
                            window::minimize(false),
                            window::gain_focus(),
                        ])
                    },
                    UiMessage::MinimizeToTray => {
                        window::change_mode::<Message>(Mode::Hidden)
                    }
                }
            }
        };
    }

    #[instrument]
    fn view(&self) -> Element<Message> {
        let body_text: Text = match self.file_extensions_hidden {
            true => text(
                "Warning - file extensions are hidden in Windows Explorer. This means a higher risk \
                 of falling for a phishing attack."
            ),
            false => text(
                "File extensions are visible in Windows Explorer, which is great! \
                 It is harder for you to fall for a phishing attack."
            )
        }.horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);

        let stop_hiding_file_extensions_button = match self.file_extensions_hidden {
            true => button("Stop hiding file extensions and restart Windows Explorer").on_press(User(UserMessage::HideFileExtensions)),
            false => button("Stop hiding file extensions and restart Windows Explorer")
        };

        let run_at_startup_checkbox = checkbox(
            "Run at Windows startup",
            self.run_at_startup,
            |run_at_startup| match run_at_startup {
                true => User(UserMessage::RunAtStartup),
                false => User(UserMessage::DontRunAtStartup)
            }
        );

        let content = column![body_text, stop_hiding_file_extensions_button, run_at_startup_checkbox]
            .align_items(Alignment::Center)
            .spacing(20)
            .padding(20);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn theme(&self) -> Theme {
        self.system_theme.clone()
    }

    #[instrument]
    fn subscription(&self) -> Subscription<Message> {
        return Subscription::batch(vec![
            get_listener_for_backend_messages(),
            get_listener_for_ui_messages(),
            get_listener_for_window_resize_messages(),
        ]);
    }
}

fn get_listener_for_backend_messages() -> Subscription<Message> {
    subscription::unfold(
        std::any::TypeId::of::<BackendMessage>(),
        0,
        |_| async {
            trace!("Waiting for a change in the Windows Explorer registry key");
            windows_ops::wait_for_any_change_in_windows_explorer_regkey()
                .expect("Failed to wait for a change in the Windows Explorer Advanced registry key");
            trace!("Received a change in the Windows Explorer registry key");

            match windows_ops::are_file_extensions_hidden().expect("Failed to check whether file extensions are currently being hidden") {
                true => (Some(Backend(BackendMessage::FileExtensionsAreNowHidden)), 0),
                false => (Some(Backend(BackendMessage::FileExtensionsAreNoLongerHidden)), 0)
            }
        }
    )
}

fn get_listener_for_ui_messages() -> Subscription<Message> {
    subscription::events_with(|event, _status|
        match event {
            iced::Event::Window(window_event) => {
                match window_event {
                    // these are typical values when user clicks on the minimize button
                    Event::Resized {width: 0, height: 0} => {
                        Some(Ui(UiMessage::MinimizeToTray))
                    },
                    _ => None
                }
            },
            _ => None
        }
    )
}

fn get_listener_for_window_resize_messages() -> Subscription<Message> {
    subscription::unfold(
        std::any::TypeId::of::<UiMessage>(),
        0,
        |_| async {
            let _: TrayEvent = TrayEvent::receiver().recv()
                .expect("Unable to listen for tray events");
            // We don't have a menu, so allow any tray event to restore the window
            (Some(Ui(UiMessage::RestoreFromTray)), 0)
        }
    )
}

fn get_commands_which_notify_user() -> Command<Message> {
    Command::batch(vec![
        window::change_mode(Mode::Windowed),
        window::minimize(false),
        window::request_user_attention(Some(UserAttention::Informational)),
        window::gain_focus(),
    ])
}
