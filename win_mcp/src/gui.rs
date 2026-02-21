use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::Graphics::Gdi::*,
    Win32::UI::Shell::*,
    Win32::Graphics::Dwm::*,
};

const WM_TRAY: u32 = WM_USER + 1;
const TRAY_ICON_ID: u32 = 1;

static mut VISION_STATUS: &str = "Unknown";
static mut INPUT_STATUS: &str = "Unknown";
static mut INSPECTOR_STATUS: &str = "Unknown";

pub struct JarvisGui {
    hwnd: HWND,
}

impl JarvisGui {
    pub fn new(vision: &'static str, input: &'static str, inspector: &'static str) -> Result<Self> {
        unsafe {
            VISION_STATUS = vision;
            INPUT_STATUS = input;
            INSPECTOR_STATUS = inspector;
        }

        let instance = unsafe { GetModuleHandleW(None)? };
        let window_class = w!("JarvisGuiClass");

        let wc = WNDCLASSW {
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
            hInstance: instance.into(),
            lpszClassName: window_class,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(Self::wnd_proc),
            ..Default::default()
        };

        unsafe { RegisterClassW(&wc) };

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                window_class,
                w!("Jarvis AI Agent"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT, CW_USEDEFAULT, 400, 300,
                None, None, instance, None,
            )?
        };

        if hwnd.0.is_null() {
            return Err(Error::from_win32());
        }

        // Apply Mica effect (DWMSBT_MAINWINDOW = 2)
        let mica_value: i32 = 2;
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &mica_value as *const _ as _,
                std::mem::size_of::<i32>() as u32,
            ).ok();
        }

        // Dark mode support (DWMWA_USE_IMMERSIVE_DARK_MODE = 20)
        let dark_mode: i32 = 1;
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const _ as _,
                std::mem::size_of::<i32>() as u32,
            ).ok();
        }

        let gui = Self { hwnd };
        gui.setup_tray()?;
        
        Ok(gui)
    }

    fn setup_tray(&self) -> Result<()> {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY,
            hIcon: unsafe { LoadIconW(None, IDI_APPLICATION)? },
            ..Default::default()
        };
        // Set tooltip
        let tip = w!("Jarvis AI Agent is running");
        let len = unsafe { tip.as_wide().len().min(127) };
        unsafe { nid.szTip[..len].copy_from_slice(&tip.as_wide()[..len]) };

        unsafe { Shell_NotifyIconW(NIM_ADD, &nid).ok()? };
        Ok(())
    }

    pub fn run(&self) {
        let mut message = MSG::default();
        unsafe {
            while GetMessageW(&mut message, None, 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_TRAY => {
                match lparam.0 as u32 {
                    WM_LBUTTONDBLCLK => unsafe {
                        let _ = ShowWindow(hwnd, SW_SHOW);
                        let _ = SetForegroundWindow(hwnd);
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_SIZE => {
                if wparam == WPARAM(SIZE_MINIMIZED as usize) {
                    unsafe { let _ = ShowWindow(hwnd, SW_HIDE); };
                }
                LRESULT(0)
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
                
                unsafe {
                    let _ = SetBkMode(hdc, TRANSPARENT);
                    let _ = SetTextColor(hdc, COLORREF(0xFFFFFF)); // White for dark mode

                    let _ = TextOutW(hdc, 20, 20, w!("Jarvis AI Agent").as_wide());
                    
                    let (v_status, i_status, u_status) = unsafe {
                        (VISION_STATUS, INPUT_STATUS, INSPECTOR_STATUS)
                    };

                    let v_fmt = format!("Vision: {}", v_status);
                    let i_fmt = format!("Input:  {}", i_status);
                    let u_fmt = format!("UI Insp: {}", u_status);

                    let v_wide: Vec<u16> = v_fmt.encode_utf16().collect();
                    let i_wide: Vec<u16> = i_fmt.encode_utf16().collect();
                    let u_wide: Vec<u16> = u_fmt.encode_utf16().collect();

                    let _ = TextOutW(hdc, 20, 60, &v_wide);
                    let _ = TextOutW(hdc, 20, 90, &i_wide);
                    let _ = TextOutW(hdc, 20, 120, &u_wide);
                    
                    let _ = EndPaint(hwnd, &ps);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                let nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: TRAY_ICON_ID,
                    ..Default::default()
                };
                unsafe {
                    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}
