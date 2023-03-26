use std::cmp::Ordering;
use druid::debug_state::DebugState;
use druid::widget::{Axis, ListIter};
use druid::{BoxConstraints, Data, Env, Event, EventCtx, KeyOrValue, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, Size, UpdateCtx, Widget, WidgetPod};

use tracing::{instrument, trace};


/// A list widget for a variable-size collection of items.
pub struct WrappingList<T> {
    closure: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    children: Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
    axis: Axis,
    spacing: KeyOrValue<f64>,
    old_bc: BoxConstraints,
}

impl<T: Data> WrappingList<T> {
    /// Create a new list widget. Closure will be called every time when a new child
    /// needs to be constructed.
    pub fn new<W: Widget<T> + 'static>(closure: impl Fn() -> W + 'static) -> Self {
        WrappingList {
            closure: Box::new(move || Box::new(closure())),
            children: Vec::new(),
            axis: Axis::Vertical,
            spacing: KeyOrValue::Concrete(0.),
            old_bc: BoxConstraints::tight(Size::ZERO),
        }
    }

    /// Sets the widget to display the list horizontally, not vertically.
    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    /// Set the spacing between elements.
    pub fn with_spacing(mut self, spacing: impl Into<KeyOrValue<f64>>) -> Self {
        self.spacing = spacing.into();
        self
    }

    /// Set the spacing between elements.
    pub fn set_spacing(&mut self, spacing: impl Into<KeyOrValue<f64>>) -> &mut Self {
        self.spacing = spacing.into();
        self
    }

    /// When the widget is created or the data changes, create or remove children as needed
    ///
    /// Returns `true` if children were added or removed.
    fn update_child_count(&mut self, data: &impl ListIter<T>, _env: &Env) -> bool {
        let len = self.children.len();
        match len.cmp(&data.data_len()) {
            Ordering::Greater => self.children.truncate(data.data_len()),
            Ordering::Less => data.for_each(|_, i| {
                if i >= len {
                    let child = WidgetPod::new((self.closure)());
                    self.children.push(child);
                }
            }),
            Ordering::Equal => (),
        }
        len != data.data_len()
    }
}


impl<C: Data, T: ListIter<C>> Widget<T> for WrappingList<C> {
    #[instrument(name = "WrappingList", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let mut children = self.children.iter_mut();
        data.for_each_mut(|child_data, _| {
            if let Some(child) = children.next() {
                child.event(ctx, event, child_data, env);
            }
        });
    }

    #[instrument(name = "WrappingList", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            if self.update_child_count(data, env) {
                ctx.children_changed();
            }
        }

        let mut children = self.children.iter_mut();
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                child.lifecycle(ctx, event, child_data, env);
            }
        });
    }

    #[instrument(name = "WrappingList", level = "trace", skip(self, ctx, _old_data, data, env))]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        // we send update to children first, before adding or removing children;
        // this way we avoid sending update to newly added children, at the cost
        // of potentially updating children that are going to be removed.
        let mut children = self.children.iter_mut();
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                child.update(ctx, child_data, env);
            }
        });

        if self.update_child_count(data, env) {
            ctx.children_changed();
        }

        if ctx.env_key_changed(&self.spacing) {
            ctx.request_layout();
        }
    }

    #[instrument(name = "WrappingList", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let axis = self.axis;
        let spacing = self.spacing.resolve(env);
        let major_max = axis.major(bc.max());
        let mut minor = axis.minor(bc.min());
        let mut major = axis.major(bc.min());
        let mut major_pos = 0.0;
        let mut paint_rect = Rect::ZERO;

        let bc_changed = self.old_bc != *bc;
        self.old_bc = *bc;

        let mut children = self.children.iter_mut();
        let child_bc = constraints(axis, bc, 0., f64::INFINITY);


        let mut minor_offset = 0.0;
        data.for_each(|child_data, _| {
            let child = match children.next() {
                Some(child) => child,
                None => {
                    return;
                }
            };

            let child_size = if bc_changed || child.layout_requested() {
                child.layout(ctx, &child_bc, child_data, env)
            } else {
                child.layout_rect().size()
            };

            if major_pos + axis.major(child_size) >= major_max {
                minor_offset += minor + spacing;
                minor = 0.0;
                major_pos = 0.0;
            }

            let child_pos: Point = axis.pack(major_pos, minor_offset).into();
            child.set_origin(ctx, child_pos);
            paint_rect = paint_rect.union(child.paint_rect());
            minor = minor.max(axis.minor(child_size));
            major_pos += axis.major(child_size);
            major = major.max(major_pos);
            major_pos += spacing;

        });


        let my_size = bc.constrain(Size::from(axis.pack(major, minor + minor_offset)));
        let insets = paint_rect - my_size.to_rect();
        ctx.set_paint_insets(insets);
        trace!("Computed layout: size={}, insets={:?}", my_size, insets);
        my_size
    }

    #[instrument(name = "WrappingList", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let mut children = self.children.iter_mut();
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                child.paint(ctx, child_data, env);
            }
        });
    }

    fn debug_state(&self, data: &T) -> DebugState {
        let mut children = self.children.iter();
        let mut children_state = Vec::with_capacity(data.data_len());
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                children_state.push(child.widget().debug_state(child_data));
            }
        });

        DebugState {
            display_name: "WrappingList".to_string(),
            children: children_state,
            ..Default::default()
        }
    }
}

fn constraints(axis: Axis, bc: &BoxConstraints, min_major: f64, major: f64, ) -> BoxConstraints {
    match axis {
        Axis::Horizontal => BoxConstraints::new(
            Size::new(min_major, bc.min().height),
            Size::new(major, bc.max().height),
        ),
        Axis::Vertical => BoxConstraints::new(
            Size::new(bc.min().width, min_major),
            Size::new(bc.max().width, major),
        ),
    }
}