use log::{debug, info};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::engine::os::{Platform, Window};

pub mod os;

pub struct EngineState {
    active_windows: HashMap<u32, Arc<dyn Window>>,
    dying_windows: HashMap<u32, Arc<dyn Window>>,
    window_id_counter: AtomicU32,
    platform: Arc<dyn Platform>,
}

pub struct Engine {
    state: RefCell<EngineState>,
    app: Box<RefCell<dyn ApplicationHandler>>,
}

impl Engine {
    pub fn new<A: ApplicationHandler + 'static>(app: A) -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            state: RefCell::new(EngineState::new()?),
            app: Box::new(RefCell::new(app)),
        }))
    }

    pub fn borrow(&self) -> Ref<'_, EngineState> {
        self.state.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, EngineState> {
        self.state.borrow_mut()
    }

    pub fn borrow_app(&self) -> Ref<'_, dyn ApplicationHandler> {
        self.app.borrow()
    }

    pub fn borrow_app_mut(&self) -> RefMut<'_, dyn ApplicationHandler> {
        self.app.borrow_mut()
    }

    // pub fn simple_message_loop(engine: Arc<Engine>) {
    //     let mut message: MaybeUninit<MSG> = MaybeUninit::uninit();
    //
    //     unsafe {
    //         loop {
    //             while PeekMessageW(
    //                 message.as_mut_ptr(),
    //                 HWND(std::ptr::null_mut()),
    //                 0,
    //                 0,
    //                 PM_REMOVE,
    //             )
    //             .0 != 0
    //             {
    //                 DispatchMessageW(message.as_ptr());
    //                 _ = TranslateMessage(message.as_ptr());
    //             }
    //
    //             engine.update();
    //
    //             if !engine.borrow().any_windows_remaining() {
    //                 break;
    //             }
    //         }
    //     }
    // }

    pub fn update(self: &Arc<Engine>) {
        self.borrow_mut().finish_killing_windows();
    }

    // pub fn recv_window_event(
    //     self: &Arc<Self>,
    //     window_id: u32,
    //     hwnd: HWND,
    //     message: u32,
    //     wparam: WPARAM,
    //     lparam: LPARAM,
    // ) -> Option<isize> {
    //     match message {
    //         WM_CLOSE => {
    //             if self.state.borrow().is_window_active(&window_id) {
    //                 if self.app.borrow_mut().on_close_request(window_id, self) {
    //                     self.state.borrow_mut().begin_kill_window(window_id);
    //                 }
    //             }
    //
    //             Some(0)
    //         }
    //         _ => None,
    //     }
    // }


}

impl EngineState {
    pub fn new() -> anyhow::Result<Self> {
        // let windows_hinstance = unsafe { HINSTANCE(GetModuleHandleW(PCWSTR::null())?.0) };
        //
        // let window_color = unsafe {
        //     let settings = UISettings::new()?;
        //     settings.GetColorValue(UIColorType::Background)?
        // };
        //
        // let hbrush_background = unsafe {
        //     info!(
        //         "color: rgba({:?}, {:?}, {:?}, {:?})",
        //         window_color.R, window_color.G, window_color.B, window_color.A
        //     );
        //     CreateSolidBrush(COLORREF(
        //         ((window_color.B as u32) << 16)
        //             | ((window_color.G as u32) << 8)
        //             | (window_color.R as u32),
        //     ))
        // };

        Ok(Self {
            active_windows: HashMap::new(),
            dying_windows: HashMap::new(),
            window_id_counter: AtomicU32::new(0),
            platform: os::create_platform(),
        })
    }

    pub unsafe fn unregister_window_class(&self, name: &U16CStr) {
        unsafe {
            _ = UnregisterClassW(PCWSTR(name.as_ptr()), self.windows_hinstance);
        }
    }

    pub unsafe fn register_window_class(
        &mut self,
        mut window_class_info: WNDCLASSEXW,
    ) -> U16CString {
        let mut class_name: U16CString = U16CString::default();
        if window_class_info.lpszClassName.is_null() {
            let name = format!(
                "Neuron_WindowClass__{:?}",
                self.window_class_id_counter.fetch_add(1, Ordering::SeqCst)
            );
            class_name = U16CString::from_str(name.as_str()).unwrap();
            window_class_info.lpszClassName = PCWSTR(class_name.as_ptr());
        } else {
            class_name = U16CString::from_ptr_str(window_class_info.lpszClassName.as_ptr());
        }
        window_class_info.hInstance = self.windows_hinstance;

        if window_class_info.hbrBackground == HBRUSH::default() {
            window_class_info.hbrBackground = self.hbrush_background;
        }

        unsafe {
            _ = RegisterClassExW(&window_class_info);
        }

        class_name
    }

    pub fn generate_window_id(&mut self) -> u32 {
        self.window_id_counter.fetch_add(1, Ordering::SeqCst)
    }

    pub fn is_window_active(&self, window_id: &u32) -> bool {
        self.active_windows.contains_key(window_id)
    }

    pub fn begin_kill_window(&mut self, window_id: u32) {
        self.dying_windows
            .insert(window_id, self.active_windows.remove(&window_id).unwrap());
        info!("Killing window {:?}", window_id);
    }

    pub(self) fn insert_active_window(&mut self, window_id: u32, window: Arc<Window>) {
        self.active_windows.insert(window_id, window);
    }

    pub fn any_windows_remaining(&self) -> bool {
        self.dying_windows.len() > 0 || self.active_windows.len() > 0
    }

    pub fn finish_killing_windows(&mut self) {
        if self.dying_windows.len() > 0 {
            let dying_windows = self.dying_windows.drain().collect::<Vec<_>>();

            for (window_id, mut window) in dying_windows {
                if Arc::strong_count(&window) > 1 {
                    self.dying_windows.insert(window_id, window);
                    debug!(
                        "Re-added {:?} to dying_windows due to remaining reference count",
                        window_id
                    );
                } else {
                    let wc = window.window_class().to_owned();
                    drop(window);
                    unsafe { self.unregister_window_class(&wc) }
                    info!("Finished killing window {:?}", window_id);
                }
            }
        }
    }
}

pub trait ApplicationHandler : OsEventHandler {}