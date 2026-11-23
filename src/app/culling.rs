//! The list of photos being culled: filter, current photo, navigation and decisions.
//! Independent of the user interface.

use crate::library::{Decision, Photo};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Unsorted,
    Keep,
    Remove,
}

impl Filter {
    fn matches(self, photo: &Photo) -> bool {
        match self {
            Filter::All => true,
            Filter::Unsorted => photo.decision.is_none(),
            Filter::Keep => photo.decision == Some(Decision::Keep),
            Filter::Remove => photo.decision == Some(Decision::Remove),
        }
    }
}

#[derive(Default)]
pub struct Culling {
    pub photos: Vec<Photo>,
    filter: Filter,
    /// Index into `photos`.
    pub current: Option<usize>,
}

impl Culling {
    /// Lists all the photos and starts at the first one not handled yet.
    pub fn new(photos: Vec<Photo>) -> Self {
        let current = photos
            .iter()
            .position(|p| p.decision.is_none())
            .or(if photos.is_empty() { None } else { Some(0) });
        Self {
            photos,
            filter: Filter::All,
            current,
        }
    }

    pub fn filter(&self) -> Filter {
        self.filter
    }

    /// Number of photos with this decision.
    pub fn count(&self, decision: Option<Decision>) -> usize {
        self.photos
            .iter()
            .filter(|p| p.decision == decision)
            .count()
    }

    /// Indices of the photos in the list, in order.
    pub fn visible(&self) -> Vec<usize> {
        (0..self.photos.len())
            .filter(|&i| self.filter.matches(&self.photos[i]))
            .collect()
    }

