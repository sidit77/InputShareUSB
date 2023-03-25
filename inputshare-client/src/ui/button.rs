use druid::widget::prelude::*;
use druid::widget::{Click, ControllerHost};
use druid::{theme, Data, LinearGradient, UnitPoint, WidgetPod};

pub struct WidgetButton<T, W: Widget<T>> {
    inner: WidgetPod<T, W>,
}

impl<T: Data, W: Widget<T>> WidgetButton<T, W> {

    pub fn new(inner: W) -> Self {
        WidgetButton {
            inner: WidgetPod::new(inner)
        }
    }

    pub fn on_click(self, f: impl Fn(&mut EventCtx, &mut T, &Env) + 'static) -> ControllerHost<Self, Click<T>> {
        ControllerHost::new(self, Click::new(f))
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for WidgetButton<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                if !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() && !ctx.is_disabled() {
                    ctx.request_paint();
                }
                ctx.set_active(false);
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::HotChanged(_) | LifeCycle::DisabledChanged(_) = event {
            ctx.request_paint();
        }
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Button");
        let label_bc = bc.loosen();
        //self.icon_size = self.icon.layout(ctx, &label_bc, data, env);
        //bc.constrain(self.icon_size)
        bc.constrain(self.inner.layout(ctx, &label_bc, data, env))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let is_active = ctx.is_active() && !ctx.is_disabled();
        let is_hot = ctx.is_hot();
        let size = ctx.size();
        let stroke_width = env.get(theme::BUTTON_BORDER_WIDTH);

        let rounded_rect = size
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));

        let bg_gradient = if ctx.is_disabled() {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (
                    env.get(theme::DISABLED_BUTTON_LIGHT),
                    env.get(theme::DISABLED_BUTTON_DARK),
                ),
            )
        } else if is_active {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::BUTTON_DARK), env.get(theme::BUTTON_LIGHT)),
            )
        } else {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::BUTTON_LIGHT), env.get(theme::BUTTON_DARK)),
            )
        };

        let border_color = if is_hot && !ctx.is_disabled() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        ctx.stroke(rounded_rect, &border_color, stroke_width);

        ctx.fill(rounded_rect, &bg_gradient);

        //let label_offset = (size.to_vec2() - self.icon_size.to_vec2()) / 2.0;

        ctx.with_save(|ctx| {
            //ctx.transform(Affine::translate(label_offset));
            self.inner.paint(ctx, data, env);
        });
    }

}
