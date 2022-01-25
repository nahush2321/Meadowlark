use vizia::*;

use crate::state::{
    ui_state::{TimelineTransportUiState, UiState},
    AppEvent, StateSystem,
};

const ICON_STOP: &str = "\u{25a0}";
const ICON_PLAY: &str = "\u{25b6}";
const ICON_PAUSE: &str = "\u{2389}";

pub fn transport_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        Label::new(cx, "TRANSPORT");
        HStack::new(cx, |cx| {
            Binding::new(
                cx,
                StateSystem::ui_state
                    .then(UiState::timeline_transport)
                    .then(TimelineTransportUiState::playhead),
                |cx, playhead| {
                    let beats = playhead.get(cx).as_beats_f64();
                    Label::new(
                        cx,
                        &format!(
                            "{}.{}.{}",
                            (beats / 4.0) as i32 + 1,
                            (beats as i32 % 4) + 1,
                            ((beats * 4.0) as i32 % 4) + 1
                        ),
                    )
                    .class("transport_playhead")
                    .background_color(Color::rgba(255, 255, 0, 0));
                },
            );

            Binding::new(
                cx,
                StateSystem::ui_state
                    .then(UiState::timeline_transport)
                    .then(TimelineTransportUiState::is_playing),
                |cx, is_playing| {
                    // Play/Pause button
                    Checkbox::with_icons(cx, *is_playing.get(cx), ICON_PAUSE, ICON_PLAY)
                        .on_checked(cx, |cx| cx.emit(AppEvent::Play))
                        .on_unchecked(cx, |cx| cx.emit(AppEvent::Pause))
                        .class("play_button");
                },
            );

            Button::new(
                cx,
                |cx| cx.emit(AppEvent::Stop),
                |cx| {
                    Label::new(cx, ICON_STOP)
                        .font("icons")
                        .font_size(24.0)
                        .width(Stretch(1.0))
                        .child_space(Stretch(1.0))
                },
            )
            .width(Pixels(30.0))
            .height(Pixels(30.0))
            .background_color(Color::rgba(255, 0, 0, 0));
        })
        .height(Pixels(30.0))
        .col_between(Pixels(5.0))
        .child_top(Stretch(1.0))
        .child_bottom(Stretch(1.0));

    })
    .child_space(Pixels(10.0));
}