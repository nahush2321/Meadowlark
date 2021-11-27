use vizia::*;

use crate::state::{AppEvent, StateSystem, ui_state::{TempoMapUiState, UiState}};


const STYLE: &str = r#"
    slider {
        height: 10px;
        top: 1s;
        bottom: 1s;
        width: 1s;
        background-color: #dfdfdf;
        border-radius: 4.5px;
    }

    slider .active {
        background-color: #f74c00;
        border-radius: 4.5px;
    }

    slider .thumb {
        background-color: white;
        top: 1s;
        bottom: 1s;
        border-radius: 14.5px;
        border-color: #757575;
        border-width: 1px;
        width: 20px;
        height: 20px;
    }
"#;

pub fn tempo_controls(cx: &mut Context) {

    cx.add_theme(STYLE);

    VStack::new(cx, |cx|{
        Label::new(cx, "TEMPO");
        HStack::new(cx, |cx|{
            Binding::new(cx, StateSystem::ui_state.then(UiState::tempo_map).then(TempoMapUiState::bpm), |cx, bpm|{
                Label::new(cx, &format!("{:.*}", 2, bpm.get(cx))).width(Pixels(60.0));
            });
            let init = (130.0 - 20.0) / 180.0;
            Slider::new(cx, init, Orientation::Horizontal)
                .on_changing(cx, |cx, val| {
                    cx.emit(AppEvent::SetBpm((val as f64* 180.0) + 20.0));
                })
                .width(Pixels(100.0))
                .left(Pixels(5.0))
                .right(Pixels(5.0));
            Label::new(cx, "TAP").width(Pixels(50.0));
            Label::new(cx, "4/4").width(Pixels(50.0));
            Label::new(cx, "Groove").width(Pixels(50.0));
        }).child_top(Stretch(1.0)).child_bottom(Stretch(1.0));
    }).child_space(Pixels(10.0));
}