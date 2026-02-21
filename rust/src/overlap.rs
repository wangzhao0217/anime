use geo_types::{Line, Point, Rect};
use std::ops::Range;

// TODO for handling geographic CRS
// calculate the distance from the top left to the bottom left corners
pub(crate) fn x_range(rect: &Rect) -> Range<f64> {
    rect.min().x..rect.max().x
}

// TODO for handling geographic CRS
// calculate the distance from the top left to the top right corners
pub(crate) fn y_range(rect: &Rect) -> Range<f64> {
    rect.min().y..rect.max().y
}
pub(crate) fn overlap_range(r1: Range<f64>, r2: Range<f64>) -> Option<Range<f64>> {
    if r1.end < r2.start || r2.end < r1.start {
        None
    } else {
        Some(r1.start.max(r2.start)..r1.end.min(r2.end))
    }
}

// When x range is known but y range is not, we need to solve for start and end points
// of the line segment
pub(crate) fn solve_no_y_overlap(x_overlap: Range<f64>, x: &Line, slope: &f64) -> (Point, Point) {
    let (known_x, known_y) = x.points().0.x_y();
    let b = known_y - (slope * known_x); // Corrected calculation of b

    let y1 = (slope * x_overlap.start) + b;
    let y2 = (slope * x_overlap.end) + b;
    let p1 = Point::new(x_overlap.start, y1);
    let p2 = Point::new(x_overlap.end, y2);
    (p1, p2)
}

