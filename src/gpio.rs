pub use platform::OutputPin;

#[cfg(target_os = "linux")]
mod platform {
    use eyre::Result;
    use rppal::gpio;

    pub struct OutputPin(gpio::OutputPin);

    impl OutputPin {
        pub fn new(pin: u8) -> Result<Self> {
            Ok(OutputPin(gpio::Gpio::new()?.get(pin)?.into_output()))
        }

        pub fn set(&mut self, state: bool) {
            if state {
                self.0.set_high();
            } else {
                self.0.set_low();
            }
        }
        pub fn state(&self) -> bool {
            self.0.is_set_high()
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use eyre::Result;
    pub struct OutputPin(u8, bool);

    impl OutputPin {
        pub fn new(pin: u8) -> Result<Self> {
            Ok(OutputPin(pin, false))
        }

        pub fn set(&mut self, state: bool) {
            println!("PIN {} => {}", self.0, if state { "HIGH" } else { "LOW" });
            self.1 = state;
        }
        pub fn state(&self) -> bool {
            self.1
        }
    }
}
