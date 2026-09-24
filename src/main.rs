use cosmic::app::{Core, Task};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{time, Alignment, Length, Subscription};
use cosmic::widget::{self, space, text};
use cosmic::Element;

use chrono::{DateTime, Local, Utc};
use std::time::Duration;
use usage::{Limit, UsageData};

mod usage;

const ICON: &[u8] = include_bytes!("../res/icon.png");
const POLL_EVERY: Duration = Duration::from_mins(5);
/// Sobre estos porcentajes el panel avisa en amarillo; el semanal solo aparece pasado su umbral.
const SESSION_WARN: f32 = 85.0;
const WEEKLY_WARN: f32 = 90.0;

#[derive(Default)]
struct State {
    core: Core,
    popup: Option<Id>,
    usage: Option<UsageData>,
    loading: bool,
    error: Option<String>,
    /// Reloj del modelo: la cuenta regresiva se calcula contra este instante, no en la vista.
    now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
enum Message {
    TogglePopup,
    PopupClosed(Id),
    FetchUsage,
    UsageReceived(Result<UsageData, String>),
    /// Avanza el reloj del modelo para la cuenta regresiva, sin volver a pedir datos.
    Tick,
}

impl cosmic::Application for State {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "io.github.qnelo.ClaudeStatus";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        (Self { core, ..Default::default() }, cosmic::task::message(Message::FetchUsage))
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            time::every(POLL_EVERY).map(|_| Message::FetchUsage),
            time::every(Duration::from_secs(60)).map(|_| Message::Tick),
        ])
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(id) = self.popup.take() {
                    destroy_popup(id)
                } else {
                    let id = Id::unique();
                    self.popup = Some(id);
                    self.now = Utc::now();
                    let parent = self.core.main_window_id().unwrap();
                    get_popup(self.core.applet.get_popup_settings(parent, id, None, None, None))
                };
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
            Message::FetchUsage => {
                self.loading = true;
                return cosmic::task::future(async { Message::UsageReceived(usage::fetch().await) });
            }
            Message::Tick => self.now = Utc::now(),
            Message::UsageReceived(result) => {
                self.loading = false;
                self.now = Utc::now();
                // Si falla, se conservan los últimos datos buenos y se muestra el error.
                match result {
                    Ok(usage) => {
                        self.usage = Some(usage);
                        self.error = None;
                    }
                    Err(e) => self.error = Some(e),
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let applet = &self.core.applet;
        let (major, minor) = applet.suggested_padding(true);
        let icon = widget::icon(widget::icon::from_raster_bytes(ICON))
            .size(applet.suggested_size(true).0);
        let mut content = widget::Row::new().push(icon).spacing(4).align_y(Alignment::Center);

        match &self.usage {
            Some(u) => {
                let warn = cosmic::theme::Text::Color(
                    cosmic::theme::active().cosmic().warning_text_color().into(),
                );
                let mut session = applet.text(format!("{:.0}%", u.session.pct));
                if u.session.pct > SESSION_WARN {
                    session = session.class(warn);
                }
                content = content.push(session);
                if u.weekly_all.pct > WEEKLY_WARN {
                    content = content
                        .push(applet.text("·"))
                        .push(applet.text(format!("S {:.0}%", u.weekly_all.pct)).class(warn));
                }
            }
            None => content = content.push(applet.text("–")),
        }

        // Sin autosize la ventana del applet mide lo que el icono y el texto queda recortado.
        let button = widget::button::custom(content)
            .padding([minor, major])
            .class(cosmic::theme::Button::AppletIcon)
            .on_press_down(Message::TogglePopup);
        applet.autosize_window(button).into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        let badge = widget::container(text::caption("Equipo"))
            .padding([2, 8])
            .class(cosmic::theme::Container::Card);
        let header = widget::Row::new()
            .push(text::title4("Tus límites de uso"))
            .push(space::horizontal())
            .push(badge)
            .align_y(Alignment::Center);

        let mut content = widget::Column::new().spacing(12).padding(16).push(header);

        if let Some(u) = &self.usage {
            content = content
                .push(text::heading("Sesión actual"))
                .push(limit_view(&u.session, |t| usage::resets_in(t, self.now)))
                .push(widget::divider::horizontal::default())
                .push(text::heading("Límites semanales"))
                .push(text::body("Todos los modelos"))
                .push(limit_view(&u.weekly_all, weekly_reset));
            for (model, limit) in &u.weekly_models {
                content = content.push(text::body(model)).push(limit_view(limit, weekly_reset));
            }
        }

        let status = if self.loading {
            "Actualizando…"
        } else {
            self.error.as_deref().unwrap_or("")
        };
        let footer = widget::Row::new()
            .push(text::caption(status))
            .push(space::horizontal())
            .push(
                widget::button::standard("Actualizar")
                    .on_press_maybe((!self.loading).then_some(Message::FetchUsage)),
            )
            .align_y(Alignment::Center);

        self.core.applet.popup_container(content.push(footer)).into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn weekly_reset(at: DateTime<Utc>) -> String {
    usage::resets_on(at.with_timezone(&Local).naive_local())
}

/// Barra con "N% usado" a la derecha y el texto de reinicio debajo.
fn limit_view<'a>(limit: &Limit, reset_text: impl Fn(DateTime<Utc>) -> String) -> Element<'a, Message> {
    let bar = widget::Row::new()
        .push(widget::determinate_linear(limit.pct / 100.0).width(Length::Fill))
        .push(text::body(format!("{:.0}% usado", limit.pct)))
        .spacing(12)
        .align_y(Alignment::Center);
    let mut col = widget::Column::new().push(bar).spacing(4);
    if let Some(at) = limit.resets_at {
        col = col.push(text::caption(reset_text(at)));
    }
    col.into()
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<State>(())
}
