use crate::error::AppError;
use std::fs;
use std::path::Path;
use windows::core::{Interface, PWSTR};
use windows::Win32::Foundation::{LPARAM, MAX_PATH, WPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationLegacyIAccessiblePattern,
    IUIAutomationTextPattern, IUIAutomationTextRange, IUIAutomationValuePattern,
    TextPatternRangeEndpoint_Start, TextUnit_Character, TreeScope_Descendants,
    UIA_ControlTypePropertyId, UIA_DocumentControlTypeId, UIA_EditControlTypeId,
    UIA_LegacyIAccessiblePatternId, UIA_TextPatternId, UIA_ValuePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorInfo, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, LoadCursorW,
    SendMessageW, CURSORINFO, CURSOR_SHOWING, GUITHREADINFO, IDC_IBEAM,
};

// bounds validation constants
const MIN_VALID_WIDTH: f64 = 1.0;
const MAX_VALID_HEIGHT: f64 = 100.0;
const MAX_VALID_COORDINATE: f64 = 10000.0;

// editable legacy control roles
const ROLE_SYSTEM_TEXT: u32 = 42;
const ROLE_SYSTEM_COMBOBOX: u32 = 46;

// Win32 Edit control messages for selection retrieval
const WM_GETTEXT: u32 = 0x000D;
const WM_GETTEXTLENGTH: u32 = 0x000E;
const EM_GETSEL: u32 = 0x00B0;

// import SafeArray functions from oleaut32.dll
#[link(name = "oleaut32")]
unsafe extern "system" {
    unsafe fn SafeArrayAccessData(
        psa: *mut std::ffi::c_void,
        ppv_data: *mut *mut std::ffi::c_void,
    ) -> i32;

    unsafe fn SafeArrayUnaccessData(psa: *mut std::ffi::c_void) -> i32;

    unsafe fn SafeArrayGetLBound(
        psa: *mut std::ffi::c_void,
        n_dim: u32,
        pl_lbound: *mut i32,
    ) -> i32;

    unsafe fn SafeArrayGetUBound(
        psa: *mut std::ffi::c_void,
        n_dim: u32,
        pl_ubound: *mut i32,
    ) -> i32;
}

/// COM resource guard.
struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    /// Initialize COM environment.
    fn new() -> Result<Self, AppError> {
        unsafe {
            let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if result.is_ok() {
                // successfully initialized, need to release on drop
                Ok(ComGuard { initialized: true })
            } else if result == windows::Win32::Foundation::RPC_E_CHANGED_MODE {
                // COM already initialized by other code, no need to release
                Ok(ComGuard { initialized: false })
            } else {
                // other errors
                Err("Failed to initialize COM".into())
            }
        }
    }
}

impl Drop for ComGuard {
    /// Release COM resources.
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

/// Get currently focused UI element.
fn get_focused_element() -> Result<IUIAutomationElement, AppError> {
    unsafe {
        // create UI Automation instance
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
            .map_err(|e| format!("Failed to create UI Automation instance: {}", e))?;

        // get focused element
        automation
            .GetFocusedElement()
            .map_err(|e| format!("Failed to get focused element: {}", e).into())
    }
}

/// Get first selected text range from given element.
fn get_selected_range(element: &IUIAutomationElement) -> Result<IUIAutomationTextRange, AppError> {
    unsafe {
        // get text pattern from element
        let text_pattern: IUIAutomationTextPattern = element
            .GetCurrentPattern(UIA_TextPatternId)
            .and_then(|p| p.cast())
            .map_err(|_| "Failed to get text pattern")?;

        // get currently selected text ranges
        let text_ranges = text_pattern
            .GetSelection()
            .map_err(|_| "Failed to get text selection")?;

        if text_ranges.Length().unwrap_or(0) == 0 {
            return Err("No text selection found".into());
        }

        // get first selection range
        text_ranges
            .GetElement(0)
            .map_err(|_| "Failed to get first selection range".into())
    }
}

/// Get selected text from focused Win32 control using SendMessage.
/// This works for standard Edit/RichEdit controls that don't support UIA TextPattern.
fn get_selection_via_win32_messages() -> Option<String> {
    unsafe {
        // get the foreground window's thread ID
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }
        let thread_id = GetWindowThreadProcessId(hwnd, None);
        if thread_id == 0 {
            return None;
        }

        // get the focused control handle via GetGUIThreadInfo
        let mut gui_info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(thread_id, &mut gui_info).is_err() {
            return None;
        }

        let focused_hwnd = gui_info.hwndFocus;
        if focused_hwnd.is_invalid() {
            return None;
        }

        // get selection range via EM_GETSEL
        let mut start: u32 = 0;
        let mut end: u32 = 0;
        SendMessageW(
            focused_hwnd,
            EM_GETSEL,
            Some(WPARAM(&mut start as *mut u32 as usize)),
            Some(LPARAM(&mut end as *mut u32 as isize)),
        );

        // no selection if start == end
        if start == end {
            return None;
        }

        // ensure start <= end
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }

