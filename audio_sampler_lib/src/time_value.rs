use crate::common_types;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TimeUnit {
    SixteenthNotes,
    QuarterNotes,
    Seconds,
    Samples,
    Bars,
}

impl TryFrom<TimeOrRatioUnit> for TimeUnit {
    type Error = ();
    fn try_from(unit: TimeOrRatioUnit) -> Result<Self, Self::Error> {
        match unit {
            TimeOrRatioUnit::SixteenthNotes => Ok(TimeUnit::SixteenthNotes),
            TimeOrRatioUnit::Seconds => Ok(TimeUnit::Seconds),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TimeOrRatio {
    Time(TimeValue),
    Ratio(f32),
}

impl TimeOrRatio {}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TimeOrRatioUnit {
    SixteenthNotes,
    Seconds,
    Ratio,
}

#[derive(Debug, Clone, Copy)]
pub enum TimeValue {
    QuarterNotes(f32),
    Samples(f32),
    Seconds(f32),
    Bars(f32),
}

impl TimeValue {
    pub fn quarter_notes(quarter_notes: f32) -> Self {
        TimeValue::QuarterNotes(quarter_notes)
    }
    pub fn samples(samples: f32) -> Self {
        TimeValue::Samples(samples)
    }
    pub fn bars(bars: f32) -> Self {
        TimeValue::Bars(bars)
    }
    pub fn from_unit_value(unit: TimeUnit, value: f32) -> Self {
        match unit {
            TimeUnit::SixteenthNotes => TimeValue::QuarterNotes(value / 4.0),
            TimeUnit::QuarterNotes => TimeValue::QuarterNotes(value),
            TimeUnit::Seconds => TimeValue::Seconds(value),
            TimeUnit::Samples => TimeValue::Samples(value),
            TimeUnit::Bars => TimeValue::Bars(value),
        }
    }
    pub fn as_samples(&self, transport: &common_types::Transport) -> f32 {
        match self {
            TimeValue::QuarterNotes(quarter_notes) => {
                calc_samples_per_quarter_note(transport) * quarter_notes
            }
            TimeValue::Samples(samples) => *samples,
            TimeValue::Seconds(seconds) => *seconds * transport.sample_rate,
            TimeValue::Bars(bars) => calc_samples_per_bar(transport) * bars,
        }
    }
}

pub fn calc_quarter_notes_per_bar(transport: &common_types::Transport) -> f32 {
    let time_sig_numerator = transport.time_sig_numerator as f32;
    let time_sig_denominator = transport.time_sig_denominator as f32;
    let quarter_notes_per_bar = time_sig_numerator / time_sig_denominator * 4.0;
    quarter_notes_per_bar
}

pub fn calc_samples_per_quarter_note(transport: &common_types::Transport) -> f32 {
    let samples_per_minute = transport.sample_rate * 60.0;
    let samples_per_quarter_note = samples_per_minute / transport.tempo;
    samples_per_quarter_note
}

pub fn calc_samples_per_bar(transport: &common_types::Transport) -> f32 {
    let samples_per_quarter_note = calc_samples_per_quarter_note(transport);
    let quarter_notes_per_bar = calc_quarter_notes_per_bar(transport);
    let samples_per_bar = samples_per_quarter_note * quarter_notes_per_bar;
    samples_per_bar
}
