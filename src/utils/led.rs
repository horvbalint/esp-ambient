use std::{time::{Instant, Duration}, f32::consts::PI};

use esp_idf_hal::ledc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use palette::{rgb::Rgb, RgbHue};
use palette::Hsv;

// struct ColorChange {
//     hue: f32,
//     saturation: f32,
//     value: f32,
// }

enum TransitionMode {
    Cycle(f32),
    Pulse,
    ShiftHue(f32, f32),
    ShiftSaturation(f32, f32),
    ShiftValue(f32, f32),
}

struct BackupValues {
    saturation: f32,
    value: f32,
}

struct Transition {
    mode: TransitionMode,
    start_at: Instant,
    duration: Duration,
    backup: Option<BackupValues>
}

impl Transition {
    fn tick(&mut self, color: &mut Hsv) {
        match &self.mode {
            TransitionMode::Cycle(start) => self.cycle(color, start),
            TransitionMode::Pulse => self.pulse(color),
            TransitionMode::ShiftHue(start, change) => self.shift_hue(color, start, change),
            TransitionMode::ShiftSaturation(start, change) => self.shift_saturation(color, start, change),
            TransitionMode::ShiftValue(start, change) => self.shift_value(color, start, change),
        }
    }

    fn cycle(&self, color: &mut Hsv, start_hue: &f32) {
        let elapsed_secs = Instant::now().duration_since(self.start_at).as_secs_f32();
        let progress = elapsed_secs / self.duration.as_secs_f32();
        let hue = (start_hue + (progress * 360.)) % 360.;
    
        color.hue = RgbHue::from_degrees(hue);
    }

    fn pulse(&self, color: &mut Hsv) {
        let elapsed_secs = Instant::now().duration_since(self.start_at).as_secs_f32();
        let progress = (elapsed_secs / self.duration.as_secs_f32()).fract();
        
        color.value = (progress * PI * 2.).sin() / 2. + 0.5;
    }

    fn shift_hue(&self, color: &mut Hsv, start_hue: &f32, hue_change: &f32) {
        let elapsed_secs = Instant::now().duration_since(self.start_at).as_secs_f32();
        let progress = elapsed_secs / self.duration.as_secs_f32();
        let hue = start_hue + progress * hue_change;

        color.hue = RgbHue::from_degrees(hue)
    }

    fn shift_saturation(&self, color: &mut Hsv, start_saturation: &f32, saturation_change: &f32) {
        let elapsed_secs = Instant::now().duration_since(self.start_at).as_secs_f32();
        let progress = elapsed_secs / self.duration.as_secs_f32();

        color.saturation = start_saturation + progress * saturation_change;
    }

    fn shift_value(&self, color: &mut Hsv, start_value: &f32, value_change: &f32) {
        let elapsed_secs = Instant::now().duration_since(self.start_at).as_secs_f32();
        let progress = elapsed_secs / self.duration.as_secs_f32();

        color.value = start_value + progress * value_change;
    }
}

pub struct Led {
    channel_r: LedcDriver<'static>,
    channel_g: LedcDriver<'static>,
    channel_b: LedcDriver<'static>,
    transition: Option<Transition>,
    pub color: Hsv,
}

impl Led {
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
            transition: None,
            color: Hsv::new(0., 1., 1.),
        })
    }

    fn display_color(&mut self) -> anyhow::Result<()> {
        let rgb: Rgb = self.color.into();
        self.channel_r.set_duty((rgb.red * 100.) as u32)?;
        self.channel_g.set_duty((rgb.green * 100.) as u32)?;
        self.channel_b.set_duty((rgb.blue * 100.) as u32)?;

        Ok(())
    }

    pub fn set_hue(&mut self, hue: f32) -> anyhow::Result<()> {
        let curr_hue_degrees = self.color.hue.to_positive_degrees();
        let transition = Transition {
            mode: TransitionMode::ShiftHue(curr_hue_degrees, hue - curr_hue_degrees),
            duration: Duration::from_millis(100),
            start_at: Instant::now(),
            backup: None,
        };

        self.transition = Some(transition);

        Ok(())
    }

    pub fn set_saturation(&mut self, saturation: f32) -> anyhow::Result<()> {
        let transition = Transition {
            mode: TransitionMode::ShiftSaturation(self.color.saturation, saturation - self.color.saturation),
            duration: Duration::from_millis(100),
            start_at: Instant::now(),
            backup: None,
        };

        self.transition = Some(transition);

        Ok(())
    }

    pub fn set_value(&mut self, value: f32) -> anyhow::Result<()> {
        let transition = Transition {
            mode: TransitionMode::ShiftValue(self.color.value, value - self.color.value),
            duration: Duration::from_millis(100),
            start_at: Instant::now(),
            backup: None,
        };

        self.transition = Some(transition);

        Ok(())
    }

    pub fn set_rgb(&mut self, rgb: Rgb) -> anyhow::Result<()> {
        self.color = rgb.into();
        self.display_color()
    }

    pub fn set_hsv(&mut self, hsv: Hsv) -> anyhow::Result<()> {
        self.color = hsv;
        self.display_color()
    }

    pub fn cycle_colors(&mut self, duration: Duration) {
        self.stop_transition();

        let transition = Transition {
            mode: TransitionMode::Cycle(self.color.hue.to_positive_degrees()),
            duration,
            start_at: Instant::now(),
            backup: None,
        };

        self.transition = Some(transition);
    }

    pub fn pulse(&mut self, duration: Duration) {
        self.stop_transition();

        let backup = BackupValues {
            saturation: self.color.saturation,
            value: self.color.value,
        };

        let transition = Transition {
            mode: TransitionMode::Pulse,
            duration,
            start_at: Instant::now(),
            backup: Some(backup),
        };

        self.transition = Some(transition);
    }

    pub fn stop_transition(&mut self) {
        if let Some(transition) = self.transition.take() {
            if let Some(backup) = transition.backup {
                self.color.saturation = backup.saturation;
                self.color.value = backup.value;
            }
        }
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        if let Some(transition) = &mut self.transition {
            transition.tick(&mut self.color);
            self.set_hsv(self.color)?;
        }

        Ok(())
    }
}
