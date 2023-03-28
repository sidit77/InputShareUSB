use druid::widget::prelude::*;
use druid::{Affine, Color, KeyOrValue, Size};
use druid_material_icons::IconPaths;

use crate::ui::widget::theme;

#[derive(Debug, Clone)]
pub struct Icon {
    paths: IconPaths,
    color: KeyOrValue<Color>
}

impl Icon {
    #[inline]
    pub fn new(paths: IconPaths) -> Self {
        Self {
            paths,
            color: KeyOrValue::from(theme::ICON_COLOR)
        }
    }

    #[allow(dead_code)]
    pub fn with_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.color = color.into();
        self
    }
}

impl From<IconPaths> for Icon {
    fn from(path: IconPaths) -> Self {
        Icon::new(path)
    }
}

impl<T: Data> Widget<T> for Icon {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {
        // no events
    }
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &T, _env: &Env) {
        #[allow(clippy::single_match)]
        match event {
            LifeCycle::DisabledChanged(_) => {
                ctx.request_layout();
            }
            _ => {}
        }
    }
    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {
        // no update
    }
    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        // Try to preserve aspect ratio if possible, but if not then allow non-uniform scaling.
        bc.constrain_aspect_ratio(self.paths.size.aspect_ratio(), self.paths.size.width)
    }
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let color = match !ctx.is_disabled() {
            true => self.color.resolve(env),
            false => env.get(theme::DISABLED_ICON_COLOR)
        };
        let Size { width, height } = ctx.size();
        let Size {
            width: icon_width,
            height: icon_height
        } = self.paths.size;
        ctx.transform(Affine::scale_non_uniform(width * icon_width.recip(), height * icon_height.recip()));
        for path in self.paths.paths {
            ctx.fill(path, &color);
        }
    }
}