    /// Keeps the current photo if it is in the new list, otherwise goes to the first one.
    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
        let visible = self.visible();
        if !self.current.is_some_and(|c| visible.contains(&c)) {
            self.current = visible.first().copied();
        }
    }

    /// Goes to the next or previous photo in the list.
    /// Returns whether the view should scroll to the new current photo.
    pub fn step(&mut self, forward: bool) -> bool {
        let visible = self.visible();
        let Some(cur) = self.current else {
            self.current = visible.first().copied();
            return false;
        };
        let next = if forward {
            visible.iter().copied().find(|&i| i > cur)
        } else {
            visible.iter().rev().copied().find(|&i| i < cur)
        };
        let Some(next) = next else { return false };
        self.current = Some(next);
        true
    }

    /// Goes one gallery row of `columns` photos down or up, to the same column.
    /// Returns whether the view should scroll to the new current photo.
    pub fn step_row(&mut self, down: bool, columns: usize) -> bool {
        let visible = self.visible();
        let Some(pos) = self
            .current
            .and_then(|c| visible.iter().position(|&i| i == c))
        else {
            self.current = visible.first().copied();
            return false;
        };
        let columns = columns.max(1);
        let target = if down {
            pos.checked_add(columns)
        } else {
            pos.checked_sub(columns)
        };
        let Some(&next) = target.and_then(|t| visible.get(t)) else {
            return false;
        };
        self.current = Some(next);
        true
    }

    /// Sets the decision of the current photo. With `advance`, or when the photo leaves the
    /// list, goes to the next photo in the list, or to the previous one at the end.
    /// Returns whether the view should scroll to the new current photo.
    pub fn set_decision(&mut self, decision: Option<Decision>, advance: bool) -> bool {
        let Some(cur) = self.current else {
            return false;
        };
        self.photos[cur].decision = decision;

        let visible = self.visible();
        if !advance && visible.contains(&cur) {
            return false;
        }
        self.current = visible
            .iter()
            .copied()
            .find(|&i| i > cur)
            .or_else(|| visible.iter().rev().copied().find(|&i| i < cur))
            .or(visible.contains(&cur).then_some(cur));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const KEEP: Option<Decision> = Some(Decision::Keep);
    const REMOVE: Option<Decision> = Some(Decision::Remove);

    fn culling(decisions: &[Option<Decision>]) -> Culling {
        let photos = decisions
            .iter()
            .enumerate()
            .map(|(i, &decision)| Photo {
                stem: format!("P{i}"),
                jpeg: Some(PathBuf::from(format!("/shoot/P{i}.JPG"))),
                raw: None,
                decision,
            })
            .collect();
        Culling::new(photos)
    }

    #[test]
    fn starts_at_the_first_photo_not_handled_yet() {
        assert_eq!(culling(&[KEEP, REMOVE, None, None]).current, Some(2));
    }

    #[test]
    fn starts_at_the_first_photo_when_all_are_handled() {
        assert_eq!(culling(&[KEEP, REMOVE]).current, Some(0));
        assert_eq!(culling(&[]).current, None);
    }

    #[test]
    fn step_skips_photos_outside_the_list() {
        let mut c = culling(&[None, KEEP, None]);
        c.set_filter(Filter::Unsorted);
        assert!(c.step(true));
        assert_eq!(c.current, Some(2));
        assert!(c.step(false));
        assert_eq!(c.current, Some(0));
    }

    #[test]
    fn step_stops_at_the_ends_of_the_list() {
        let mut c = culling(&[None, None]);
        assert!(!c.step(false));
        assert_eq!(c.current, Some(0));
        c.current = Some(1);
        assert!(!c.step(true));
        assert_eq!(c.current, Some(1));
    }

    #[test]
    fn set_filter_keeps_the_current_photo_when_it_is_in_the_list() {
        let mut c = culling(&[KEEP, None, KEEP]);
        c.current = Some(2);
        c.set_filter(Filter::Keep);
        assert_eq!(c.current, Some(2));
        c.set_filter(Filter::Unsorted);
        assert_eq!(c.current, Some(1));
        c.set_filter(Filter::Remove);
        assert_eq!(c.current, None);
    }

    #[test]
    fn step_row_moves_by_one_row_and_stops_at_the_edges() {
        let mut c = culling(&[None; 7]);
        c.current = Some(1);
        assert!(c.step_row(true, 3));
        assert_eq!(c.current, Some(4));
        assert!(!c.step_row(true, 3));
        assert_eq!(c.current, Some(4));
        assert!(c.step_row(false, 3));
        assert_eq!(c.current, Some(1));
        assert!(!c.step_row(false, 3));
    }

    #[test]
    fn decision_with_advance_goes_to_the_next_photo() {
        let mut c = culling(&[None, None, None]);
        assert!(c.set_decision(KEEP, true));
        assert_eq!(c.photos[0].decision, KEEP);
        assert_eq!(c.current, Some(1));
    }

    #[test]
    fn decision_on_the_last_photo_goes_to_the_previous_one() {
        let mut c = culling(&[None, None]);
        c.current = Some(1);
        assert!(c.set_decision(REMOVE, true));
        assert_eq!(c.current, Some(0));
    }

    #[test]
    fn decision_on_the_only_photo_stays_on_it() {
        let mut c = culling(&[None]);
        assert!(c.set_decision(KEEP, true));
        assert_eq!(c.current, Some(0));
    }

    #[test]
    fn cleared_photo_that_leaves_the_list_moves_to_the_next_one() {
        let mut c = culling(&[KEEP, KEEP]);
        c.set_filter(Filter::Keep);
        assert!(c.set_decision(None, false));
        assert_eq!(c.current, Some(1));
    }

    #[test]
    fn decision_without_advance_stays_on_the_photo() {
        let mut c = culling(&[None, None]);
        assert!(!c.set_decision(KEEP, false));
        assert_eq!(c.current, Some(0));
    }

    #[test]
    fn decision_without_current_photo_does_nothing() {
        let mut c = culling(&[None]);
        c.current = None;
        assert!(!c.set_decision(KEEP, true));
        assert_eq!(c.photos[0].decision, None);
    }
}
