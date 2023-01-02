use std::{time::{Instant, Duration}, f32::consts::PI};

use esp_idf_hal::ledc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use palette::{rgb::Rgb, RgbHue};
use palette::Hsv;

enum TransitionMode {
    Cycle(f32),
    Pulse,
    ShiftHue{
        start: f32,
        change: f32
    },
    ShiftSaturation{
        start: f32,
        change: f32
    },
    ShiftValue{
        start: f32,
        change: f32
    },
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
    fn tick(&mut self, color: &mut Hsv) -> bool {
        let elapsed_secs = Instant::now().duration_since(self.start_at).as_secs_f32();
        let progress = elapsed_secs / self.duration.as_secs_f32();

        match &self.mode {
            TransitionMode::Cycle(start) => Transition::cycle(color, progress, start),
            TransitionMode::Pulse => Transition::pulse(color, progress),
            TransitionMode::ShiftHue{start, change} => Transition::shift_hue(color, progress, start, change),
            TransitionMode::ShiftSaturation{start, change} => Transition::shift_saturation(color, progress, start, change),
            TransitionMode::ShiftValue{start, change} => Transition::shift_value(color, progress, start, change),
        }
    }

    fn cycle(color: &mut Hsv, progress: f32, start_hue: &f32) -> bool {
        let hue = (start_hue + (progress * 360.)) % 360.;
        color.hue = RgbHue::from_degrees(hue);
        
        return false;
    }

    fn pulse(color: &mut Hsv, progress: f32) -> bool {
        color.value = (progress.fract() * PI * 2.).sin() / 2. + 0.5;
        
        return false;
    }

    fn shift_hue(color: &mut Hsv, progress: f32, start_hue: &f32, hue_change: &f32) -> bool {
        let progress = progress.min(1.);
        color.hue = RgbHue::from_degrees(start_hue + progress * hue_change);

        return progress == 1.;
    }

    fn shift_saturation(color: &mut Hsv, progress: f32, start_saturation: &f32, saturation_change: &f32) -> bool {
        let progress = progress.min(1.);
        color.saturation = start_saturation + progress * saturation_change;

        return progress == 1.;
    }

    fn shift_value(color: &mut Hsv, progress: f32, start_value: &f32, value_change: &f32) -> bool {
        let progress = progress.min(1.);
        color.value = start_value + progress * value_change;

        return progress == 1.;
    }
}

pub struct Led {
    channel_r: LedcDriver<'static>,
    channel_g: LedcDriver<'static>,
    channel_b: LedcDriver<'static>,
    transitions: Vec<Transition>,
    pub color: Hsv,
}

impl Led {
    pub fn new(peripherals: Peripherals) -> anyhow::Result<Self> {
        let timer = LedcTimerDriver::new(
            peripherals.ledc.timer0,
            &config::TimerConfig::new().frequency(25.kHz().into()),
        )?;

        Ok(Self {
            channel_r: LedcDriver::new(peripherals.ledc.channel0, &timer, peripherals.pins.gpio0)?,
            channel_g: LedcDriver::new(peripherals.ledc.channel1, &timer, peripherals.pins.gpio1)?,
            channel_b: LedcDriver::new(peripherals.ledc.channel2, &timer, peripherals.pins.gpio2)?,
            transitions: vec![],
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

    pub fn set_rgb(&mut self, rgb: Rgb) -> anyhow::Result<()> {
        self.color = rgb.into();
        self.display_color()
    }

    pub fn set_hsv(&mut self, hsv: Hsv) -> anyhow::Result<()> {
        self.color = hsv;
        self.display_color()
    }

    pub fn set_hue(&mut self, hue: f32) {
        let curr_hue_degrees = self.color.hue.to_positive_degrees();
        let diff = hue - curr_hue_degrees;

        let change = diff;

        let transition = Transition {
            mode: TransitionMode::ShiftHue {
                start: curr_hue_degrees,
                change
            },
            duration: Duration::from_millis(200),
            start_at: Instant::now(),
            backup: None,
        };

        self.transitions.push(transition);
    }

    pub fn set_saturation(&mut self, saturation: f32) {
        let transition = Transition {
            mode: TransitionMode::ShiftSaturation {
                start: self.color.saturation,
                change: saturation - self.color.saturation
            },
            duration: Duration::from_millis(200),
            start_at: Instant::now(),
            backup: None,
        };

        self.transitions.push(transition);
    }

    pub fn set_value(&mut self, value: f32) {
        let transition = Transition {
            mode: TransitionMode::ShiftValue {
                start: self.color.value,
                change: value - self.color.value
            },
            duration: Duration::from_millis(200),
            start_at: Instant::now(),
            backup: None,
        };

        self.transitions.push(transition);
    }

    pub fn cycle_colors(&mut self, duration: Duration) {
        self.stop_transition();

        let transition = Transition {
            mode: TransitionMode::Cycle(self.color.hue.to_positive_degrees()),
            duration,
            start_at: Instant::now(),
            backup: None,
        };

        self.transitions.push(transition);
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

        self.transitions.push(transition);
    }

    pub fn stop_transition(&mut self) {
        for transition in self.transitions.drain(..) {
            if let Some(backup) = transition.backup {
                self.color.saturation = backup.saturation;
                self.color.value = backup.value;
            }
        }
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        self.transitions.retain_mut(|transition| !transition.tick(&mut self.color));
        self.set_hsv(self.color)?;

        Ok(())
    }
}
