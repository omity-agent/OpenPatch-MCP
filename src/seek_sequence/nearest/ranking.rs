use crate::seek_sequence::SequenceMatch;
use core::cmp::Ordering;
pub(super) const THRESHOLD: Similarity = Similarity {
    matching: 1,
    maximum: 2,
};
#[derive(Clone, Copy)]
pub(super) struct Similarity {
    matching: usize,
    maximum: usize,
}
impl Similarity {
    fn from_distance(maximum: usize, distance: usize) -> Self {
        Self {
            matching: maximum
                .checked_sub(distance)
                .unwrap_or_else(|| panic!("Levenshtein distance must not exceed maximum")),
            maximum,
        }
    }
    pub(super) fn length_bound(pattern_length: usize, candidate_length: usize) -> Self {
        Self {
            matching: pattern_length.min(candidate_length),
            maximum: pattern_length.max(candidate_length),
        }
    }
    pub(super) fn compare(self, other: Self) -> Ordering {
        let left = widen(self.matching) * widen(other.maximum);
        let right = widen(other.matching) * widen(self.maximum);
        left.cmp(&right)
    }
    pub(super) const fn is_exact(self) -> bool {
        self.matching == self.maximum
    }
    pub(super) fn distance_cutoff(self, candidate_maximum: usize) -> usize {
        scaled_distance(
            self.maximum - self.matching,
            candidate_maximum,
            self.maximum,
        )
    }
    pub(super) fn strict_distance_cutoff(self, candidate_maximum: usize) -> usize {
        let scaled = widen(self.maximum - self.matching) * widen(candidate_maximum);
        let strict = scaled
            .checked_sub(1)
            .unwrap_or_else(|| panic!("similarity threshold must permit a positive distance"));
        usize::try_from(strict.div_euclid(widen(self.maximum)))
            .unwrap_or_else(|_| panic!("distance cutoff must fit usize"))
    }
}
fn scaled_distance(numerator: usize, scale: usize, denominator: usize) -> usize {
    let scaled = widen(numerator) * widen(scale);
    usize::try_from(scaled.div_euclid(widen(denominator)))
        .unwrap_or_else(|_| panic!("distance cutoff must fit usize"))
}
fn widen(value: usize) -> u128 {
    u128::try_from(value).unwrap_or_else(|_| panic!("usize must fit u128"))
}
pub(super) struct Candidate {
    pub(super) sequence: SequenceMatch,
    pub(super) similarity: Similarity,
    distance: usize,
    length_delta: usize,
}
impl Candidate {
    pub(super) fn new(
        sequence: SequenceMatch,
        pattern_length: usize,
        fragment_length: usize,
        distance: usize,
    ) -> Self {
        Self {
            sequence,
            similarity: Similarity::from_distance(pattern_length.max(fragment_length), distance),
            distance,
            length_delta: pattern_length.abs_diff(fragment_length),
        }
    }
    pub(super) fn is_better_than(&self, other: &Self) -> bool {
        self.similarity
            .compare(other.similarity)
            .then_with(|| other.distance.cmp(&self.distance))
            .then_with(|| other.length_delta.cmp(&self.length_delta))
            .then_with(|| other.sequence.start.cmp(&self.sequence.start))
            .then_with(|| other.sequence.length.cmp(&self.sequence.length))
            .is_gt()
    }
}
