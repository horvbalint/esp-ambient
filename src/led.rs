use esp_idf_hal::ledc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;

pub struct Led<'a> {
    channel_r: LedcDriver<'a>,
    channel_g: LedcDriver<'a>,
    channel_b: LedcDriver<'a>,
}

impl<'a> Led<'a> {
    pub fn new(peripherals: Peripherals) -> anyhow::Result<Self> {
        let channel_r = LedcDriver::new(
            peripherals.ledc.channel0,
            LedcTimerDriver::new(
                peripherals.ledc.timer0,
                &config::TimerConfig::new().frequency(25.kHz().into()),
            )?,
            peripherals.pins.gpio0,
        )?;

        let channel_g = LedcDriver::new(
            peripherals.ledc.channel1,
            LedcTimerDriver::new(
                peripherals.ledc.timer1,
                &config::TimerConfig::new().frequency(25.kHz().into()),
            )?,
            peripherals.pins.gpio1,
        )?;

        let channel_b = LedcDriver::new(
            peripherals.ledc.channel2,
            LedcTimerDriver::new(
                peripherals.ledc.timer2,
                &config::TimerConfig::new().frequency(25.kHz().into()),
            )?,
            peripherals.pins.gpio2,
        )?;

        Ok(Self {
            channel_r,
            channel_g,
            channel_b,
        })
    }

    pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) -> anyhow::Result<()> {
        self.channel_r.set_duty((r as f64 / 255. * 100.) as u32)?;
        self.channel_g.set_duty((g as f64 / 255. * 100.) as u32)?;
        self.channel_b.set_duty((b as f64 / 255. * 100.) as u32)?;

        Ok(())
    }
}
