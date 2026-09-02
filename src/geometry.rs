use crate::{LayoutUnit, TextDirection, WritingMode};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub x: LayoutUnit,
    pub y: LayoutUnit,
    pub width: LayoutUnit,
    pub height: LayoutUnit,
}

impl Rect {
    pub const fn new(x: LayoutUnit, y: LayoutUnit, width: LayoutUnit, height: LayoutUnit) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(self) -> LayoutUnit {
        self.x + self.width
    }

    pub fn bottom(self) -> LayoutUnit {
        self.y + self.height
    }

    pub fn intersects_vertical_band(self, top: LayoutUnit, bottom: LayoutUnit) -> bool {
        self.height > LayoutUnit::ZERO && self.y < bottom && self.bottom() > top
    }

    pub fn expanded(self, insets: Insets) -> Self {
        Self {
            x: self.x - insets.start,
            y: self.y - insets.top,
            width: self.width + insets.start + insets.end,
            height: self.height + insets.top + insets.bottom,
        }
    }

    pub fn horizontal_intersection(self, other: Self) -> Option<Interval> {
        let start = self.x.max(other.x);
        let end = self.right().min(other.right());
        (end > start).then_some(Interval { start, end })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Insets {
    pub start: LayoutUnit,
    pub top: LayoutUnit,
    pub end: LayoutUnit,
    pub bottom: LayoutUnit,
}

/// Exclusion margins as the DOCUMENT stores them: along the inline axis (the
/// one the words run along) and the block axis (the one lines advance along).
///
/// These live logically because the physical side each one lands on depends on
/// the paragraph's resolved direction and writing mode, and the resolved
/// direction is not known until the engine has run the bidi algorithm. That is
/// exactly why this resolution cannot sit in a platform adapter: with
/// `baseDirection: auto` — the default for a letter — the adapter does not yet
/// know whether the paragraph is ltr or rtl.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LogicalInsets {
    pub inline_start: LayoutUnit,
    pub block_start: LayoutUnit,
    pub inline_end: LayoutUnit,
    pub block_end: LayoutUnit,
}

impl LogicalInsets {
    pub const fn uniform(value: LayoutUnit) -> Self {
        Self {
            inline_start: value,
            block_start: value,
            inline_end: value,
            block_end: value,
        }
    }

    /// Map the four logical sides onto the four physical ones.
    ///
    /// In horizontal-ltr the two coincide, which is why a mapping that ignored
    /// direction looked correct for so long.
    pub fn resolve(self, writing_mode: WritingMode, base_direction: TextDirection) -> Insets {
        let rtl = base_direction == TextDirection::RightToLeft;
        match writing_mode {
            // Inline runs across, block runs down.
            WritingMode::HorizontalTb => Insets {
                start: if rtl { self.inline_end } else { self.inline_start },
                end: if rtl { self.inline_start } else { self.inline_end },
                top: self.block_start,
                bottom: self.block_end,
            },
            // Inline runs down the column, block runs LEFT from the right edge.
            WritingMode::VerticalRl => Insets {
                top: if rtl { self.inline_end } else { self.inline_start },
                bottom: if rtl { self.inline_start } else { self.inline_end },
                end: self.block_start,
                start: self.block_end,
            },
            // Inline runs down the column, block runs right from the left edge.
            WritingMode::VerticalLr => Insets {
                top: if rtl { self.inline_end } else { self.inline_start },
                bottom: if rtl { self.inline_start } else { self.inline_end },
                start: self.block_start,
                end: self.block_end,
            },
        }
    }
}

impl Insets {
    pub const fn uniform(value: LayoutUnit) -> Self {
        Self {
            start: value,
            top: value,
            end: value,
            bottom: value,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Interval {
    pub start: LayoutUnit,
    pub end: LayoutUnit,
}

impl Interval {
    pub fn width(self) -> LayoutUnit {
        self.end - self.start
    }
}

pub(crate) fn subtract_intervals(
    container: Interval,
    blocked: &mut Vec<Interval>,
) -> Vec<Interval> {
    blocked.retain(|interval| interval.end > container.start && interval.start < container.end);
    for interval in blocked.iter_mut() {
        interval.start = interval.start.max(container.start);
        interval.end = interval.end.min(container.end);
    }
    blocked.sort_by_key(|interval| (interval.start, interval.end));

    let mut merged: Vec<Interval> = Vec::with_capacity(blocked.len());
    for interval in blocked.iter().copied() {
        match merged.last_mut() {
            Some(last) if interval.start <= last.end => {
                last.end = last.end.max(interval.end);
            }
            _ => merged.push(interval),
        }
    }

    let mut available = Vec::with_capacity(merged.len() + 1);
    let mut cursor = container.start;
    for interval in merged {
        if interval.start > cursor {
            available.push(Interval {
                start: cursor,
                end: interval.start,
            });
        }
        cursor = cursor.max(interval.end);
    }
    if cursor < container.end {
        available.push(Interval {
            start: cursor,
            end: container.end,
        });
    }
    available
}
