use interception::{Interception, MouseState, KeyState, Stroke, Filter, MouseFlags as MouseFlag, MouseFilter, KeyFilter};
use anyhow::{Result, anyhow};

pub struct HardwareExecutor {
    context: Interception,
}

unsafe impl Send for HardwareExecutor {}
unsafe impl Sync for HardwareExecutor {}

impl HardwareExecutor {
    pub fn new() -> Result<Self> {
        let context = Interception::new().ok_or_else(|| anyhow!("Failed to initialize Interception context. Is the driver installed?"))?;
        
        context.set_filter(interception::is_mouse, Filter::MouseFilter(MouseFilter::empty()));
        context.set_filter(interception::is_keyboard, Filter::KeyFilter(KeyFilter::empty()));

        Ok(Self { context })
    }

    pub fn click(&self, _x: i32, _y: i32) -> Result<()> {
        let stroke = Stroke::Mouse {
            state: MouseState::LEFT_BUTTON_DOWN,
            flags: MouseFlag::MOVE_RELATIVE,
            rolling: 0,
            x: 0,
            y: 0,
            information: 0,
        };
        self.context.send(1, &[stroke]);

        std::thread::sleep(std::time::Duration::from_millis(50));

        let stroke = Stroke::Mouse {
            state: MouseState::LEFT_BUTTON_UP,
            flags: MouseFlag::MOVE_RELATIVE,
            rolling: 0,
            x: 0,
            y: 0,
            information: 0,
        };
        self.context.send(1, &[stroke]);
        
        Ok(())
    }

    pub fn move_to(&self, x: i32, y: i32) -> Result<()> {
        let stroke = Stroke::Mouse {
            state: MouseState::empty(),
            flags: MouseFlag::MOVE_ABSOLUTE,
            rolling: 0,
            x,
            y,
            information: 0,
        };
        
        self.context.send(1, &[stroke]);
        Ok(())
    }

    pub fn smooth_move(&self, target_x: i32, target_y: i32, steps: usize) -> Result<()> {
        for i in 1..=steps {
            let x = (target_x * i as i32) / steps as i32;
            let y = (target_y * i as i32) / steps as i32;
            self.move_to(x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }

    pub fn type_text(&self, text: &str) -> Result<()> {
        for c in text.chars() {
            if c.is_ascii_lowercase() {
                let scan_code = (c as u8 - b'a' + 0x1E) as u16;
                let stroke = Stroke::Keyboard {
                    code: scan_code.try_into().unwrap_or(interception::ScanCode::Esc),
                    state: KeyState::DOWN,
                    information: 0,
                };
                self.context.send(1, &[stroke]);
                
                let stroke = Stroke::Keyboard {
                    code: scan_code.try_into().unwrap_or(interception::ScanCode::Esc),
                    state: KeyState::UP,
                    information: 0,
                };
                self.context.send(1, &[stroke]);
            }
        }
        Ok(())
    }
}
