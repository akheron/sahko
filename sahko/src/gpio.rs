use eyre::Result;
use gpio_cdev::{Chip, LineDirection, LineRequestFlags};

pub enum StateChange {
    None,
    Change {
        changed_pins: Vec<usize>,
        powered_on: bool,
    },
}

/// Returns indices of pins whose state was changed
pub fn set_pin_states(pins: &[(u8, bool)]) -> Result<StateChange> {
    let mut powered_on = false;
    let mut changed_pins = Vec::new();

    let mut chip = Chip::new("/dev/gpiochip0")?;
    for (i, (pin, state)) in pins.iter().enumerate() {
        let value = u8::from(*state);

        let line = chip.get_line(*pin as u32)?;
        if line.info()?.direction() != LineDirection::Out {
            powered_on = true;
            changed_pins.push(i);
        } else {
            let line_handle = line.request(LineRequestFlags::empty(), 0, "sahko")?;
            let current_value = line_handle.get_value()?;
            if current_value != value {
                changed_pins.push(i);
            }
        }
        line.request(LineRequestFlags::OUTPUT, value, "sahko")?;
    }

    Ok(if changed_pins.is_empty() {
        StateChange::None
    } else {
        StateChange::Change {
            changed_pins,
            powered_on,
        }
    })
}
