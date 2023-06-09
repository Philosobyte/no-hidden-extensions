#![windows_subsystem = "windows"]

use clap::{Parser, command, arg};
use iced::{Application, Settings, Theme};
use tray_icon::{TrayIcon, TrayIconBuilder};
use anyhow::{anyhow, Result};
use image::RgbaImage;
use tracing::instrument;

use crate::err::IconLoadingError;
use crate::ui::{APPLICATION_DISPLAY_NAME, NoHiddenExtensionsState, UiOptions};

mod windows_ops;
mod ui;
mod err;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    start_minimized: bool,
}

#[instrument]
fn load_visual_data_for_tray_and_window_icon() -> Result<(Vec<u8>, u32, u32)> {
    // embed bytes into the executable at compile-time
    let image_bytes = include_bytes!("..\\resources\\tray_and_window_icon.png");
    let image: RgbaImage = image::load_from_memory(image_bytes)?
        .into_rgba8();

    let (width, height) = image.dimensions();
    let rgba: Vec<u8> = image.into_raw();

    Ok((rgba, width, height))
}


pub fn main() -> Result<()> {
    // log to stdout
    tracing_subscriber::fmt::fmt()
        .try_init()
        .map_err(|e| anyhow!(e))?;

    let (rgba, width, height) = load_visual_data_for_tray_and_window_icon()
        .map_err(|error| IconLoadingError::FailedToLoadIconBytes(error))?;

    // add to the Windows system tray
    let tray_ic: tray_icon::icon::Icon = tray_icon::icon::Icon::from_rgba(rgba.clone(), width.clone(), height.clone())
        .map_err(|bad_icon| IconLoadingError::FailedToConstructTrayIcon(Box::new(bad_icon)))?;

    let _tray_ic: TrayIcon = TrayIconBuilder::new()
        .with_tooltip(APPLICATION_DISPLAY_NAME)
        .with_icon(tray_ic)
        .build()
        .map_err(|error| IconLoadingError::FailedToConstructTrayIcon(Box::new(error)))?;

    let main_window_ic: iced::window::Icon = iced::window::Icon::from_rgba(rgba, width, height)
        .map_err(|error| IconLoadingError::FailedToConstructWindowIcon(Box::new(error)))?;

    let theme: Theme = match dark_light::detect() {
        dark_light::Mode::Dark => Theme::Dark,
        dark_light::Mode::Light => Theme::Light,
        dark_light::Mode::Default => Theme::default()
    };

    let executable_args: Args = Args::parse();
    let mut settings: Settings<UiOptions> = Settings::with_flags(
        UiOptions {
            start_minimized: executable_args.start_minimized,
            theme
        }
    );

    settings.window.icon = Some(main_window_ic);
    settings.window.size = (475, 175);
    settings.window.visible = !executable_args.start_minimized;

    NoHiddenExtensionsState::run(settings)
        .map_err(|e| anyhow!(e))
}
