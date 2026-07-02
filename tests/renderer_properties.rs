//! Property-based tests for renderer layout scaling.
//!
//! Feature: lmahjong, Property 19: Layout Scaling Preserves Aspect Ratio

use lmahjong::renderer::{compute_layout_rect, LAYOUT_GRID_HEIGHT, LAYOUT_GRID_WIDTH};
use proptest::prelude::*;

// Feature: lmahjong, Property 19: Layout Scaling Preserves Aspect Ratio
// Validates: Requirements 12.2
proptest! {
    #[test]
    fn prop_layout_scaling_preserves_aspect_ratio(
        window_width in 800u32..=3840,
        window_height in 600u32..=2160
    ) {
        let metrics = compute_layout_rect(window_width, window_height);

        // Assert: layout dimensions are positive
        prop_assert!(
            metrics.layout_w > 0.0,
            "Layout width must be positive, got {}",
            metrics.layout_w
        );
        prop_assert!(
            metrics.layout_h > 0.0,
            "Layout height must be positive, got {}",
            metrics.layout_h
        );

        // Assert: layout fits within window
        prop_assert!(
            metrics.layout_w <= window_width as f32,
            "Layout width {} exceeds window width {}",
            metrics.layout_w,
            window_width
        );
        prop_assert!(
            metrics.layout_h <= window_height as f32,
            "Layout height {} exceeds window height {}",
            metrics.layout_h,
            window_height
        );

        // Assert: layout is centered (offset ≈ (window - layout) / 2)
        let expected_offset_x = (window_width as f32 - metrics.layout_w) / 2.0;
        let expected_offset_y = (window_height as f32 - metrics.layout_h) / 2.0;

        prop_assert!(
            (metrics.offset_x - expected_offset_x).abs() < 0.01,
            "Offset X {} not centered (expected {})",
            metrics.offset_x,
            expected_offset_x
        );
        prop_assert!(
            (metrics.offset_y - expected_offset_y).abs() < 0.01,
            "Offset Y {} not centered (expected {})",
            metrics.offset_y,
            expected_offset_y
        );

        // Assert: aspect ratio maintained (layout_w / layout_h ≈ 28.0 / 14.0 = 2.0)
        let expected_ratio = LAYOUT_GRID_WIDTH / LAYOUT_GRID_HEIGHT;
        let actual_ratio = metrics.layout_w / metrics.layout_h;

        prop_assert!(
            (actual_ratio - expected_ratio).abs() < 0.01,
            "Aspect ratio {} differs from expected {} (LAYOUT_GRID_WIDTH / LAYOUT_GRID_HEIGHT)",
            actual_ratio,
            expected_ratio
        );
    }
}
