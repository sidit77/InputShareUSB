use druid::{BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, RenderContext, Size, UnitPoint, UpdateCtx, Widget, WidgetExt, WidgetPod};
use druid::debug_state::DebugState;
use druid::widget::BackgroundBrush;
use tracing::instrument;

pub struct Popup<T> {
    #[allow(clippy::type_complexity)]
    closure: Box<dyn Fn(&T, &Env) -> bool>,
    overlay: WidgetPod<T, Box<dyn Widget<T>>>,
    base: WidgetPod<T, Disabled<T, Box<dyn Widget<T>>>>,
    current: bool,
}

impl<T: Data> Popup<T> {
    pub fn new(
        closure: impl Fn(&T, &Env) -> bool + 'static,
        overlay: impl Widget<T> + 'static,
        base: impl Widget<T> + 'static,
    ) -> Popup<T> {
        Popup {
            closure: Box::new(closure),
            overlay: WidgetPod::new(overlay).boxed(),
            base: WidgetPod::new(Disabled::new(base.boxed())),
            current: false,
        }
    }
}

impl<T: Data> Widget<T> for Popup<T> {
    #[instrument(name = "Popup", level = "trace", skip(self, ctx, event, data, env), fields(branch = self.current))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if event.should_propagate_to_hidden() || self.current {
            self.overlay.event(ctx, event, data, env);
        }
        self.base.event(ctx, event, data, env);
    }

    #[instrument(name = "Popup", level = "trace", skip(self, ctx, event, data, env), fields(branch = self.current))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.current = (self.closure)(data, env);
            self.base.widget_mut().disabled = self.current;
        }

        if event.should_propagate_to_hidden() || self.current {
            self.overlay.lifecycle(ctx, event, data, env);
        }
        self.base.lifecycle(ctx, event, data, env);
    }

    #[instrument(name = "Popup", level = "trace", skip(self, ctx, _old_data, data, env), fields(branch = self.current))]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let current = (self.closure)(data, env);
        if current != self.current {
            self.current = current;
            self.base.widget_mut().disabled = self.current;
            ctx.children_changed();
        }
        self.base.update(ctx, data, env)
    }

    #[instrument(name = "Popup", level = "trace", skip(self, ctx, bc, data, env), fields(branch = self.current))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let base_size = self.base.layout(ctx, bc, data, env);
        self.overlay.layout(ctx, &BoxConstraints::new(Size::ZERO, base_size), data, env);

        let mut paint_rect = Rect::ZERO;

        let remaining = base_size - self.base.layout_rect().size();
        let origin = UnitPoint::CENTER.resolve(remaining.to_rect());
        self.base.set_origin(ctx, origin);
        paint_rect = paint_rect.union(self.base.paint_rect());

        let remaining = base_size - self.overlay.layout_rect().size();
        let origin = UnitPoint::CENTER.resolve(remaining.to_rect());
        self.overlay.set_origin(ctx, origin);
        paint_rect = paint_rect.union(self.overlay.paint_rect());

        ctx.set_paint_insets(paint_rect - base_size.to_rect());
        ctx.set_baseline_offset(self.base.baseline_offset());

        base_size
    }

    #[instrument(name = "Popup", level = "trace", skip(self, ctx, data, env), fields(branch = self.current))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.base.paint(ctx, data, env);
        if self.current {
            self.overlay.paint(ctx, data, env);
        }
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.base.widget().debug_state(data), self.overlay.widget().debug_state(data)],
            ..Default::default()
        }
    }
}

pub struct Disabled<T, W> {
    child: WidgetPod<T, W>,
    disabled: bool,
}

impl<T: Data, W: Widget<T>> Disabled<T, W> {
    pub fn new(widget: W) -> Self {
        Disabled {
            child: WidgetPod::new(widget),
            disabled: true,
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Disabled<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            ctx.set_disabled(self.disabled);
        }
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        ctx.set_disabled(self.disabled);
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        self.child.set_origin(ctx, Point::ZERO);
        ctx.set_baseline_offset(self.child.baseline_offset());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);

        if self.disabled {
            let mut foreground = BackgroundBrush::Color(Color::rgba8(0, 0, 0, 128));
            let panel = ctx.size().to_rect();

            ctx.with_save(|ctx| {
                ctx.clip(panel);
                foreground.paint(ctx, data, env);
            });
        }
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.widget().debug_state(data)],
            ..Default::default()
        }
    }
}