        // get total text length
        let text_len =
            SendMessageW(focused_hwnd, WM_GETTEXTLENGTH, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;
        if text_len == 0 || end as usize > text_len {
            return None;
        }

        // allocate buffer and get full text (UTF-16)
        let mut buffer = vec![0u16; text_len + 1];
        let copied = SendMessageW(
            focused_hwnd,
            WM_GETTEXT,
            Some(WPARAM(buffer.len())),
            Some(LPARAM(buffer.as_mut_ptr() as isize)),
        )
        .0 as usize;

        if copied == 0 {
            return None;
        }

        // extract the selected portion
        let start = start as usize;
        let end = (end as usize).min(copied);
        if start >= end || start >= copied {
            return None;
        }

        // convert UTF-16 slice to String
        String::from_utf16(&buffer[start..end]).ok().filter(|s| !s.is_empty())
    }
}

/// Get selected text in currently focused element.
pub fn get_selection() -> Result<String, AppError> {
    unsafe {
        // initialize COM
        let _com = ComGuard::new()?;

        // get focused element
        let focused_element = get_focused_element()?;

        // Strategy 1: try to get selected text from focused element's TextPattern
        if let Ok(text_range) = get_selected_range(&focused_element) {
            if let Ok(text) = text_range.GetText(-1) {
                let text = text.to_string();
                if !text.is_empty() {
                    return Ok(text);
                }
            }
        }

        // Strategy 2: search for Document/Edit controls with TextPattern in the foreground window
        // This helps when the focused element itself doesn't support TextPattern
        // but a parent/sibling Document or Edit element does (common in browsers and editors)
        let hwnd = GetForegroundWindow();
        if !hwnd.is_invalid() {
            let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
                .map_err(|e| format!("Failed to create UI Automation instance: {}", e))?;

            if let Ok(root_element) = automation.ElementFromHandle(hwnd) {
                // search for Document/Edit controls with text selection
                if let Some(text) =
                    find_selection_in_descendants(&root_element, &automation)
                {
                    if !text.is_empty() {
                        return Ok(text);
                    }
                }
            }
        }

        // Strategy 3: try Win32 SendMessage approach for standard Edit/RichEdit controls
        // This covers controls like Notepad's Edit control and other Win32 native controls
        // that don't expose UIA TextPattern but respond to EM_GETSEL + WM_GETTEXT
        if let Some(text) = get_selection_via_win32_messages() {
            return Ok(text);
        }

        // no selection found via UIA or Win32 messages
        Ok(String::new())
    }
}

/// Search for selected text in descendant elements with TextPattern.
/// Tries Document controls first, then Edit controls.
unsafe fn find_selection_in_descendants(
    root_element: &IUIAutomationElement,
    automation: &IUIAutomation,
) -> Option<String> {
    // try Document controls first (most likely to have text selection in browsers)
    if let Some(text) = find_selection_by_control_type(
        root_element,
        automation,
        UIA_DocumentControlTypeId.0,
    ) {
        return Some(text);
    }

    // try Edit controls (covers standard text editors and some custom controls)
    if let Some(text) = find_selection_by_control_type(
        root_element,
        automation,
        UIA_EditControlTypeId.0,
    ) {
        return Some(text);
    }

    None
}

