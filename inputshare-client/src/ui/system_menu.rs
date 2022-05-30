use std::ptr;
use native_windows_gui::{ControlHandle, NwgError};
use winapi::shared::minwindef::FALSE;
use winapi::um::winuser::GetSystemMenu;

#[derive(Default, PartialEq, Eq)]
pub struct SystemMenu {
    pub handle: ControlHandle
}

impl From<&SystemMenu> for ControlHandle {
    fn from(sys: &SystemMenu) -> Self {
        sys.handle
    }
}

impl SystemMenu {
    pub fn builder<'a>() -> SystemMenuBuilder {
        SystemMenuBuilder {
            parent: None
        }
    }
}

pub struct SystemMenuBuilder {
    parent: Option<ControlHandle>
}

impl<'a> SystemMenuBuilder {

    pub fn parent<C: Into<ControlHandle>>(mut self, p: C) -> Self {
        self.parent = Some(p.into());
        self
    }

    pub fn build(self, menu: &mut SystemMenu) -> Result<(), NwgError> {
        if self.parent.is_none() {
            return Err(NwgError::no_parent_menu());
        }

        let hmenu = unsafe {
            GetSystemMenu(self.parent.unwrap().hwnd().unwrap(), FALSE)
        };
        menu.handle = ControlHandle::Menu(ptr::null_mut(), hmenu);

        Ok(())
    }
}
