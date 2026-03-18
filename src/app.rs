use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{
    executor, theme, Alignment, Application, Color, Command, Element, Length, Settings, Theme,
};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

use crate::core;
use crate::csv_io;

const DEFAULT_UNC_BASE: &str = r"\\etail.rs\files\hardwaretools\Images2\Images_Uploaded_3_18_26";

pub fn run() -> iced::Result {
    ImageProcessorApp::run(Settings::default())
}

#[derive(Debug, Clone)]
enum Message {
    ImageFolderChanged(String),
    OutputFolderChanged(String),
    UncBaseChanged(String),
    PickImageFolder,
    PickOutputFolder,
    RunPressed,
}

#[derive(Debug, Clone, Copy)]
enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
struct AppStatus {
    level: StatusLevel,
    text: String,
}

impl AppStatus {
    fn info(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Info,
            text: text.into(),
        }
    }

    fn success(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Success,
            text: text.into(),
        }
    }

    fn warning(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Warning,
            text: text.into(),
        }
    }

    fn error(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Error,
            text: text.into(),
        }
    }
}

struct ImageProcessorApp {
    image_folder: String,
    output_folder: String,
    unc_base: String,
    status: AppStatus,
    colors: AppColors,
}

#[derive(Clone, Copy)]
struct AppColors {
    background: Color,
    text: Color,
    primary: Color,
    success: Color,
    warning: Color,
    danger: Color,
}

impl Application for ImageProcessorApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                image_folder: String::new(),
                output_folder: String::new(),
                unc_base: DEFAULT_UNC_BASE.to_owned(),
                status: AppStatus::info("Select inputs, then click Run."),
                colors: load_colors(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "eTail Image Processor".to_owned()
    }

    fn theme(&self) -> Self::Theme {
        Theme::custom(
            "etail-colorscheme".to_owned(),
            theme::Palette {
                background: self.colors.background,
                text: self.colors.text,
                primary: self.colors.primary,
                success: self.colors.success,
                danger: self.colors.danger,
            },
        )
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::ImageFolderChanged(value) => self.image_folder = value,
            Message::OutputFolderChanged(value) => self.output_folder = value,
            Message::UncBaseChanged(value) => self.unc_base = value,
            Message::PickImageFolder => {
                if let Some(path) = FileDialog::new().pick_folder() {
                    self.image_folder = path.display().to_string();
                }
            }
            Message::PickOutputFolder => {
                let mut dialog = FileDialog::new();
                if !self.image_folder.trim().is_empty() {
                    dialog = dialog.set_directory(self.image_folder.trim());
                }
                if let Some(path) = dialog.pick_folder() {
                    self.output_folder = path.display().to_string();
                }
            }
            Message::RunPressed => match self.run_processing() {
                Ok(status) => self.set_status(status),
                Err(error) => self.set_status(AppStatus::error(format!("Error: {error:#}"))),
            },
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let label_color = Color::WHITE;
        let status_color = self.status_color();

        let status_text = text(&self.status.text).style(status_color).size(16);

        let image_folder_row = row![
            text("Image Folder")
                .width(Length::Fixed(130.0))
                .style(label_color),
            text_input("Select folder with JPG images", &self.image_folder)
                .on_input(Message::ImageFolderChanged)
                .width(Length::Fill),
            button("Browse")
                .style(primary_button_style(self.colors.primary))
                .on_press(Message::PickImageFolder),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let output_folder_row = row![
            text("Output Folder")
                .width(Length::Fixed(130.0))
                .style(label_color),
            text_input("Leave blank to use image folder", &self.output_folder)
                .on_input(Message::OutputFolderChanged)
                .width(Length::Fill),
            button("Browse")
                .style(primary_button_style(self.colors.primary))
                .on_press(Message::PickOutputFolder),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let unc_row = row![
            text("UNC Base")
                .width(Length::Fixed(130.0))
                .style(label_color),
            text_input("\\\\server\\share\\path", &self.unc_base)
                .on_input(Message::UncBaseChanged)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let run_row = row![button("Run")
            .style(primary_button_style(self.colors.primary))
            .on_press(Message::RunPressed)]
        .spacing(8);

        let content = column![
            status_text,
            image_folder_row,
            output_folder_row,
            unc_row,
            run_row
        ]
        .spacing(10)
        .padding(24)
        .width(Length::Fill)
        .height(Length::Shrink);

        let panel = container(content).max_width(900);

        container(panel)
            .padding(32)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .into()
    }
}

impl ImageProcessorApp {
    fn status_color(&self) -> Color {
        match self.status.level {
            StatusLevel::Error => self.colors.danger,
            StatusLevel::Warning => self.colors.warning,
            StatusLevel::Success => self.colors.success,
            StatusLevel::Info => Color::WHITE,
        }
    }

    fn set_status(&mut self, status: AppStatus) {
        println!("{}", status.text);
        self.status = status;
    }

    fn run_processing(&mut self) -> Result<AppStatus> {
        let image_folder = self.required_path(&self.image_folder, "Image folder")?;
        if !image_folder.is_dir() {
            anyhow::bail!(
                "Image folder is not a directory: {}",
                image_folder.display()
            );
        }

        let output_folder = if self.output_folder.trim().is_empty() {
            image_folder.clone()
        } else {
            PathBuf::from(self.output_folder.trim())
        };

        if !output_folder.exists() {
            anyhow::bail!("Output folder does not exist: {}", output_folder.display());
        }
        if !output_folder.is_dir() {
            anyhow::bail!(
                "Output path is not a directory: {}",
                output_folder.display()
            );
        }

        let unc_base = self.unc_base.trim();
        if unc_base.is_empty() {
            anyhow::bail!("UNC base path cannot be blank");
        }

        let image_rows = core::collect_image_rows(&image_folder, unc_base)?;
        let date = Local::now().date_naive();
        let output_target = csv_io::build_image_output_target(&output_folder, date);

        let output_path = match self.resolve_output_path(&output_target.images_csv)? {
            Some(path) => path,
            None => {
                return Ok(AppStatus::warning(
                    "Warning: operation cancelled during output conflict resolution.",
                ));
            }
        };

        let rows_written = csv_io::write_image_csv(&output_path, &image_rows)?;

        Ok(AppStatus::success(format!(
            "Done.\n\nRows written: {rows_written}\nOutput: {}",
            output_path.display()
        )))
    }

    fn resolve_output_path(&self, target_path: &Path) -> Result<Option<PathBuf>> {
        if !target_path.exists() {
            return Ok(Some(target_path.to_path_buf()));
        }

        let action = MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title("Output file already exists")
            .set_description(&format!(
                "File exists:\n{}\n\nYes = Overwrite\nNo = Rename\nCancel = Abort",
                target_path.display()
            ))
            .set_buttons(MessageButtons::YesNoCancel)
            .show();

        match action {
            MessageDialogResult::Yes => Ok(Some(target_path.to_path_buf())),
            MessageDialogResult::No => {
                let renamed = self.prompt_rename_path(target_path)?;
                Ok(renamed)
            }
            _ => Ok(None),
        }
    }

    fn prompt_rename_path(&self, target_path: &Path) -> Result<Option<PathBuf>> {
        let parent = target_path
            .parent()
            .context("Cannot rename output without a parent directory")?;
        let default_name = target_path
            .file_name()
            .and_then(|value| value.to_str())
            .context("Output file has an invalid name")?;

        loop {
            let selected = FileDialog::new()
                .set_title("Rename output CSV")
                .set_directory(parent)
                .set_file_name(default_name)
                .save_file();

            let Some(path) = selected else {
                return Ok(None);
            };

            if path.exists() {
                let overwrite = MessageDialog::new()
                    .set_level(MessageLevel::Warning)
                    .set_title("Selected file already exists")
                    .set_description(
                        "Selected target exists. Choose Yes to overwrite it, or No to pick another name.",
                    )
                    .set_buttons(MessageButtons::YesNo)
                    .show();

                if matches!(overwrite, MessageDialogResult::Yes) {
                    return Ok(Some(path));
                }

                continue;
            }

            return Ok(Some(path));
        }
    }

    fn required_path(&self, raw: &str, label: &str) -> Result<PathBuf> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            anyhow::bail!("{label} is required");
        }
        Ok(PathBuf::from(trimmed))
    }
}

