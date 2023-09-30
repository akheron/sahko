use eyre::Result;
use gpio_cdev::{Chip, LineDirection, LineRequestFlags};

/// Returns indices of pins whose state was changed
pub fn set_pin_states(pins: &[(u8, bool)]) -> Result<Vec<usize>> {
    let mut changed = Vec::new();

    let mut chip = Chip::new("/dev/gpiochip0")?;
    for (i, (pin, state)) in pins.iter().enumerate() {
        let mut pin_changed = false;
        let value = u8::from(*state);

        let line = chip.get_line(*pin as u32)?;
        if line.info()?.direction() != LineDirection::Out {
            pin_changed = true;
        } else {
            let line_handle = line.request(LineRequestFlags::empty(), 0, "sahko")?;
            let current_value = line_handle.get_value()?;
            if current_value != value {
                pin_changed = true;
            }
        }
        line.request(LineRequestFlags::OUTPUT, value, "sahko")?;
        if pin_changed {
            changed.push(i);
        }
    }

    Ok(changed)
}
