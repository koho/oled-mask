#![windows_subsystem = "windows"]
use anyhow::Result;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::fs;
use windows::Data::Xml::Dom::XmlDocument;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, CreateFontA, DeleteDC, DeleteObject, FillRect,
    GetDC, GetStockObject, ReleaseDC, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BLACK_BRUSH,
    BLENDFUNCTION, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, FF_SWISS, FW_NORMAL,
    HBRUSH, HGDIOBJ, OUT_TT_PRECIS, VARIABLE_PITCH, WHITE_BRUSH,
};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::{
    core::*, Win32::Foundation::*, Win32::System::LibraryLoader::GetModuleHandleA,
    Win32::System::WindowsProgramming::*, Win32::UI::Controls::*,
    Win32::UI::WindowsAndMessaging::*,
};

const WND_CLS_MASK: &str = "oled_mask";

fn main() -> Result<()> {
    unsafe {
        let instance = GetModuleHandleA(None)?;
        let window_class = s!("oled_mask_main_window");

        let wc = WNDCLASSA {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: instance.into(),
            lpszClassName: window_class,
            hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            ..Default::default()
        };
        let window_class_mask = HSTRING::from(WND_CLS_MASK);
        let wc_mask = WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: instance.into(),
            lpszClassName: PCWSTR(window_class_mask.as_ptr()),
            lpfnWndProc: Some(def_window_proc),
            ..Default::default()
        };

        RegisterClassA(&wc);
        RegisterClassW(&wc_mask);

        CreateWindowExA(
            WINDOW_EX_STYLE::default(),
            window_class,
            s!("OLED Mask"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            None,
            None,
            None,
            None,
        )?;

        let mut message = MSG::default();
        while GetMessageA(&mut message, None, 0, 0).into() {
            DispatchMessageA(&message);
        }
        Ok(())
    }
}