#[derive(Debug, Clone)]
struct PrimaryButtonStyle {
    color: Color,
}

impl button::StyleSheet for PrimaryButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(self.color.into()),
            text_color: Color::WHITE,
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(lighten(self.color, 0.08).into());
        appearance
    }
}

fn load_colors() -> AppColors {
    let defaults = AppColors {
        background: hex_or_default("#161320", Color::from_rgb8(22, 19, 32)),
        text: Color::WHITE,
        primary: Color::from_rgb8(77, 142, 250),
        success: Color::from_rgb8(114, 217, 138),
        warning: Color::from_rgb8(242, 221, 31),
        danger: Color::from_rgb8(225, 81, 104),
    };

    let Ok(content) = fs::read_to_string("Colorscheme") else {
        return defaults;
    };

    let mut dark_base = None;
    let mut main_text_dark = None;
    let mut blue = None;
    let mut green = None;
    let mut yellow = None;
    let mut red = None;

    for line in content.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let parsed = parse_hex_color(value.trim());
        if parsed.is_none() {
            continue;
        }

        match key.trim() {
            "Dark base" => dark_base = parsed,
            "Main text dark" => main_text_dark = parsed,
            "Blue" => blue = parsed,
            "Green" => green = parsed,
            "Warning yellow" => yellow = parsed,
            "Critical red" => red = parsed,
            _ => {}
        }
    }

    AppColors {
        background: main_text_dark.or(dark_base).unwrap_or(defaults.background),
        text: defaults.text,
        primary: blue.unwrap_or(defaults.primary),
        success: green.unwrap_or(defaults.success),
        warning: yellow.unwrap_or(defaults.warning),
        danger: red.unwrap_or(defaults.danger),
    }
}

fn parse_hex_color(raw: &str) -> Option<Color> {
    let token = raw.trim().split_whitespace().next().unwrap_or(raw.trim());
    let hex = token.strip_prefix('#').unwrap_or(token);
    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::from_rgb8(red, green, blue))
}

fn hex_or_default(raw: &str, fallback: Color) -> Color {
    parse_hex_color(raw).unwrap_or(fallback)
}

fn lighten(color: Color, amount: f32) -> Color {
    Color::from_rgba(
        (color.r + amount).min(1.0),
        (color.g + amount).min(1.0),
        (color.b + amount).min(1.0),
        color.a,
    )
}

fn primary_button_style(color: Color) -> theme::Button {
    theme::Button::Custom(Box::new(PrimaryButtonStyle { color }))
}
