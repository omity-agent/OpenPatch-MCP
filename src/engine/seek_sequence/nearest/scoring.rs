pub(super) const MATCH_SCORE: i64 = 3;
pub(super) const MISMATCH_SCORE: i64 = -3;
pub(super) const GAP_OPEN_SCORE: i64 = -5;
pub(super) const GAP_EXTEND_SCORE: i64 = -1;
pub(super) const MIN_ACCEPTED_SCORE: i64 = 0;
pub(super) const MAX_STEP_MAGNITUDE: i64 = 5;
pub(super) const fn substitution_score(pattern: char, target: char) -> i64 {
    if pattern == target {
        MATCH_SCORE
    } else {
        MISMATCH_SCORE
    }
}