pub(crate) fn solve_no_x_overlap(y_overlap: Range<f64>, x: &Line, slope: &f64) -> (Point, Point) {
    let (known_x, known_y) = x.points().0.x_y();
    let b = known_y - (slope * known_x); // Corrected calculation of b

    // create bindings to x vars that will be set in if statement
    let x1;
    let x2;

    // handle undefined slope
    if slope.is_infinite() || slope.is_nan() {
        // Assign a constant value to x1 and x2
        x1 = known_x;
        x2 = known_x;
    } else {
        x1 = (y_overlap.start - b) / slope;
        x2 = (y_overlap.end - b) / slope;
    }
    let p1 = Point::new(x1, y_overlap.start);
    let p2 = Point::new(x2, y_overlap.end);
    (p1, p2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{coord, Rect};

    #[test]
    fn test_x_range() {
        let rect = Rect::new(coord! {x: 0.0, y: 0.0}, coord! {x: 10.0, y: 5.0});
        let range = x_range(&rect);
        assert_eq!(range.start, 0.0);
        assert_eq!(range.end, 10.0);
    }

    #[test]
    fn test_y_range() {
        let rect = Rect::new(coord! {x: 0.0, y: 0.0}, coord! {x: 10.0, y: 5.0});
        let range = y_range(&rect);
        assert_eq!(range.start, 0.0);
        assert_eq!(range.end, 5.0);
    }

    #[test]
    fn test_overlap_range_with_overlap() {
        let r1 = 0.0..10.0;
        let r2 = 5.0..15.0;
        let overlap = overlap_range(r1, r2);
        assert!(overlap.is_some());
        let overlap = overlap.unwrap();
        assert_eq!(overlap.start, 5.0);
        assert_eq!(overlap.end, 10.0);
    }

    #[test]
    fn test_overlap_range_no_overlap() {
        let r1 = 0.0..5.0;
        let r2 = 10.0..15.0;
        let overlap = overlap_range(r1, r2);
        assert!(overlap.is_none());
    }

    #[test]
    fn test_overlap_range_touching() {
        let r1 = 0.0..5.0;
        let r2 = 5.0..10.0;
        let overlap = overlap_range(r1, r2);
        assert!(overlap.is_some());
        let overlap = overlap.unwrap();
        assert_eq!(overlap.start, 5.0);
        assert_eq!(overlap.end, 5.0);
    }

    #[test]
    fn test_overlap_range_complete_overlap() {
        let r1 = 0.0..10.0;
        let r2 = 2.0..8.0;
        let overlap = overlap_range(r1, r2);
        assert!(overlap.is_some());
        let overlap = overlap.unwrap();
        assert_eq!(overlap.start, 2.0);
        assert_eq!(overlap.end, 8.0);
    }

    #[test]
    fn test_overlap_range_reverse_order() {
        let r1 = 5.0..15.0;
        let r2 = 0.0..10.0;
        let overlap = overlap_range(r1, r2);
        assert!(overlap.is_some());
        let overlap = overlap.unwrap();
        assert_eq!(overlap.start, 5.0);
        assert_eq!(overlap.end, 10.0);
    }

    // https://github.com/JosiahParry/anime/issues/59
    // This reproduces the issue where two parallel near-vertical lines
    // can have opposite slope signs (one +89°, one -89°) depending on
    // whether dx is positive or negative, causing them to appear ~178° apart
    // and fail angle tolerance checks even though they are genuinely parallel.
    #[test]
    fn test_near_vertical_parallel_lines() {
        use crate::Anime;
        use geo_types::{coord, LineString};

        // Target line: going north, leaning very slightly left
        // dx = -0.5, dy = 50 -> slope ≈ -100
        let target = vec![LineString::new(vec![
            coord! {x: 100.0, y: 0.0},
            coord! {x: 99.5, y: 50.0},
        ])];

        // Source A (CLOSE, ~2m away): leaning slightly RIGHT (opposite slope sign)
        // dx = +0.5 -> slope ≈ +100
        let source_close =
            LineString::new(vec![coord! {x: 102.0, y: 0.0}, coord! {x: 102.5, y: 50.0}]);

        // Source B (FAR, ~10m away): leaning slightly left (same slope sign as target)
        // dx = -0.5 -> slope ≈ -100
        let source_far =
            LineString::new(vec![coord! {x: 110.0, y: 0.0}, coord! {x: 109.5, y: 50.0}]);

        let sources = vec![source_close, source_far];

        let anime = Anime::new(sources.into_iter(), target.into_iter(), 15.0, 20.0);

        let matches = anime.matches.get().unwrap();

        // Both sources should match the target since they are both parallel
        // and within distance tolerance
        if let Some(target_matches) = matches.get(&0) {
            // We expect BOTH source lines to match (source_id 0 and 1)
            assert_eq!(
                target_matches.len(),
                2,
                "Both near-vertical parallel lines should match regardless of slope sign"
            );

            // Verify both source indices are present
            let source_indices: Vec<usize> =
                target_matches.iter().map(|m| m.source_index).collect();
            assert!(
                source_indices.contains(&0),
                "Close source (opposite slope sign) should match"
            );
            assert!(
                source_indices.contains(&1),
                "Far source (same slope sign) should match"
            );
        } else {
            panic!("Target should have matches");
        }
    }

    // https://github.com/JosiahParry/anime/issues/59
    #[test]
    fn test_diagonal_parallel_lines() {
        use crate::Anime;
        use geo_types::{coord, LineString};

        // Target line at ~45° angle
        let target = vec![LineString::new(vec![
            coord! {x: 0.0, y: 0.0},
            coord! {x: 40.0, y: 50.0},
        ])];

        // Two parallel sources, shifted left and right
        let source_a = LineString::new(vec![coord! {x: 2.0, y: 0.0}, coord! {x: 42.0, y: 50.0}]);

        let source_b = LineString::new(vec![coord! {x: -2.0, y: 0.0}, coord! {x: 38.0, y: 50.0}]);

        let sources = vec![source_a, source_b];

        let anime = Anime::new(
            sources.into_iter(),
            target.into_iter(),
            5.0,  // distance_tolerance
            20.0, // angle_tolerance
        );

        let matches = anime.matches.get().unwrap();

        // Both diagonal parallels should match correctly
        if let Some(target_matches) = matches.get(&0) {
            assert_eq!(
                target_matches.len(),
                2,
                "Both diagonal parallel lines should match"
            );
        } else {
            panic!("Target should have matches");
        }
    }
}
