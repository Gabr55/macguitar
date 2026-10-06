//! Events the application listens to: keys, playback, the window.

use iced::{Subscription, keyboard, stream, system, window};

use crate::ui::app::App;
use crate::ui::app::message::Message;
use iced::futures::{SinkExt, Stream};
use iced::keyboard::key::Named::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, F11};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::Notify;

impl App {
    #[allow(clippy::unused_self)]
    pub(super) fn audio_player_beat_subscription(
        current_tick: Arc<AtomicU32>,
        beat_notify: Arc<Notify>,
    ) -> impl Stream<Item = Message> {
        stream::channel(1, async move |mut output| {
            loop {
                beat_notify.notified().await;
                let tick = current_tick.load(Ordering::Acquire);
                // the receiver is gone once the application closes
                if output.send(Message::FocusTick(tick)).await.is_err() {
                    break;
                }
            }
        })
    }

    pub(super) fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = Vec::with_capacity(4);

        // keyboard event subscription
        let keyboard_subscription = keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed {
                key,
                modified_key,
                modifiers,
                ..
            } = event
            else {
                return None;
            };
            // Ctrl+Cmd+F, the fullscreen shortcut of macOS, read on the key
            // itself: the modifiers change the character it types
            if modifiers.control()
                && modifiers.logo()
                && matches!(key.as_ref(), keyboard::Key::Character(c) if c.eq_ignore_ascii_case("f"))
            {
                return Some(Message::ToggleFullscreen);
            }
            // Cmd belongs to the system (Cmd+M minimizes, Cmd+H hides...)
            if modifiers.logo() {
                return None;
            }
            match modified_key.as_ref() {
                keyboard::Key::Named(keyboard::key::Named::Space) => Some(Message::PlayPause),
                keyboard::Key::Named(ArrowUp) if modifiers.control() => {
                    Some(Message::IncreaseTempo)
                }
                keyboard::Key::Named(ArrowDown) if modifiers.control() => {
                    Some(Message::DecreaseTempo)
                }
                keyboard::Key::Named(ArrowLeft) => Some(Message::PreviousMeasure),
                keyboard::Key::Named(ArrowRight) => Some(Message::NextMeasure),
                keyboard::Key::Character(c) if c.eq_ignore_ascii_case("s") => {
                    Some(Message::ToggleSolo)
                }
                keyboard::Key::Character(c) if c.eq_ignore_ascii_case("m") => {
                    Some(Message::ToggleMute)
                }
                keyboard::Key::Named(F11) => Some(Message::ToggleFullscreen),
                keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Escape),
                keyboard::Key::Character(c) if c.eq_ignore_ascii_case("l") => {
                    Some(Message::ToggleLoop)
                }
                _ => None,
            }
        });
        subscriptions.push(keyboard_subscription);

        // next beat notifier subscription
        subscriptions.push(Subscription::run_with(
            BeatSubscriptionData(self.current_tick.clone(), self.beat_notify.clone()),
            |data| Self::audio_player_beat_subscription(data.0.clone(), data.1.clone()),
        ));

        subscriptions.push(system::theme_changes().map(Message::SystemThemeChanged));

        // the right button ends a loop being drawn wherever it is let go
        subscriptions.push(iced::event::listen_with(|event, _status, _window| {
            matches!(
                event,
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                    iced::mouse::Button::Right
                ))
            )
            .then_some(Message::LoopDrawn)
        }));

        let window_resized = window::resize_events().map(|_| Message::WindowResized);
        subscriptions.push(window_resized);

        let file_dropped = window::events().filter_map(|(_, event)| {
            if let window::Event::FileDropped(path) = event {
                Some(Message::OpenFile(path))
            } else {
                None
            }
        });
        subscriptions.push(file_dropped);

        Subscription::batch(subscriptions)
    }
}

struct BeatSubscriptionData(Arc<AtomicU32>, Arc<Notify>);

impl std::hash::Hash for BeatSubscriptionData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        "beat-subscription".hash(state); // The ID is constant
    }
}

impl PartialEq for BeatSubscriptionData {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for BeatSubscriptionData {}
