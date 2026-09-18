pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        return 0.0;
    }

    dot / denom
}

// Pure arithmetic: no Ollama, no filesystem, nothing to mock.
#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn identical_vectors_score_one() {
        let v = vec![0.5, 0.25, 0.125, 1.0];
        assert!(close(cosine_similarity(&v, &v), 1.0));
    }

    #[test]
    fn orthogonal_vectors_score_zero() {
        assert!(close(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]), 0.0));
    }

    #[test]
    fn opposed_vectors_score_minus_one() {
        assert!(close(cosine_similarity(&[1.0, 2.0], &[-1.0, -2.0]), -1.0));
    }

    #[test]
    fn magnitude_does_not_matter() {
        let short = [1.0, 2.0, 3.0];
        let long = [10.0, 20.0, 30.0];
        assert!(close(cosine_similarity(&short, &long), 1.0));
    }

    #[test]
    fn zero_vector_on_either_side_scores_zero() {
        let zero = [0.0, 0.0, 0.0];
        let v = [1.0, 2.0, 3.0];

        // The denom == 0.0 guard: without it these would be NaN.
        assert!(close(cosine_similarity(&zero, &v), 0.0));
        assert!(close(cosine_similarity(&v, &zero), 0.0));
        assert!(close(cosine_similarity(&zero, &zero), 0.0));
    }

    #[test]
    fn empty_slices_score_zero() {
        let empty: [f32; 0] = [];
        assert!(close(cosine_similarity(&empty, &empty), 0.0));
        assert!(close(cosine_similarity(&empty, &[1.0, 2.0]), 0.0));
    }

    #[test]
    fn nan_input_propagates() {
        // Documented, not endorsed. engine::score_and_rank coerces the NaN to
        // zero afterwards, which is the only reason this does not poison the
        // result ordering.
        let result = cosine_similarity(&[f32::NAN, 1.0], &[1.0, 1.0]);
        assert!(result.is_nan());
    }

    #[test]
    fn mismatched_lengths_silently_compare_the_common_prefix() {
        // This is a trap, and the test exists to document it rather than to
        // bless it. zip() stops at the shorter input and both norms are
        // computed over that prefix only, so a 3-dimensional vector and its
        // own 2-dimensional truncation are reported as a perfect match
        // instead of as the dimension mismatch they are.
        let a = [1.0, 0.0, 99.0];
        let b = [1.0, 0.0];
        assert!(close(cosine_similarity(&a, &b), 1.0));

        // Same in the other direction.
        assert!(close(cosine_similarity(&b, &a), 1.0));

        // And a mismatch never errors, because there is no error to return.
        let _ = cosine_similarity(&[1.0], &[1.0, 2.0, 3.0, 4.0]);
    }
}
