use crate::Source;

pub trait Statistic {
    /// the reported name of the series
    fn name(&self) -> &str;

    /// the unit of measurement
    fn unit(&self) -> Option<&str> {
        None
    }

    /// scaling that must be applied to the measurement before reporting
    fn multiplier(&self) -> f64 {
        1.0
    }

    /// describe the meaning of the statistic
    fn description(&self) -> Option<&str> {
        None
    }

    /// the source of the measurement
    fn source(&self) -> Source;
}
