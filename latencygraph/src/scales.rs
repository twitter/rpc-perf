
use plotters::prelude::*;
use plotters::coord::AsRangedCoord;

pub struct InvLogCoord {
    range: LogCoord<f64>
}

impl InvLogCoord {
    pub fn new(off: f64) -> Self {
        Self {
            range: LogRange(off .. 0.5).into()
        }
    }
}

impl Ranged for InvLogCoord {
    type ValueType = f64;

    fn map(&self, value: &f64, limit: (i32, i32)) -> i32 {        
        limit.1 - self.range.map(&(1.0 - *value), limit) + limit.0
    }
    
    fn key_points(&self, max_points: usize) -> Vec<f64> {
        let mut points = self.range.key_points(max_points - 1);
        points.push(0.5);

        for point in points.iter_mut() {
            *point = 1.0 - *point;
        }

        points
    }

    fn range(&self) -> std::ops::Range<f64> {
        0.5 .. 1.0
    }
}

impl AsRangedCoord for InvLogCoord {
    type CoordDescType = Self;
    type Value = f64;
}

pub struct DoubleLogCoord {
    lower: LogCoord<f64>,
    upper: LogCoord<f64>,
}

impl DoubleLogCoord {
    pub fn new(bot: f64, top: f64) -> Self {
        Self {
            lower: LogRange(bot..0.5).into(),
            upper: LogRange((1.0 - top)..0.5).into(),
        }
    }
}

impl Ranged for DoubleLogCoord {
    type ValueType = f64;

    fn map(&self, value: &f64, limit: (i32, i32)) -> i32 {
        if *value > 0.5 {
            let offset = self
                .upper
                .map(&(1.0 - value), (limit.0 + (limit.1 - limit.0) / 2, limit.1));

            limit.1 - (offset - limit.0) + (limit.1 - limit.0) / 2
        } else {
            self.lower
                .map(value, (limit.0, limit.1 - (limit.1 - limit.0) / 2))
        }
    }

    fn key_points(&self, max_points: usize) -> Vec<f64> {
        let mut lower = self.lower.key_points(max_points / 2);
        lower.extend(
            self.upper
                .key_points(max_points / 2)
                .into_iter()
                .rev()
                .map(|x| (1.0 - x))
                .chain(std::iter::once(0.5)),
        );

        lower
    }

    fn range(&self) -> std::ops::Range<f64> {
        0.0..1.0
    }
}

impl AsRangedCoord for DoubleLogCoord {
    type CoordDescType = Self;
    type Value = f64;
}

pub struct PlusOneRange<R>(pub R)
where
    R: AsRangedCoord<Value = f64>;

impl<R> AsRangedCoord for PlusOneRange<R>
where
    R: AsRangedCoord<Value = f64>,
    R::CoordDescType: Ranged<ValueType = f64>
{
    type CoordDescType = PlusOneCoord<R::CoordDescType>;
    type Value = f64;
}

impl<R> From<PlusOneRange<R>> for PlusOneCoord<R::CoordDescType>
where
    R: AsRangedCoord<Value = f64>,
    R::CoordDescType: Ranged<ValueType = f64>
{
    fn from(x: PlusOneRange<R>) -> PlusOneCoord<R::CoordDescType> {
        PlusOneCoord(x.0.into())
    }
}

pub struct PlusOneCoord<C>(pub C)
where
    C: Ranged;

impl<C: Ranged<ValueType = f64>> Ranged for PlusOneCoord<C> {
    type ValueType = f64;

    fn map(&self, value: &f64, limit: (i32, i32)) -> i32 {
        self.0.map(&(*value + 1.0), limit)
    }

    fn key_points(&self, max_points: usize) -> Vec<f64> {
        self.0.key_points(max_points)
    }

    fn range(&self) -> std::ops::Range<f64> {
        let r = self.0.range();

        r.start - 1.0 .. r.end - 1.0
    }
} 
