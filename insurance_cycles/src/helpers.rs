//! Helper functions for insurance cycles simulation

use std::f64::consts::PI;

/// Calculate the shortest distance on a circle between two positions
///
/// Positions are in radians [0, 2π). The distance is the shorter of the
/// two possible arcs connecting the points.
///
/// # Examples
///
/// ```
/// use insurance_cycles::helpers::circular_distance;
/// use std::f64::consts::PI;
///
/// // Same position
/// assert_eq!(circular_distance(0.0, 0.0), 0.0);
///
/// // Opposite sides of circle
/// assert!((circular_distance(0.0, PI) - PI).abs() < 1e-10);
///
/// // Wrap-around case: 0.1 to 2π-0.1 = 0.2 (shorter than going the long way)
/// let dist = circular_distance(0.1, 2.0 * PI - 0.1);
/// assert!((dist - 0.2).abs() < 1e-10);
/// ```
pub fn circular_distance(pos1: f64, pos2: f64) -> f64 {
    let diff = (pos1 - pos2).abs();
    diff.min(2.0 * PI - diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_distance_same_position() {
        assert_eq!(circular_distance(0.0, 0.0), 0.0);
        assert_eq!(circular_distance(PI, PI), 0.0);
        assert_eq!(circular_distance(1.5, 1.5), 0.0);
    }

    #[test]
    fn test_circular_distance_opposite_sides() {
        // Maximum distance is π (half the circle)
        let dist = circular_distance(0.0, PI);
        assert!((dist - PI).abs() < 1e-10);

        let dist2 = circular_distance(PI, 0.0);
        assert!((dist2 - PI).abs() < 1e-10);
    }

    #[test]
    fn test_circular_distance_symmetry() {
        // Distance should be symmetric
        let pos1 = 1.0;
        let pos2 = 2.5;

        let dist1 = circular_distance(pos1, pos2);
        let dist2 = circular_distance(pos2, pos1);

        assert_eq!(dist1, dist2);
    }

    #[test]
    fn test_circular_distance_wraparound() {
        // 0.1 to (2π - 0.1) should be 0.2 (the short way around)
        let pos1 = 0.1;
        let pos2 = 2.0 * PI - 0.1;
        let dist = circular_distance(pos1, pos2);

        assert!((dist - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_circular_distance_quarter_circle() {
        // π/2 radians apart (90 degrees)
        let dist = circular_distance(0.0, PI / 2.0);
        assert!((dist - PI / 2.0).abs() < 1e-10);

        // 3π/2 radians the long way = π/2 the short way
        let dist2 = circular_distance(0.0, 3.0 * PI / 2.0);
        assert!((dist2 - PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_circular_distance_near_boundary() {
        // Test positions near 0 and 2π boundary
        let dist1 = circular_distance(0.01, 2.0 * PI - 0.01);
        assert!((dist1 - 0.02).abs() < 1e-10);

        let dist2 = circular_distance(2.0 * PI - 0.01, 0.01);
        assert!(dist2 < 0.1); // Should be small distance
    }

    #[test]
    fn test_circular_distance_always_positive() {
        // Distance should always be non-negative
        let positions = vec![0.0, 0.5, 1.0, PI / 2.0, PI, 3.0 * PI / 2.0, 2.0 * PI - 0.1];

        for &p1 in &positions {
            for &p2 in &positions {
                let dist = circular_distance(p1, p2);
                assert!(dist >= 0.0, "Distance should be non-negative");
                assert!(dist <= PI, "Distance should not exceed π");
            }
        }
    }
}