/// Find selected text in elements of a specific control type.
/// Searches all matching elements (not just the first) to handle windows with multiple
/// Document/Edit controls (e.g., browsers with multiple iframes).
unsafe fn find_selection_by_control_type(
    root_element: &IUIAutomationElement,
    automation: &IUIAutomation,
    control_type_id: i32,
) -> Option<String> {
    // create property condition for specified control type
    let condition = automation
        .CreatePropertyCondition(UIA_ControlTypePropertyId, &control_type_id.into())
        .ok()?;

    // find all matching elements
    let elements = root_element
        .FindAll(TreeScope_Descendants, &condition)
        .ok()?;

    let count = elements.Length().unwrap_or(0);
    for i in 0..count {
        let element = match elements.GetElement(i) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // try to get TextPattern and selection
        let text_pattern: IUIAutomationTextPattern = match element
            .GetCurrentPattern(UIA_TextPatternId)
            .and_then(|p| p.cast())
        {
            Ok(tp) => tp,
            Err(_) => continue,
        };

        let text_ranges = match text_pattern.GetSelection() {
            Ok(tr) => tr,
            Err(_) => continue,
        };

        if text_ranges.Length().unwrap_or(0) == 0 {
            continue;
        }

        let range = match text_ranges.GetElement(0) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if let Ok(text) = range.GetText(-1) {
            let text = text.to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }

    None
}

/// Get the coordinates of the bottom-right corner of the selected text.
pub fn get_cursor_location() -> Result<(i32, i32), AppError> {
    unsafe {
        // initialize COM
        let _com = ComGuard::new()?;

        // get focused element
        let focused_element = get_focused_element()?;

        // get first selected text range
        let text_range = get_selected_range(&focused_element)?;

        // get bounding rectangles for the text range
        let rect_array = text_range
            .GetBoundingRectangles()
            .map_err(|_| "Failed to get bounding rectangles")?;

        // access the SafeArray data
        let mut rect_ptr: *mut f64 = std::ptr::null_mut();
        let hr = SafeArrayAccessData(rect_array as *mut _, &mut rect_ptr as *mut _ as *mut _);
        if hr != 0 {
            return Err("Failed to access SafeArray data".into());
        }

        // get array bounds
        let mut lower_bound: i32 = 0;
        let mut upper_bound: i32 = 0;
        let hr = SafeArrayGetLBound(rect_array as *mut _, 1, &mut lower_bound);
        if hr != 0 {
            SafeArrayUnaccessData(rect_array as *mut _);
            return Err("Failed to get lower bound".into());
        }
        let hr = SafeArrayGetUBound(rect_array as *mut _, 1, &mut upper_bound);
        if hr != 0 {
            SafeArrayUnaccessData(rect_array as *mut _);
            return Err("Failed to get upper bound".into());
        }

        let rect_count = ((upper_bound - lower_bound + 1) / 4) as usize;
        if rect_count == 0 {
            SafeArrayUnaccessData(rect_array as *mut _);
            return Err("No bounding rectangles found".into());
        }

        // find last valid rectangle and calculate coordinates
        let mut result = Err("No valid rectangle found".into());
        for i in (0..rect_count).rev() {
            let rect_index = i * 4;
            let left = *rect_ptr.add(rect_index);
            let top = *rect_ptr.add(rect_index + 1);
            let width = *rect_ptr.add(rect_index + 2);
            let height = *rect_ptr.add(rect_index + 3);

            // validate rectangle bounds
            if width > MIN_VALID_WIDTH
                && height > 0.0
                && height < MAX_VALID_HEIGHT
                && left >= 0.0
                && top >= 0.0
                && left < MAX_VALID_COORDINATE
                && top < MAX_VALID_COORDINATE
            {
                // calculate bottom-right corner coordinates
                let bottom_right_x = (left + width) as i32;
                let bottom_right_y = (top + height) as i32;
                result = Ok((bottom_right_x, bottom_right_y));
                break;
            }
        }

        // unaccess the SafeArray data
        SafeArrayUnaccessData(rect_array as *mut _);

        result
    }
}

/// Check if currently focused element is editable.
pub fn is_cursor_editable() -> Result<bool, AppError> {
    unsafe {
        // initialize COM
        let _com = ComGuard::new()?;

        // get focused element
        let focused_element = get_focused_element()?;

        // Strategy 1: check if control type is editable
        if let Ok(control_type) = focused_element.CurrentControlType() {
            let is_edit_control = control_type.0 == UIA_EditControlTypeId.0;
            let is_document_control = control_type.0 == UIA_DocumentControlTypeId.0;
            if is_edit_control || is_document_control {
                return Ok(true);
            }
        }

        // Strategy 2: check if value pattern is not read-only
        if let Ok(is_readonly) = focused_element
            .GetCurrentPattern(UIA_ValuePatternId)
            .and_then(|p| p.cast::<IUIAutomationValuePattern>())
            .and_then(|vp| vp.CurrentIsReadOnly())
        {
            if !is_readonly.as_bool() {
                return Ok(true);
            }
        }

        // Strategy 3: check if legacy control role is editable
        if let Ok(role) = focused_element
            .GetCurrentPattern(UIA_LegacyIAccessiblePatternId)
            .and_then(|p| p.cast::<IUIAutomationLegacyIAccessiblePattern>())
            .and_then(|lp| lp.CurrentRole())
        {
            if role == ROLE_SYSTEM_TEXT || role == ROLE_SYSTEM_COMBOBOX {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Check if current cursor is I-Beam (text cursor).
pub fn is_ibeam_cursor() -> bool {
    unsafe {
        let mut cursor_info = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            flags: Default::default(),
            hCursor: Default::default(),
            ptScreenPos: Default::default(),
        };

        // get current cursor information
        if GetCursorInfo(&mut cursor_info).is_err() {
            return false;
        }

        // check if cursor is showing
        if cursor_info.flags != CURSOR_SHOWING {
            return false;
        }

        // load the system I-Beam cursor to get its handle
        let Ok(ibeam_cursor) = LoadCursorW(None, IDC_IBEAM) else {
            return false;
        };

        // compare cursor handles
        cursor_info.hCursor == ibeam_cursor
    }
}

/// Select specified number of characters from current cursor position backward.
pub fn select_backward_chars(chars: usize) -> Result<(), AppError> {
    unsafe {
        // initialize COM
        let _com = ComGuard::new()?;

        // get focused element
        let focused_element = get_focused_element()?;

        // get first selected text range
        let text_range = get_selected_range(&focused_element)?;

        // move endpoint backward
        text_range
            .MoveEndpointByUnit(
                TextPatternRangeEndpoint_Start,
                TextUnit_Character,
                -(chars as i32),
            )
            .map_err(|_| "Failed to move endpoint backward")?;

        // select new range
        text_range
            .Select()
            .map_err(|_| "Failed to select new range")?;

        Ok(())
    }
}

/// Get application identifier from an application path.
pub fn get_app_id(app_path: &Path) -> Result<String, AppError> {
    // canonicalize the application path
    if let Ok(canonical_path) = fs::canonicalize(app_path) {
        if let Some(normalized_path) = canonical_path.to_str() {
            // remove the "\\?\" prefix that Windows adds for long paths
            return Ok(normalized_path.trim_start_matches(r"\\?\").to_string());
        }
    }

    Err("Failed to get application identifier".into())
}

/// Get the executable path of the frontmost application.
pub fn get_frontmost_app_id() -> Option<String> {
    unsafe {
        // get foreground window
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }

        // get process ID
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        // open process with query information permission
        let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

        // get process executable path
        let mut buffer = vec![0u16; MAX_PATH as usize];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process_handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        if result.is_err() || size == 0 {
            return None;
        }

        // convert to string
        String::from_utf16(&buffer[..size as usize]).ok()
    }
}

/// Get the current website URL from the frontmost browser.
pub fn get_frontmost_url() -> Option<String> {
    unsafe {
        // initialize COM
        let _com = ComGuard::new().ok()?;

        // get foreground window
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }

        // create UI Automation instance
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL).ok()?;

        // get root UI element from window handle
        let element = automation.ElementFromHandle(hwnd).ok()?;

        // Strategy 1: try to find first edit control
        if let Some(url) = find_url_in_element(&element, &automation, UIA_EditControlTypeId.0) {
            return Some(url);
        }

        // Strategy 2: try to find first document control
        if let Some(url) = find_url_in_element(&element, &automation, UIA_DocumentControlTypeId.0) {
            return Some(url);
        }

        None
    }
}

/// Try to extract URL from element with specified control type.
unsafe fn find_url_in_element(
    root_element: &IUIAutomationElement,
    automation: &IUIAutomation,
    control_type_id: i32,
) -> Option<String> {
    // create property condition for specified control type
    let condition = automation
        .CreatePropertyCondition(UIA_ControlTypePropertyId, &control_type_id.into())
        .ok()?;

    // find element with specified control type
    let element = root_element
        .FindFirst(TreeScope_Descendants, &condition)
        .ok()?;

    // get value pattern from element
    let value_pattern = element
        .GetCurrentPattern(UIA_ValuePatternId)
        .and_then(|p| p.cast::<IUIAutomationValuePattern>())
        .ok()?;

    // extract value
    let value = value_pattern.CurrentValue().ok()?;

    // validate URL format
    let url = value.to_string();
    if !url.is_empty() && (url.starts_with("http://") || url.starts_with("https://")) {
        Some(url)
    } else {
        None
    }
}
