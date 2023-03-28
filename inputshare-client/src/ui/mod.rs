use druid::widget::{Button, CrossAxisAlignment, Flex, Label, Maybe, Scroll, TextBox};
use druid::{Color, Widget, WidgetExt};
use druid_material_icons::normal::action::SEARCH;
use druid_material_icons::normal::content::ADD;
use yawi::VirtualKey;

use crate::model::{AppState, Config, ConnectionState, Hotkey};
use crate::ui::actions::{initiate_connection, start_search, open_key_picker};
use crate::ui::widget::{Icon, WidgetButton, WrappingList};
use crate::utils::keyset::VirtualKeySet;

pub mod actions;
pub mod popup;
pub mod widget;

pub fn ui() -> impl Widget<AppState> {
    let popup = Maybe::or_empty(popup::ui)
        .lens(AppState::popup);
    druid::widget::ZStack::new(main_ui())
        .with_centered_child(popup)
}

fn main_ui() -> impl Widget<AppState> + 'static {
    let config = config_ui()
        .lens(AppState::config)
        .disabled_if(|data, _| data.connection_state != ConnectionState::Disconnected);
    Flex::column()
        .with_flex_child(config, 1.0)
        .with_spacer(5.0)
        .with_child(status_ui())
        .padding(5.0)
}

fn config_ui() -> impl Widget<Config> + 'static {
    let host = host_ui()
        .lens(Config::host_address);
    let blacklist = blacklist_ui()
        .lens(Config::blacklist);
    let hotkey = hotkey_ui()
        .lens(Config::hotkey);
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(host)
        .with_default_spacer()
        .with_child(Label::new("Hotkey"))
        .with_child(hotkey)
        .with_default_spacer()
        .with_child(Label::new("Blacklist"))
        .with_flex_child(blacklist, 1.0)
}

fn host_ui() -> impl Widget<String> + 'static {
    let host = TextBox::new().expand_width();
    let search = WidgetButton::new(Icon::from(SEARCH)
        .padding(5.0))
        .on_click(|ctx, _, _| start_search(ctx));
    Flex::row()
        .with_flex_child(host, 1.0)
        .with_spacer(5.0)
        .with_child(search)
}

fn hotkey_ui() -> impl Widget<Hotkey> + 'static {
    let add = add_button(|data, key| {
        let hotkey = &mut data.config.hotkey;
        match key != hotkey.trigger {
            true => hotkey.modifiers.insert(key),
            false => tracing::warn!("The trigger can not be a modifier")
        }
    });
    let list = WrappingList::new(key_ui)
        .with_end(add)
        .horizontal()
        .with_spacing(2.0)
        .padding(2.0);
    let modifiers = Scroll::new(list)
        .horizontal()
        .lens(Hotkey::modifiers);
    let trigger = Button::dynamic(|data: &VirtualKey, _| data.to_string())
        .on_click(|ctx, _, _| {
            open_key_picker(ctx, |data, key| {
                let hotkey = &mut data.config.hotkey;
                hotkey.modifiers.remove(key);
                hotkey.trigger = key;
            })
        })
        .lens(Hotkey::trigger);
    Flex::row()
        .with_child(modifiers)
        .with_child(Icon::from(ADD))
        .with_child(trigger)
        .with_spacer(2.0)
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
}

fn blacklist_ui() -> impl Widget<VirtualKeySet> + 'static {
    let add = add_button(|data, key| data.config.blacklist.insert(key));
    let list = WrappingList::new(key_ui)
        .with_end(add)
        .horizontal()
        .with_spacing(2.0)
        .padding(2.0);
    Scroll::new(list)
        .vertical()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
}

fn add_button(setter: fn(&mut AppState, VirtualKey)) -> impl Widget<()> + 'static {
    Button::new("+")
        .env_scope(|env, _| {
            env.set(druid::theme::BUTTON_DARK, Color::TRANSPARENT);
            env.set(druid::theme::BUTTON_LIGHT, Color::TRANSPARENT);
            env.set(druid::theme::DISABLED_BUTTON_DARK, Color::TRANSPARENT);
            env.set(druid::theme::DISABLED_BUTTON_LIGHT, Color::TRANSPARENT);
        })
        .on_click(move |ctx, _, _| open_key_picker(ctx, setter))
}

fn key_ui() -> impl Widget<(VirtualKeySet, VirtualKey)> + 'static {
    Button::<(VirtualKeySet, VirtualKey)>::dynamic(|(_, key): &(_, VirtualKey), _| key.to_string())
        .on_click(|_, (set, key), _| set.remove(*key))
}

fn status_ui() -> impl Widget<AppState> + 'static {
    let status = Label::dynamic(|data: &AppState, _| format!("{:?}", data.connection_state))
        .center()
        .border(druid::theme::BORDER_DARK, 2.0)
        .rounded(2.0)
        .expand();
    let button = Button::new("Connect")
        .fix_width(100.0)
        .on_click(|ctx, _, _| initiate_connection(ctx))
        .expand_height();
    Flex::row()
        .with_flex_child(status, 1.0)
        .with_spacer(5.0)
        .with_child(button)
        .fix_height(50.0)
}
