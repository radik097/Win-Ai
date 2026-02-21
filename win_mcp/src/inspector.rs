use windows::Win32::UI::Accessibility::*;
use windows::Win32::System::Com::*;
use windows::core::BSTR;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct UiElement {
    pub name: String,
    pub control_type: String,
    pub automation_id: String,
    pub rect: [i32; 4], // [left, top, right, bottom]
    pub children: Vec<UiElement>,
}

pub struct UiInspector {
    automation: IUIAutomation,
}

unsafe impl Send for UiInspector {}
unsafe impl Sync for UiInspector {}

impl UiInspector {
    pub fn new() -> Result<Self> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
                .context("Failed to create UI Automation instance")?;
            Ok(Self { automation })
        }
    }

    pub fn get_ui_tree(&self, max_depth: usize) -> Result<UiElement> {
        unsafe {
            let root = self.automation.GetRootElement().context("Failed to get root element")?;
            self.traverse_element(&root, 0, max_depth)
        }
    }

    fn traverse_element(&self, element: &IUIAutomationElement, depth: usize, max_depth: usize) -> Result<UiElement> {
        unsafe {
            let name = element.CurrentName().unwrap_or(BSTR::from("")).to_string();
            let control_type = element.CurrentLocalizedControlType().unwrap_or(BSTR::from("")).to_string();
            let automation_id = element.CurrentAutomationId().unwrap_or(BSTR::from("")).to_string();
            let rect = element.CurrentBoundingRectangle().unwrap_or_default();

            let mut ui_el = UiElement {
                name,
                control_type,
                automation_id,
                rect: [rect.left, rect.top, rect.right, rect.bottom],
                children: Vec::new(),
            };

            if depth < max_depth {
                let condition = self.automation.CreateTrueCondition().context("Failed to create condition")?;
                let children = element.FindAll(TreeScope_Children, &condition).context("Failed to find children")?;
                let count = children.Length().unwrap_or(0);

                for i in 0..count {
                    if let Ok(child) = children.GetElement(i) {
                        if let Ok(child_tree) = self.traverse_element(&child, depth + 1, max_depth) {
                            ui_el.children.push(child_tree);
                        }
                    }
                }
            }

            Ok(ui_el)
        }
    }
}
