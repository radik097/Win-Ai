use interception::{Interception, MouseState, KeyState, Stroke, MouseStroke, KeyStroke, Filter, MouseFlag};
use anyhow::{Result, anyhow};

pub struct HardwareExecutor {
    context: Interception,
}

impl HardwareExecutor {
    pub fn new() -> Result<Self> {
        let context = Interception::new().ok_or_else(|| anyhow!("Failed to initialize Interception context. Is the driver installed?"))?;
        
        context.set_filter(interception::is_mouse, Filter::None);
        context.set_filter(interception::is_keyboard, Filter::None);

        Ok(Self { context })
    }

    pub fn click(&self, _x: i32, _y: i32) -> Result<()> {
        let mut stroke = MouseStroke::default();
        stroke.state = MouseState::LEFT_BUTTON_DOWN;
        self.context.send(1, &Stroke::Mouse(stroke));

        std::thread::sleep(std::time::Duration::from_millis(50));

        stroke.state = MouseState::LEFT_BUTTON_UP;
        self.context.send(1, &Stroke::Mouse(stroke));
        
        Ok(())
    }

    pub fn move_to(&self, x: i32, y: i32) -> Result<()> {
        let mut stroke = MouseStroke::default();
        stroke.x = x;
        stroke.y = y;
        stroke.flags = MouseFlag::MOVE_ABSOLUTE;
        
        self.context.send(1, &Stroke::Mouse(stroke));
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
                let mut stroke = KeyStroke::default();
                stroke.code = scan_code;
                stroke.state = KeyState::DOWN;
                self.context.send(1, &Stroke::Keyboard(stroke));
                
                stroke.state = KeyState::UP;
                self.context.send(1, &Stroke::Keyboard(stroke));
            }
        }
        Ok(())
    }
}