extern "system" fn wnd_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_CTLCOLORSTATIC => LRESULT(GetStockObject(WHITE_BRUSH).0 as _),
            WM_DPICHANGED => {
                let mut children: Vec<HWND> = Vec::new();
                let _ = EnumChildWindows(
                    Some(window),
                    Some(get_child_windows),
                    LPARAM(&mut children as *mut _ as _),
                );
                let _ = update_windows(window, children);
                LRESULT(0)
            }
            WM_COMMAND => {
                let code = (wparam.0 as u32 >> 16 & 0xFFFF) as u16;
                match code as u32 {
                    BN_CLICKED => {
                        let flag = match DLG_BUTTON_CHECK_STATE(
                            SendMessageA(HWND(lparam.0 as _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0
                                as u32,
                        ) {
                            BST_CHECKED => SWP_SHOWWINDOW,
                            BST_UNCHECKED => SWP_HIDEWINDOW,
                            _ => SET_WINDOW_POS_FLAGS(0),
                        };
                        let param = (lparam.0, flag);
                        let _ = EnumWindows(Some(toggle_masks), LPARAM(&param as *const _ as _));
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_CREATE => {
                SetWindowLongA(
                    window,
                    GWL_STYLE,
                    GetWindowLongA(window, GWL_STYLE)
                        & !WS_SIZEBOX.0 as i32
                        & !WS_MAXIMIZEBOX.0 as i32,
                );
                match create_checkboxes(window) {
                    Ok(rect) => {
                        let screen_width = GetSystemMetrics(SM_CXSCREEN);
                        let screen_height = GetSystemMetrics(SM_CYSCREEN);
                        let _ = SetWindowPos(
                            window,
                            None,
                            (screen_width - (rect.right - rect.left)) / 2,
                            (screen_height - (rect.bottom - rect.top)) / 2,
                            0,
                            0,
                            SWP_NOZORDER | SWP_NOSIZE,
                        );
                    }
                    Err(e) => {
                        let err_msg = HSTRING::from(e.to_string());
                        MessageBoxW(
                            Some(window),
                            PCWSTR(err_msg.as_ptr()),
                            w!("OLED Mask"),
                            MB_ICONERROR | MB_OK,
                        );
                        PostQuitMessage(0);
                    }
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}

fn update_windows(parent: HWND, handles: Vec<HWND>) -> Result<RECT> {
    unsafe {
        let dpi = GetDpiForWindow(parent);
        let margin = from96dpi(16.0, dpi);
        let mut max_cx = [0i32; 3];
        let mut checkboxes: Vec<(HWND, SIZE)> = Vec::new();
        for (i, handle) in handles.into_iter().enumerate() {
            let old_font = SendMessageA(handle, WM_GETFONT, WPARAM(0), LPARAM(0)).0;
            if old_font != 0 {
                let _ = DeleteObject(HGDIOBJ(old_font as _));
            }
            let font = CreateFontA(
                -MulDiv(9, dpi as i32, 72),
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_TT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                (VARIABLE_PITCH.0 | FF_SWISS.0) as u32,
                s!("Microsoft YaHei UI"),
            );
            SendMessageA(handle, WM_SETFONT, WPARAM(font.0 as _), LPARAM(1));
            let mut size = SIZE::default();
            SendMessageA(
                handle,
                BCM_GETIDEALSIZE,
                WPARAM(0),
                LPARAM(&mut size as *mut _ as _),
            );
            if size.cx > max_cx[i % 3] {
                max_cx[i % 3] = size.cx;
            }
            checkboxes.push((handle, size));
        }
        let mut points = [
            POINT { x: margin, y: 0 },
            POINT {
                x: max_cx[0] + 2 * margin,
                y: 0,
            },
            POINT {
                x: max_cx[0] + max_cx[1] + 3 * margin,
                y: 0,
            },
        ];
        let count = checkboxes.len() as i32;
        for (i, (checkbox, size)) in checkboxes.into_iter().enumerate() {
            let col = i % 3;
            points[col].y += margin;
            SetWindowPos(
                checkbox,
                None,
                points[col].x,
                points[col].y,
                size.cx,
                size.cy,
                SWP_NOZORDER,
            )?;
            points[col].y += size.cy;
        }
        let height = max(max(points[0].y, points[1].y), points[2].y) + margin;
        let width = max_cx.iter().sum::<i32>() + margin + min(count, 3) * margin;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        AdjustWindowRectExForDpi(
            &mut rect,
            WS_OVERLAPPEDWINDOW,
            false,
            WINDOW_EX_STYLE::default(),
            dpi,
        )?;
        SetWindowPos(
            parent,
            None,
            0,
            0,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOZORDER | SWP_NOMOVE,
        )?;
        Ok(rect)
    }
}

fn create_checkboxes(parent: HWND) -> Result<RECT> {
    unsafe {
        let mut handles: Vec<HWND> = Vec::new();
        let doc = XmlDocument::new()?;
        let contents = fs::read_to_string("annotation.xml")?;
        let xml = HSTRING::from(contents);
        doc.LoadXml(&xml)?;
        let root = doc.DocumentElement()?;
        let objects = root.GetElementsByTagName(h!("object"))?;
        let mut checkbox_indexes: HashMap<HSTRING, usize> = HashMap::new();
        for object in objects {
            let name = object.SelectSingleNode(h!("name"))?.InnerText()?;
            let bound = object.SelectSingleNode(h!("bndbox"))?;
            let rect = RECT {
                left: bound
                    .SelectSingleNode(h!("xmin"))?
                    .InnerText()?
                    .to_string()
                    .parse()?,
                top: bound
                    .SelectSingleNode(h!("ymin"))?
                    .InnerText()?
                    .to_string()
                    .parse()?,
                right: bound
                    .SelectSingleNode(h!("xmax"))?
                    .InnerText()?
                    .to_string()
                    .parse()?,
                bottom: bound
                    .SelectSingleNode(h!("ymax"))?
                    .InnerText()?
                    .to_string()
                    .parse()?,
            };
            let checkbox = if let Some(index) = checkbox_indexes.get(&name) {
                handles[*index]
            } else {
                let checkbox = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("BUTTON"),
                    PCWSTR(name.as_ptr()),
                    WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                    0,
                    0,
                    0,
                    0,
                    Some(parent),
                    None,
                    None,
                    None,
                )?;
                SendMessageA(checkbox, BM_SETCHECK, WPARAM(BST_CHECKED.0 as _), LPARAM(0));
                handles.push(checkbox);
                checkbox_indexes.insert(name, handles.len() - 1);
                checkbox
            };
            create_mask(checkbox, &rect)?;
        }
        Ok(update_windows(parent, handles)?)
    }
}

fn create_mask(owner: HWND, rect: &RECT) -> Result<HWND> {
    unsafe {
        let window_class_mask = HSTRING::from(WND_CLS_MASK);
        let mask = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            PCWSTR(window_class_mask.as_ptr()),
            w!(""),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        )?;
        SetWindowLongPtrA(mask, GWLP_USERDATA, owner.0 as _);
        let size = SIZE {
            cx: rect.right - rect.left,
            cy: rect.bottom - rect.top,
        };
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let bmp = CreateCompatibleBitmap(hdc_screen, size.cx, size.cy);
        let bmp_old = SelectObject(hdc_mem, bmp.into());
        let brush = HBRUSH(GetStockObject(BLACK_BRUSH).0);
        let mask_rect = RECT {
            left: 0,
            top: 0,
            right: size.cx,
            bottom: size.cy,
        };
        FillRect(hdc_mem, &mask_rect, brush);
        let bf = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 0xff,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let position = POINT {
            x: rect.left,
            y: rect.top,
        };
        UpdateLayeredWindow(
            mask,
            Some(hdc_screen),
            Some(&position),
            Some(&size),
            Some(hdc_mem),
            Some(&POINT { x: 0, y: 0 }),
            COLORREF(0x00FFFFFF),
            Some(&bf),
            ULW_COLORKEY,
        )?;
        SetWindowPos(
            mask,
            Some(HWND_TOPMOST),
            position.x,
            position.y,
            size.cx,
            size.cy,
            SWP_SHOWWINDOW,
        )?;
        SelectObject(hdc_mem, bmp_old);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
        Ok(mask)
    }
}

fn from96dpi(value: f64, dpi: u32) -> i32 {
    (value * (dpi as f64 / 96.0)).round() as i32
}

extern "system" fn def_window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcA(window, message, wparam, lparam) }
}

extern "system" fn get_child_windows(window: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let v = lparam.0 as *mut Vec<HWND>;
        (*v).push(window);
        TRUE
    }
}

extern "system" fn toggle_masks(window: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let v = lparam.0 as *const (isize, SET_WINDOW_POS_FLAGS);
        let (checkbox, flag) = *v;
        if GetWindowLongPtrA(window, GWLP_USERDATA) == checkbox {
            let _ = SetWindowPos(
                window,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | flag,
            );
        }
        TRUE
    }
}
