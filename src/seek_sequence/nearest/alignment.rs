use super::scoring::{
    GAP_EXTEND_SCORE, GAP_OPEN_SCORE, MATCH_SCORE, MAX_STEP_MAGNITUDE, substitution_score,
};
const INVALID_SCORE: i64 = i64::MIN >> 1;
#[derive(Clone, Copy)]
struct Cell {
    score: i64,
    start: usize,
}
impl Cell {
    const INVALID: Self = Self {
        score: INVALID_SCORE,
        start: 0,
    };
    const fn new(score: i64, start: usize) -> Self {
        Self { score, start }
    }
    const fn add(self, score: i64) -> Self {
        if self.score == INVALID_SCORE {
            self
        } else {
            Self::new(self.score + score, self.start)
        }
    }
    const fn best(self, other: Self) -> Self {
        if other.score > self.score || other.score == self.score && other.start > self.start {
            other
        } else {
            self
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Alignment {
    pub(super) score: i64,
    pub(super) start: usize,
    pub(super) end: usize,
}
impl Alignment {
    const fn from_cell(cell: Cell, end: usize) -> Self {
        Self {
            score: cell.score,
            start: cell.start,
            end,
        }
    }
    const fn is_better_than(self, other: Self) -> bool {
        self.score > other.score
            || self.score == other.score
                && (self.end - self.start < other.end - other.start
                    || self.end - self.start == other.end - other.start && self.start < other.start)
    }
}
pub(super) fn fit(pattern: &[char], target: &[char]) -> Alignment {
    validate_dimensions(pattern.len(), target.len());
    let width = pattern
        .len()
        .checked_add(1)
        .unwrap_or_else(|| panic!("alignment row width must fit usize"));
    let mut previous = vec![Cell::INVALID; width];
    let mut current = vec![Cell::INVALID; width];
    let mut previous_insertions = vec![Cell::INVALID; width];
    let mut current_insertions = vec![Cell::INVALID; width];
    initialize_pattern_row(&mut previous);
    let maximum_score = i64::try_from(pattern.len())
        .unwrap_or_else(|_| panic!("pattern length must fit i64"))
        * MATCH_SCORE;
    let mut best = Alignment {
        score: INVALID_SCORE,
        start: 0,
        end: 0,
    };
    for (target_index, target_character) in target.iter().copied().enumerate() {
        let end = target_index + 1;
        let Some((current_zero, current_cells)) = current.split_first_mut() else {
            panic!("alignment row must contain its zero column");
        };
        *current_zero = Cell::new(0, end);
        let Some((current_insertion_zero, current_insertion_cells)) =
            current_insertions.split_first_mut()
        else {
            panic!("insertion row must contain its zero column");
        };
        *current_insertion_zero = Cell::INVALID;
        let mut deletion = Cell::INVALID;
        let mut left = *current_zero;
        let previous_pairs = previous.windows(2);
        let previous_insertion_cells = previous_insertions.iter().skip(1);
        let columns = current_cells
            .iter_mut()
            .zip(current_insertion_cells)
            .zip(previous_pairs)
            .zip(previous_insertion_cells)
            .zip(pattern.iter().copied());
        for (
            (((current_cell, current_insertion), previous_pair), previous_insertion),
            pattern_character,
        ) in columns
        {
            let &[diagonal_source, insertion_source] = previous_pair else {
                panic!("previous alignment window must contain two cells");
            };
            deletion = left
                .add(GAP_OPEN_SCORE)
                .best(deletion.add(GAP_EXTEND_SCORE));
            let insertion = insertion_source
                .add(GAP_OPEN_SCORE)
                .best(previous_insertion.add(GAP_EXTEND_SCORE));
            *current_insertion = insertion;
            let diagonal =
                diagonal_source.add(substitution_score(pattern_character, target_character));
            let resolved = diagonal.best(deletion).best(insertion);
            *current_cell = resolved;
            left = resolved;
        }
        let Some(last) = current.last().copied() else {
            panic!("alignment row must contain its zero column");
        };
        let candidate = Alignment::from_cell(last, end);
        if candidate.score == maximum_score {
            return candidate;
        }
        if candidate.start < candidate.end && candidate.is_better_than(best) {
            best = candidate;
        }
        core::mem::swap(&mut previous, &mut current);
        core::mem::swap(&mut previous_insertions, &mut current_insertions);
    }
    best
}
fn initialize_pattern_row(row: &mut [Cell]) {
    let Some(first) = row.first_mut() else {
        panic!("alignment row must contain its zero column");
    };
    *first = Cell::new(0, 0);
    let mut deletion = Cell::INVALID;
    let mut left = *first;
    for cell in row.iter_mut().skip(1) {
        deletion = left
            .add(GAP_OPEN_SCORE)
            .best(deletion.add(GAP_EXTEND_SCORE));
        *cell = deletion;
        left = deletion;
    }
}
fn validate_dimensions(pattern_length: usize, target_length: usize) {
    let total_length = pattern_length
        .checked_add(target_length)
        .unwrap_or_else(|| panic!("combined alignment length must fit usize"));
    let total_score = i64::try_from(total_length)
        .unwrap_or_else(|_| panic!("combined alignment length must fit i64"))
        .checked_mul(MAX_STEP_MAGNITUDE)
        .unwrap_or_else(|| panic!("alignment score range must fit i64"));
    assert!(
        total_score < -INVALID_SCORE,
        "alignment score range must stay above the invalid sentinel"
    );
}
