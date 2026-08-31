use crate::LayoutUnit;

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
