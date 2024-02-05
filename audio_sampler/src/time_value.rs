use nih_plug::prelude::{Enum, Transport};

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum TimeUnit {
    #[name = "1/16 notes"]
    SixteenthNotes,
    #[name = "1/4 notes"]
    QuarterNotes,
    #[name = "Seconds"]
    Seconds,
    #[name = "Samples"]
    Samples,
    #[name = "Bars"]
    Bars,
}

impl TryFrom<TimeOrRatioUnit> for TimeUnit {
    type Error = ();
    fn try_from(unit: TimeOrRatioUnit) -> Result<Self, Self::Error> {
        match unit {
            TimeOrRatioUnit::SixteenthNotes => Ok(TimeUnit::SixteenthNotes),
            TimeOrRatioUnit::QuarterNotes => Ok(TimeUnit::QuarterNotes),
            TimeOrRatioUnit::Seconds => Ok(TimeUnit::Seconds),
            TimeOrRatioUnit::Samples => Ok(TimeUnit::Samples),
            TimeOrRatioUnit::Bars => Ok(TimeUnit::Bars),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TimeOrRatio {
    Time(TimeValue),
    Ratio(f32),
}

impl TimeOrRatio {
    pub fn from_unit_value(unit: TimeOrRatioUnit, value: f32) -> TimeOrRatio {
        match unit {
            TimeOrRatioUnit::Ratio => TimeOrRatio::Ratio(value),
            _ => TimeOrRatio::Time(TimeValue::from_unit_value(
                unit.try_into().unwrap(),
                value as f64,
            )),
        }
    }
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum TimeOrRatioUnit {
    #[name = "1/16 notes"]
    SixteenthNotes,
    #[name = "1/4 notes"]
    QuarterNotes,
    #[name = "Seconds"]
    Seconds,
    #[name = "Samples"]
    Samples,
    #[name = "Bars"]
    Bars,
    #[name = "Ratio"]
    Ratio,
}

#[derive(Debug, Clone, Copy)]
pub enum TimeValue {
    QuarterNotes(f64),
    Samples(f64),
    Seconds(f64),
    Bars(f64),
}

impl TimeValue {
    pub fn quarter_notes(quarter_notes: f64) -> Self {
        TimeValue::QuarterNotes(quarter_notes)
    }
    pub fn samples(samples: f64) -> Self {
        TimeValue::Samples(samples)
    }
    pub fn bars(bars: f64) -> Self {
        TimeValue::Bars(bars)
    }
    pub fn from_unit_value(unit: TimeUnit, value: f64) -> Self {
        match unit {
            TimeUnit::SixteenthNotes => TimeValue::QuarterNotes(value / 4.0),
            TimeUnit::QuarterNotes => TimeValue::QuarterNotes(value),
            TimeUnit::Seconds => TimeValue::Seconds(value),
            TimeUnit::Samples => TimeValue::Samples(value),
            TimeUnit::Bars => TimeValue::Bars(value),
        }
    }
    pub fn as_samples_f64(&self, transport: &Transport) -> Option<f64> {
        Some(match self {
            TimeValue::QuarterNotes(quarter_notes) => {
                calc_quarter_notes_per_bar(transport)? * quarter_notes
            }
            TimeValue::Samples(samples) => *samples,
            TimeValue::Seconds(seconds) => *seconds * (transport.sample_rate as f64),
            TimeValue::Bars(bars) => calc_samples_per_bar(transport)? * bars,
        })
    }
}

pub fn calc_quarter_notes_per_bar(transport: &Transport) -> Option<f64> {
    let time_sig_numerator = transport.time_sig_numerator? as f64;
    let time_sig_denominator = transport.time_sig_denominator? as f64;
    let quarter_notes_per_bar = time_sig_numerator / time_sig_denominator * 4.0;
    Some(quarter_notes_per_bar)
}

pub fn calc_samples_per_quarter_note(transport: &Transport) -> Option<f64> {
    let sr = transport.sample_rate as f64;
    let samples_per_minute = sr * 60.0;
    let samples_per_quarter_note = samples_per_minute / transport.tempo?;
    Some(samples_per_quarter_note)
}

pub fn calc_samples_per_bar(transport: &Transport) -> Option<f64> {
    let samples_per_quarter_note = calc_samples_per_quarter_note(transport)?;
    let quarter_notes_per_bar = calc_quarter_notes_per_bar(transport)?;
    let samples_per_bar = samples_per_quarter_note * quarter_notes_per_bar;
    Some(samples_per_bar)
}
