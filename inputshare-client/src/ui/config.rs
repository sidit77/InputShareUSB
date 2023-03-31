use std::time::Duration;

use druid::widget::{Button, Controller, CrossAxisAlignment, Flex, Label, Scroll, TextBox};
use druid::{Color, Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, TimerToken, UpdateCtx, Widget, WidgetExt};
use druid_material_icons::normal::action::SEARCH;
use druid_material_icons::normal::content::ADD;
use yawi::VirtualKey;

use crate::model::{AppState, Config, Hotkey};
use crate::runtime::ExtEventSinkCallback;
use crate::ui::actions::{open_key_picker, start_search};
use crate::ui::widget::{Icon, WidgetButton, WrappingList};
use crate::utils::keyset::VirtualKeySet;

pub fn ui() -> impl Widget<Config> + 'static {
    let host = host_ui()
        .lens(Config::host_address);
    let blacklist = blacklist_ui()
        .lens(Config::blacklist);
    let hotkey = hotkey_ui()
        .lens(Config::hotkey);
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("Host"))
        .with_child(host)
        .with_default_spacer()
        .with_child(Label::new("Hotkey"))
        .with_child(hotkey)
        .with_default_spacer()
        .with_child(Label::new("Blacklist"))
        .with_flex_child(blacklist, 1.0)
        .controller(SaveController::default())
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

const CONFIG_SAVE_DELAY: Duration = Duration::from_secs(10);

#[derive(Debug, Default, Copy, Clone)]
struct SaveController {
    last_timer: Option<TimerToken>
}

impl<W: Widget<Config>> Controller<Config, W> for SaveController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut Config, env: &Env) {
        let save = match event {
            Event::Timer(token) => Some(*token) == self.last_timer,
            Event::WindowCloseRequested => self.last_timer.is_some(),
            _ => false
        };
        if save {
            self.last_timer.take();
            let config = data.clone();
            ctx.add_rt_callback(move |rt, _| {
                rt.runtime.spawn_blocking(move || match config.save() {
                    Ok(_) => tracing::trace!("Successfully saved config"),
                    Err(err) => tracing::warn!("Failed to save config: {}", err)
                });
            })
        }
        child.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, child: &mut W, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Config, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            let handle = ctx.get_external_handle();
            handle.clone().add_rt_callback(move |rt, _| {
                rt.runtime.spawn_blocking(move || match Config::load() {
                    Ok(config) => handle.add_idle_callback(move |data: &mut AppState| {
                        tracing::trace!("Successfully loaded config");
                        data.config = config;
                    }),
                    Err(err) => tracing::warn!("Failed to load config: {}", err)
                });
            });
        }
        child.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &Config, data: &Config, env: &Env) {
        if !old_data.same(data) {
            self.last_timer = Some(ctx.request_timer(CONFIG_SAVE_DELAY));
        }
        child.update(ctx, old_data, data, env)
    }
}
