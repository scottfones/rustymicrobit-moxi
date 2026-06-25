//! Sensor measurement types.

/// SCD41 reading.
#[derive(Clone, Copy, Debug)]
pub struct Co2Measurement {
    pub co2: f32,
    pub humidity: f32,
    pub temp_c: f32,
}

impl Co2Measurement {
    /// Build a measurement from read SCD41 values.
    #[must_use]
    pub fn new(co2: u16, humidity: f32, temp_c: f32) -> Self {
        Self {
            co2: f32::from(co2),
            humidity,
            temp_c,
        }
    }
}

/// BMP581 reading.
#[derive(Clone, Copy, Debug)]
pub struct PressureMeasurement {
    pub hpa: f32,
    pub temp_c: f32,
}

impl PressureMeasurement {
    /// Build a measurement from read BMP581 values (pressure in pascals).
    #[must_use]
    pub fn new(pa: f32, temp_c: f32) -> Self {
        Self {
            hpa: pa / 100.0,
            temp_c,
        }
    }
}

/// Convert from degrees Celsius to degrees Fahrenheit.
#[must_use]
pub fn fahrenheit(celsius: f32) -> f32 {
    celsius * 9.0 / 5.0 + 32.0
}
