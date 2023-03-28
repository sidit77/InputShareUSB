use std::mem::Discriminant;

use druid::widget::{BackgroundBrush, Button, Flex, Label, List, ViewSwitcher};
use druid::{Color, Lens, LensExt, Widget, WidgetExt};
use druid::im::Vector;

use crate::model::{PopupType, SearchResult};
use crate::runtime::ExtEventSinkCallback;

pub fn ui() -> impl Widget<PopupType> + 'static {
    ViewSwitcher::<PopupType, Discriminant<PopupType>>::new(
        |t, _| std::mem::discriminant(t),
        |_, s, _| match s {
            PopupType::Searching(_) =>
                search_popup_ui()
                    .lens(search_lens())
                    .boxed(),
            PopupType::PressKey =>
                key_popup_ui()
                    .boxed()
        }
    )
    .center()
    .background(BackgroundBrush::Color(Color::rgba8(0, 0, 0, 128)))
    .expand()
}

fn key_popup_ui() -> impl Widget<PopupType> + 'static {
    Label::new("Press any key")
        .center()
        .fix_size(200.0, 100.0)
        .background(druid::theme::BACKGROUND_DARK)
        .rounded(5.0)
}

fn search_popup_ui() -> impl Widget<Vector<SearchResult>> + 'static {
    Flex::column()
        .with_child(List::new(|| Label::dynamic(|res: &SearchResult, _| res.addrs.to_string())))
        .with_child(Button::new("Back").on_click(|ctx, _, _| {
            ctx.add_rt_callback(|rt, data| {
                if let Some(t) = rt.mdns.take() { t.abort() }
                data.popup = None
            })
        }))
        .center()
        .fix_size(200.0, 100.0)
        .background(druid::theme::BACKGROUND_DARK)
        .rounded(5.0)
}

fn search_lens() -> impl Lens<PopupType, Vector<SearchResult>> {
    druid::lens::Identity.map(
        |data| match data {
            PopupType::Searching(s) => s.clone(),
            _ => unreachable!()
        },
        |data, vec| *data = PopupType::Searching(vec)
    )
}