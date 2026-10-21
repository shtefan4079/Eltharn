use crate::components::GuiEventListener;
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenAxis, ScreenDirection, ScreenRectangle,
};
use std::cmp::Ordering;
pub fn resolve_tab<W: GuiEventListener + ?Sized>(
    widgets: &[Box<W>],
    focused: Option<usize>,
    forward: bool,
) -> Option<usize> {
    let mut sorted: Vec<usize> = (0..widgets.len()).collect();
    sorted.sort_by_key(|&index| widgets[index].tab_order_group());
    let position = focused
        .filter(|&index| index < widgets.len())
        .and_then(|index| sorted.iter().position(|&candidate| candidate == index));
    let start = match position {
        Some(position) => position + usize::from(forward),
        None if forward => 0,
        None => sorted.len(),
    };
    let event = FocusNavigationEvent::TabNavigation { forward };
    let mut candidates: Box<dyn Iterator<Item = usize>> = if forward {
        Box::new(sorted.into_iter().skip(start))
    } else {
        Box::new(sorted.into_iter().take(start).rev())
    };
    candidates.find(|&index| widget_accepts_navigation(&*widgets[index], &event))
}
pub fn resolve_initial_focus<W: GuiEventListener + ?Sized>(widgets: &[Box<W>]) -> Option<usize> {
    resolve_tab(widgets, None, true)
}
pub fn resolve_arrow<W: GuiEventListener + ?Sized>(
    widgets: &[Box<W>],
    focused: usize,
    direction: ScreenDirection,
    previous_focus: Option<ScreenRectangle>,
) -> Option<usize> {
    if let Some(widget) = widgets.get(focused) {
        let border = widget.border_for_arrow_navigation(direction);
        next_focus_path_in_direction(widgets, border, direction, Some(focused))
    } else if let Some(previous) = previous_focus {
        next_focus_path_in_direction(widgets, previous, direction, None)
    } else {
        next_focus_path_in_direction(widgets, ScreenRectangle::empty(), direction, None)
    }
}
pub fn resolve_arrow_from<W: GuiEventListener + ?Sized>(
    widgets: &[Box<W>],
    origin: ScreenRectangle,
    direction: ScreenDirection,
) -> Option<usize> {
    next_focus_path_in_direction(widgets, origin, direction, None)
}
pub fn widget_accepts_navigation<W: GuiEventListener + ?Sized>(
    widget: &W,
    event: &FocusNavigationEvent,
) -> bool {
    widget.next_focus_path(event).is_some()
}
pub fn resolve_path<W: GuiEventListener + ?Sized>(
    widgets: &[Box<W>],
    path: &ComponentPath,
) -> Option<usize> {
    match path {
        ComponentPath::Leaf(handle) => widgets.get(*handle).map(|_| *handle),
        ComponentPath::Path { component, child } => {
            if widgets.get(*component).is_none() {
                return None;
            }
            resolve_path(widgets, child)
        }
    }
}
fn next_focus_path_in_direction<W: GuiEventListener + ?Sized>(
    widgets: &[Box<W>],
    origin: ScreenRectangle,
    direction: ScreenDirection,
    excluded: Option<usize>,
) -> Option<usize> {
    let axis = direction.axis();
    let other_axis = axis.orthogonal();
    let positive_other_axis = other_axis.positive();
    let origin_bound = bound_in_direction(origin, direction.opposite());
    let mut candidates: Vec<usize> = Vec::new();
    for (index, widget) in widgets.iter().enumerate() {
        if Some(index) == excluded {
            continue;
        }
        let rectangle = widget.rectangle();
        if !overlaps_in_axis(rectangle, origin, other_axis) {
            continue;
        }
        let child_bound = bound_in_direction(rectangle, direction.opposite());
        if direction.is_after(child_bound, origin_bound)
            || (child_bound == origin_bound
                && direction.is_after(
                    bound_in_direction(rectangle, direction),
                    bound_in_direction(origin, direction),
                ))
        {
            candidates.push(index);
        }
    }
    candidates.sort_by(|&left, &right| {
        let left_rectangle = widgets[left].rectangle();
        let right_rectangle = widgets[right].rectangle();
        compare_in_direction(
            direction,
            bound_in_direction(left_rectangle, direction.opposite()),
            bound_in_direction(right_rectangle, direction.opposite()),
        )
        .then_with(|| {
            compare_in_direction(
                positive_other_axis,
                bound_in_direction(left_rectangle, positive_other_axis.opposite()),
                bound_in_direction(right_rectangle, positive_other_axis.opposite()),
            )
        })
    });
    let event = FocusNavigationEvent::ArrowNavigation { direction };
    if let Some(index) = candidates
        .iter()
        .find(|&&index| widget_accepts_navigation(&*widgets[index], &event))
    {
        return Some(*index);
    }
    next_focus_path_vaguely_in_direction(widgets, origin, direction, excluded)
}
fn next_focus_path_vaguely_in_direction<W: GuiEventListener + ?Sized>(
    widgets: &[Box<W>],
    origin: ScreenRectangle,
    direction: ScreenDirection,
    excluded: Option<usize>,
) -> Option<usize> {
    let axis = direction.axis();
    let other_axis = axis.orthogonal();
    let origin_travel = bound_in_direction(origin, direction);
    let origin_other = center_in_axis(origin, other_axis);
    let mut candidates: Vec<(usize, i64)> = Vec::new();
    for (index, widget) in widgets.iter().enumerate() {
        if Some(index) == excluded {
            continue;
        }
        let rectangle = widget.rectangle();
        let travel = bound_in_direction(rectangle, direction.opposite());
        let other = center_in_axis(rectangle, other_axis);
        if !direction.is_after(travel, origin_travel) {
            continue;
        }
        let dx = i64::from(travel - origin_travel);
        let dy = i64::from(other - origin_other);
        candidates.push((index, dx * dx + dy * dy));
    }
    candidates.sort_by_key(|&(_, distance)| distance);
    let event = FocusNavigationEvent::ArrowNavigation { direction };
    candidates
        .iter()
        .find(|&&(index, _)| widget_accepts_navigation(&*widgets[index], &event))
        .map(|&(index, _)| index)
}
pub fn java_border(rectangle: ScreenRectangle, direction: ScreenDirection) -> ScreenRectangle {
    let axis = direction.axis();
    let orthogonal = axis.orthogonal();
    let start_first = bound_in_direction(rectangle, direction);
    let start_second = bound_in_direction(rectangle, orthogonal.negative());
    let length = match orthogonal {
        ScreenAxis::Horizontal => rectangle.width,
        ScreenAxis::Vertical => rectangle.height,
    };
    let (x, y, width, height) = match axis {
        ScreenAxis::Horizontal => (start_first, start_second, 1, length),
        ScreenAxis::Vertical => (start_second, start_first, length, 1),
    };
    let (x, y) = match direction {
        ScreenDirection::Left => (x - 1, y),
        ScreenDirection::Right => (x + 1, y),
        ScreenDirection::Up => (x, y - 1),
        ScreenDirection::Down => (x, y + 1),
    };
    ScreenRectangle::new(x, y, width, height)
}
fn bound_in_direction(rectangle: ScreenRectangle, direction: ScreenDirection) -> i32 {
    let (start, length) = match direction.axis() {
        ScreenAxis::Horizontal => (rectangle.left(), rectangle.width),
        ScreenAxis::Vertical => (rectangle.top(), rectangle.height),
    };
    if direction.is_positive() {
        start + length - 1
    } else {
        start
    }
}
fn center_in_axis(rectangle: ScreenRectangle, axis: ScreenAxis) -> i32 {
    let positive = bound_in_direction(rectangle, axis.positive());
    let negative = bound_in_direction(rectangle, axis.negative());
    (positive + negative) / 2
}
fn overlaps_in_axis(a: ScreenRectangle, b: ScreenRectangle, axis: ScreenAxis) -> bool {
    let a_lower = bound_in_direction(a, axis.negative());
    let b_lower = bound_in_direction(b, axis.negative());
    let a_upper = bound_in_direction(a, axis.positive());
    let b_upper = bound_in_direction(b, axis.positive());
    a_lower.max(b_lower) <= a_upper.min(b_upper)
}
fn compare_in_direction(direction: ScreenDirection, a: i32, b: i32) -> Ordering {
    if a == b {
        Ordering::Equal
    } else if direction.is_after(b, a) {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}
