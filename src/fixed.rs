use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// Coordinates are signed 26.6 fixed point values in logical device-independent
/// pixels. Using integers makes line breaks reproducible in native and WASM builds.
pub const LAYOUT_UNITS_PER_DIP: i32 = 64;

/// Vertical placement is stored in 1/1024ths of the anchor line height.
pub const BLOCK_OFFSET_SCALE: i32 = 1024;

#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LayoutUnit(pub i32);

impl LayoutUnit {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(i32::MAX);

    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> i32 {
        self.0
    }

    pub fn from_dip(dip: f32) -> Self {
        let scaled = (dip as f64) * (LAYOUT_UNITS_PER_DIP as f64);
        Self(scaled.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32)
    }

    pub fn to_dip(self) -> f32 {
        self.0 as f32 / LAYOUT_UNITS_PER_DIP as f32
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0))
    }
}

impl fmt::Debug for LayoutUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}dip", self.to_dip())
    }
}

impl Add for LayoutUnit {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.saturating_add(rhs)
    }
}

impl AddAssign for LayoutUnit {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.saturating_add(rhs);
    }
}

impl Sub for LayoutUnit {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.saturating_sub(rhs)
    }
}

impl SubAssign for LayoutUnit {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.saturating_sub(rhs);
    }
}

/// A portable unsigned normalized value. 0 is the start edge and 65,535 is
/// the end edge. The denominator is part of the LastDraft v1 data contract.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Normalized(pub u16);

impl Normalized {
    pub const START: Self = Self(0);
    pub const CENTER: Self = Self(32_768);
    pub const END: Self = Self(u16::MAX);

    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Multiplies a layout value by this normalized value, rounding half up.
    pub fn of(self, value: LayoutUnit) -> LayoutUnit {
        if value.raw() <= 0 {
            return LayoutUnit::ZERO;
        }
        let numerator = value.raw() as i64 * self.0 as i64 + (u16::MAX as i64 / 2);
        LayoutUnit::from_raw((numerator / u16::MAX as i64) as i32)
    }
}

pub(crate) fn scale_ratio(value: LayoutUnit, numerator: i32, denominator: i32) -> LayoutUnit {
    if denominator <= 0 || numerator <= 0 || value.raw() <= 0 {
        return LayoutUnit::ZERO;
    }
    let product = value.raw() as i64 * numerator as i64;
    LayoutUnit::from_raw(((product + denominator as i64 / 2) / denominator as i64) as i32)
}

pub(crate) fn scale_block_offset(line_height: LayoutUnit, offset: i32) -> LayoutUnit {
    let product = line_height.raw() as i64 * offset as i64;
    let half = (BLOCK_OFFSET_SCALE / 2) as i64;
    let rounded = if product >= 0 {
        product + half
    } else {
        product - half
    };
    LayoutUnit::from_raw(
        (rounded / BLOCK_OFFSET_SCALE as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
    )
}
