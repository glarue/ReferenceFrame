use super::*;

impl PlanViewGeometry {
    /// Build all rectangles from pre-computed scale and origin.
    ///
    /// When `display_dims` is `None`, dimensions come directly from the design
    /// (standard path). When `Some((artwork_w, artwork_h, outer_w, outer_h))`,
    /// display (compressed) dimensions are used for axis-break rendering while
    /// frame band, mat border, and rabbet still use actual design dimensions.
    fn build_rects(
        design: &FrameDesign,
        scale: f64,
        origin_x: f64,
        origin_y: f64,
        display_dims: Option<(f64, f64, f64, f64)>,
    ) -> Self {
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();

        // Resolve outer and inner dimensions: use display overrides when present,
        // otherwise derive from the design's actual measurements.
        let (outer_w, outer_h, inner_w, inner_h) = if let Some((_, _, disp_outer_w, disp_outer_h)) = display_dims {
            // Display inner = display outer minus frame band on each side
            let disp_inner_w = disp_outer_w - 2.0 * design.frame_material_width;
            let disp_inner_h = disp_outer_h - 2.0 * design.frame_material_width;
            (disp_outer_w, disp_outer_h, disp_inner_w, disp_inner_h)
        } else {
            let (fi_h, fi_w) = design.get_frame_inside_dimensions();
            (frame_outer_width, frame_outer_height, fi_w, fi_h)
        };

        let origin = Point::new(origin_x, origin_y);
        let frame_outer = Rect::new(origin_x, origin_y, outer_w * scale, outer_h * scale);

        let frame_width_scaled = design.frame_material_width * scale;
        let frame_inner = Rect::new(
            origin_x + frame_width_scaled,
            origin_y + frame_width_scaled,
            inner_w * scale,
            inner_h * scale,
        );

        // Mat geometry (if mat is present)
        let (mat_visible, mat_opening) = if design.has_mat() {
            let (mat_opening_height, mat_opening_width) = design.get_mat_opening_dimensions();

            // When display overrides are active, compress mat opening by the same
            // amount as artwork (mat_opening = artwork + 2*mat_overlap).
            let (mat_open_w, mat_open_h) = if let Some((disp_art_w, disp_art_h, _, _)) = display_dims {
                (mat_opening_width  - (design.artwork_width  - disp_art_w),
                 mat_opening_height - (design.artwork_height - disp_art_h))
            } else {
                (mat_opening_width, mat_opening_height)
            };

            let mat_opening_scaled_w = mat_open_w * scale;
            let mat_opening_scaled_h = mat_open_h * scale;

            let mat_vis = Some(frame_inner);
            let opening_x = frame_inner.x + (frame_inner.width - mat_opening_scaled_w) / 2.0;
            let opening_y = frame_inner.y + (frame_inner.height - mat_opening_scaled_h) / 2.0;
            let mat_open = Some(Rect::new(opening_x, opening_y, mat_opening_scaled_w, mat_opening_scaled_h));

            (mat_vis, mat_open)
        } else {
            (None, None)
        };

        // Content area (extends under the frame lip by lip_over_art — zero for
        // sight-size/float, so the artwork fills the opening exactly).
        let rabbet_width_scaled = design.lip_over_art() * scale;
        let content_area = Rect::new(
            frame_inner.x - rabbet_width_scaled,
            frame_inner.y - rabbet_width_scaled,
            frame_inner.width + 2.0 * rabbet_width_scaled,
            frame_inner.height + 2.0 * rabbet_width_scaled,
        );

        // Artwork rectangle: use display artwork when overrides are active
        let (art_w, art_h) = if let Some((disp_art_w, disp_art_h, _, _)) = display_dims {
            (disp_art_w, disp_art_h)
        } else {
            (design.artwork_width, design.artwork_height)
        };
        let artwork_scaled_w = art_w * scale;
        let artwork_scaled_h = art_h * scale;
        let artwork = if design.has_mat() {
            Rect::new(
                content_area.x + (content_area.width - artwork_scaled_w) / 2.0,
                content_area.y + (content_area.height - artwork_scaled_h) / 2.0,
                artwork_scaled_w,
                artwork_scaled_h,
            )
        } else {
            content_area
        };

        Self {
            frame_outer,
            frame_inner,
            mat_visible,
            mat_opening,
            artwork,
            content_area,
            scale,
            origin,
            use_axis_break_x: false,
            use_axis_break_y: false,
            break_x_start: 0.0,
            break_x_end: 0.0,
            break_y_start: 0.0,
            break_y_end: 0.0,
            thumbnail: None,
            thumbnail_below: false,
            corner_detail: None,
            thumbnail_label_position: ThumbnailLabelPosition::Below,

            annotation_bounds: AnnotationBounds::empty(),
        }
    }

    /// Calculate geometry from a frame design
    pub fn from_design(
        design: &FrameDesign,
        canvas_width: f64,
        canvas_height: f64,
        style: &DiagramStyle,
    ) -> Self {
        Self::from_design_with_mode(design, canvas_width, canvas_height, style, DetailMode::Auto, true, true, false, false, false)
    }

    /// Calculate geometry with explicit detail mode and feature flags.
    ///
    /// High-level recipe:
    /// 1. Decide which axes need breaks and whether corner detail is needed.
    /// 2. Compute compressed display dimensions for break axes.
    /// 3. Build base geometry (rects, scale, origin) from display dimensions.
    /// 4. Place break gap indicators, corner detail, mat cut extent, and thumbnail.
    pub fn from_design_with_mode(
        design: &FrameDesign,
        canvas_width: f64,
        canvas_height: f64,
        style: &DiagramStyle,
        detail_mode: DetailMode,
        corner_detail_enabled: bool,
        axis_breaks_enabled: bool,
        unit_mm: bool,
        use_tape_segments: bool,
        use_decimal: bool,
    ) -> Self {
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();

        // Calculate available canvas area (accounting for margins and dimension callouts).
        // Top and right always have callouts (frame dims); bottom and left only when mat present.
        let callout_reservation = style.dimension_offset_base + style.dimension_offset_step;
        let has_mat = design.has_mat();
        let top_reserve = callout_reservation;
        let bottom_reserve = if has_mat { callout_reservation } else { style.margin };
        let right_reserve = callout_reservation;
        let left_reserve = if has_mat { callout_reservation } else { style.margin };
        let available_width = canvas_width - 2.0 * style.margin - right_reserve - left_reserve;
        let available_height = canvas_height - 2.0 * style.margin - top_reserve - bottom_reserve;

        // Trial scale per axis: how many pixels per inch at full fit
        let native_scale_x = available_width / frame_outer_width;
        let native_scale_y = available_height / frame_outer_height;
        let native_scale = native_scale_x.min(native_scale_y);

        // Phase 1: Decide break strategy
        let bd = decide_axis_breaks(
            design, frame_outer_width, frame_outer_height, native_scale,
            style, detail_mode, corner_detail_enabled, axis_breaks_enabled,
            unit_mm, use_tape_segments, use_decimal,
        );

        if !bd.use_break_x && !bd.use_break_y {
            return Self::build_no_break_geometry(
                design, native_scale, canvas_width, canvas_height,
                bd.use_corner_detail, style,
            );
        }

        // Phase 2: Compute compressed display dimensions
        let dd = compute_display_dimensions(
            design, frame_outer_width, frame_outer_height,
            bd.use_break_x, bd.use_break_y, bd.frame_band,
            available_width, available_height, canvas_width, canvas_height,
            native_scale_x, style,
        );

        if !dd.use_break_x && !dd.use_break_y {
            return Self::build_no_break_geometry(
                design, native_scale, canvas_width, canvas_height,
                bd.use_corner_detail, style,
            );
        }

        // Phase 3: Build base geometry from display dimensions, or fall back
        // to no-break if the marginal break guard rejects the compression.
        let mut geo = match Self::build_break_geometry(
            design, &dd, frame_outer_width, frame_outer_height,
            native_scale, canvas_width, canvas_height,
            available_height, bd.use_corner_detail, style,
        ) {
            Some(g) => g,
            None => return Self::build_no_break_geometry(
                design, native_scale, canvas_width, canvas_height,
                bd.use_corner_detail, style,
            ),
        };

        // Phase 4: Place overlays — break gaps, corner detail, mat cut, thumbnail
        Self::apply_break_positions(&mut geo, dd.use_break_x, dd.use_break_y);
        Self::place_corner_detail_if_needed(&mut geo, design, canvas_width, bd.use_corner_detail, style);
        let (mat_cut_extent, occupied) = Self::compute_mat_cut_and_occupied(&geo, design, style);

        Self::compute_thumbnail_placement(
            &mut geo, frame_outer_width, frame_outer_height,
            &occupied, mat_cut_extent, style,
        );

        geo
    }

    /// Build base geometry for the axis-break path.
    ///
    /// Computes display outer dimensions, applies the marginal break guard
    /// (rejecting single-axis breaks that don't meaningfully improve the
    /// aspect ratio), then calculates final scale, origin, and rects.
    /// Returns `None` if the marginal guard rejects the break.
    fn build_break_geometry(
        design: &FrameDesign,
        dd: &DisplayDimensions,
        frame_outer_width: f64,
        frame_outer_height: f64,
        _native_scale: f64,
        canvas_width: f64,
        canvas_height: f64,
        available_height: f64,
        _use_corner_detail: bool,
        style: &DiagramStyle,
    ) -> Option<Self> {
        // Display outer = actual_outer - actual_artwork + display_artwork
        let display_outer_w = frame_outer_width - design.artwork_width + dd.artwork_w;
        let display_outer_h = frame_outer_height - design.artwork_height + dd.artwork_h;

        // Marginal break guard (single-axis only): if the break barely compresses
        // the frame, the break gap (8px) can make the visual aspect ratio as extreme
        // as (or more than) the actual frame. Skip the break when the rendered AR
        // wouldn't improve over the true AR by at least 10%.
        // Dual-axis breaks always compress meaningfully (both axes are extreme).
        let is_single_axis = dd.use_break_x != dd.use_break_y;
        {
            let actual_ar = (frame_outer_width / frame_outer_height)
                .max(frame_outer_height / frame_outer_width);
            let display_ar = (display_outer_w / display_outer_h)
                .max(display_outer_h / display_outer_w);
            if is_single_axis && display_ar > actual_ar * BREAK_IMPROVEMENT_THRESHOLD {
                return None;
            }
        }

        // Final scale from display dimensions.
        // Axis break frames have no left-side callouts, so we can use asymmetric
        // margins: only `margin` on the left, full callout space on the right.
        // This gives the frame more horizontal space to expand into.
        let right_offset = style.margin + style.dimension_offset_base + style.dimension_offset_step;
        let left_offset = style.margin;
        let break_available_width = canvas_width - left_offset - right_offset;
        let scale_x = break_available_width / display_outer_w;
        let scale_y = available_height / display_outer_h;
        let scale = scale_x.min(scale_y);

        let scaled_width = display_outer_w * scale;
        let scaled_height = display_outer_h * scale;

        let origin_x = ((break_available_width - scaled_width) / 2.0 + left_offset).max(left_offset);
        let min_offset_y = style.total_callout_reserve();
        let origin_y = ((canvas_height - scaled_height) / 2.0).max(min_offset_y);

        // Build rects using display artwork dimensions.
        // Frame band, mat border, rabbet all use actual design dimensions at the new scale.
        Some(Self::build_rects(
            design, scale, origin_x, origin_y,
            Some((dd.artwork_w, dd.artwork_h, display_outer_w, display_outer_h)),
        ))
    }

    /// Set break gap positions on the geometry.
    ///
    /// Offsets breaks so the top-left corner gets more visible area
    /// (biased toward the corner where the corner detail overlay sits).
    fn apply_break_positions(geo: &mut Self, use_break_x: bool, use_break_y: bool) {
        let break_center_x = geo.artwork.x + geo.artwork.width * BREAK_CENTER_BIAS_X;
        let break_center_y = geo.artwork.y + geo.artwork.height * BREAK_CENTER_BIAS_Y;

        geo.use_axis_break_x = use_break_x;
        geo.use_axis_break_y = use_break_y;

        if use_break_x {
            geo.break_x_start = break_center_x - BREAK_GAP_PX / 2.0;
            geo.break_x_end = break_center_x + BREAK_GAP_PX / 2.0;
        }
        if use_break_y {
            geo.break_y_start = break_center_y - BREAK_GAP_PX / 2.0;
            geo.break_y_end = break_center_y + BREAK_GAP_PX / 2.0;
        }
    }

    /// Conditionally add corner detail overlay when the frame face is too
    /// narrow at the current scale to show internal details (rabbet, mat overlap).
    fn place_corner_detail_if_needed(
        geo: &mut Self,
        design: &FrameDesign,
        canvas_width: f64,
        use_corner_detail: bool,
        style: &DiagramStyle,
    ) {
        // Corner detail zooms the rabbet lip / mat overlap; there's nothing to
        // show for sight-size/float (no lip over the art), so skip it there.
        if use_corner_detail && design.frame_material_width > 0.0 && design.lip_over_art() > 0.0 {
            geo.corner_detail = Some(Self::compute_corner_detail(design, geo, canvas_width, style));
        }
    }

    /// Compute mat cut extent and build the occupied-rect list for thumbnail placement.
    ///
    /// Two-pass placement: computes mat cut extent first (choosing left or right side
    /// based on corner detail position), then assembles the occupied list so thumbnail
    /// placement is collision-free without approximation loops.
    fn compute_mat_cut_and_occupied(
        geo: &Self,
        design: &FrameDesign,
        style: &DiagramStyle,
    ) -> (Option<(Point, Point)>, Vec<Rect>) {
        let cd_occupied: Vec<Rect> = geo.corner_detail.as_ref()
            .map(|cd| vec![cd.box_rect]).unwrap_or_default();

        let mat_cut_extent: Option<(Point, Point)> = if design.has_mat() {
            geo.mat_opening.as_ref().map(|mat_opening| {
                Self::choose_mat_cut_extent(
                    &geo.frame_inner,
                    &geo.content_area,
                    mat_opening,
                    &cd_occupied,
                    style,
                )
            })
        } else {
            None
        };

        // Build occupied list from already-placed elements (corner detail + mat cut label).
        let mut occupied: Vec<Rect> = Vec::new();
        if let Some(cd) = &geo.corner_detail {
            occupied.push(cd.box_rect);
            if let Some((ref start, ref end)) = mat_cut_extent {
                occupied.push(Self::mat_cut_label_bounds_from_extent(
                    &geo.frame_outer, start, end, style,
                ));
            }
        }

        (mat_cut_extent, occupied)
    }

    /// Build geometry for the no-break (standard) path.
    ///
    /// Computes origin from native scale, optionally adds corner detail,
    /// computes mat cut extent, and returns the fully-populated geometry.
    /// No thumbnail is placed on the no-break path.
    fn build_no_break_geometry(
        design: &FrameDesign,
        native_scale: f64,
        canvas_width: f64,
        canvas_height: f64,
        use_corner_detail: bool,
        style: &DiagramStyle,
    ) -> Self {
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();
        let scale = native_scale;
        let scaled_width = frame_outer_width * scale;
        let scaled_height = frame_outer_height * scale;

        let min_offset = style.total_callout_reserve();
        let origin_x = ((canvas_width - scaled_width) / 2.0).max(min_offset);
        let origin_y = ((canvas_height - scaled_height) / 2.0).max(min_offset);

        let mut geo = Self::build_rects(design, scale, origin_x, origin_y, None);

        if use_corner_detail && design.frame_material_width > 0.0 && design.lip_over_art() > 0.0 {
            geo.corner_detail = Some(Self::compute_corner_detail(design, &geo, canvas_width, style));
        }

        let cd_occupied: Vec<Rect> = geo.corner_detail.as_ref().map(|cd| vec![cd.box_rect]).unwrap_or_default();
        let mat_cut_extent: Option<(Point, Point)> = if design.has_mat() {
            geo.mat_opening.as_ref().map(|mat_opening| {
                Self::choose_mat_cut_extent(
                    &geo.frame_inner,
                    &geo.content_area,
                    mat_opening,
                    &cd_occupied,
                    style,
                )
            })
        } else {
            None
        };

        geo.annotation_bounds = AnnotationBounds {
            corner_detail_box: geo.corner_detail.as_ref().map(|cd| cd.box_rect),
            mat_cut_extent,
            ..AnnotationBounds::empty()
        };

        geo
    }

    /// Compute thumbnail sizing, preferred position, and annotation bounds.
    ///
    /// Places a proportional silhouette thumbnail in the margin around the
    /// frame, avoiding collision with corner detail and mat cut labels.
    /// The collision pass in svg.rs handles fine adjustments (nudging away
    /// from arrow stubs, callout labels, etc.).
    fn compute_thumbnail_placement(
        geo: &mut Self,
        frame_outer_width: f64,
        frame_outer_height: f64,
        occupied: &[Rect],
        mat_cut_extent: Option<(Point, Point)>,
        style: &DiagramStyle,
    ) {
        let tm = style.thumbnail_metrics();
        let is_portrait = frame_outer_height >= frame_outer_width;
        let has_cd_and_mc = occupied.len() == 2;

        // Sizing: rotation-invariant when CD + MC both present (smaller thumb to fit gap),
        // standard orientation-aware sizing otherwise.
        let (thumb_w, thumb_h) = if has_cd_and_mc {
            let frame_long = frame_outer_width.max(frame_outer_height);
            let frame_short = frame_outer_width.min(frame_outer_height);
            let mini_max_h = style.two_line_label_bounds_height() * tm.scale_factor;
            let thumb_scale = (THUMBNAIL_MINI_MAX_WIDTH / frame_long).min(mini_max_h / frame_short);
            ((frame_outer_width * thumb_scale).max(tm.min_px),
             (frame_outer_height * thumb_scale).max(tm.min_px))
        } else {
            let (thumbnail_max_w, thumbnail_max_h) = if is_portrait {
                (tm.short_dim, tm.long_dim)
            } else {
                (tm.long_dim, tm.short_dim)
            };
            let scale_w = thumbnail_max_w / frame_outer_width;
            let scale_h = thumbnail_max_h / frame_outer_height;
            let thumb_scale = scale_w.min(scale_h);
            ((frame_outer_width * thumb_scale).max(tm.min_px),
             (frame_outer_height * thumb_scale).max(tm.min_px))
        };

        let label_below_h = tm.text_below_height;

        // Preferred position: one per orientation, with CD/MC gap awareness.
        let (thumb_x, thumb_y, thumb_label_pos) = if is_portrait {
            // Left of frame, vertically centered, label below
            let x = geo.frame_outer.left() - tm.gap - thumb_w;
            let centered_y = if has_cd_and_mc {
                geo.frame_outer.top() + (geo.frame_outer.height - (thumb_h + label_below_h)) / 2.0
            } else {
                geo.frame_outer.top() + (geo.frame_outer.height - thumb_h) / 2.0
            };
            // If centered position overlaps corner detail, shift above it
            if let Some(cd) = &geo.corner_detail {
                let full_bottom = centered_y + thumb_h + label_below_h;
                if full_bottom > cd.box_rect.top() - 6.0 {
                    let shifted_y = cd.box_rect.top() - 12.0 - label_below_h - thumb_h;
                    (x, shifted_y, ThumbnailLabelPosition::Below)
                } else {
                    (x, centered_y, ThumbnailLabelPosition::Below)
                }
            } else {
                (x, centered_y, ThumbnailLabelPosition::Below)
            }
        } else if has_cd_and_mc {
            // Landscape with CD + MC: center in gap between them
            let corner_right = occupied[0].right();
            let mat_cut_left = occupied[1].left();
            let mini_gap = 10.0;
            let avail = mat_cut_left - corner_right - 2.0 * mini_gap;
            let y = geo.frame_outer.bottom() + tm.gap;
            if avail >= thumb_w {
                let x = corner_right + mini_gap + (avail - thumb_w) / 2.0;
                (x, y, ThumbnailLabelPosition::Below)
            } else {
                let x = corner_right + mini_gap;
                (x, y, ThumbnailLabelPosition::Below)
            }
        } else {
            // Landscape: bottom-right of frame
            let x = geo.frame_outer.right() - thumb_w;
            let y = geo.frame_outer.bottom() + tm.gap;
            (x, y, ThumbnailLabelPosition::Right)
        };

        geo.thumbnail = Some(Rect::new(thumb_x, thumb_y, thumb_w, thumb_h));
        geo.thumbnail_below = thumb_y > geo.frame_outer.bottom();
        geo.thumbnail_label_position = thumb_label_pos;

        geo.annotation_bounds = AnnotationBounds {
            corner_detail_box: geo.corner_detail.as_ref().map(|cd| cd.box_rect),
            thumbnail_box: geo.thumbnail,
            thumbnail_label_position: thumb_label_pos,
            mat_cut_extent,
            ..AnnotationBounds::empty()
        };
    }

    /// Compute corner detail geometry for the inset overlay.
    /// Box size is proportional to the frame diagram so it stays visually balanced.
    /// Corner origin is at bottom-left of the box; L-shape extends RIGHT and UP.
    fn compute_corner_detail(design: &FrameDesign, geo: &Self, canvas_width: f64, style: &super::DiagramStyle) -> CornerDetailGeometry {
        // Size the box relative to canvas width — the viewBox includes callout
        // margins so frame_outer is much smaller than the visible canvas.
        // Target: box should be ~30% of canvas width for readable labels.
        // Also cap relative to rendered frame size so the box doesn't dominate a
        // small frame (e.g. PDF combined view where plan canvas height is limited).
        let target_w = canvas_width * CORNER_DETAIL_WIDTH_RATIO;
        // Use max (not min) so that extreme AR frames, where the short rendered dimension
        // is very small due to scale, don't shrink the corner detail to the minimum.
        // The canvas_width target already keeps the box proportional to the viewport;
        // frame_cap just prevents it from dominating an actually small canvas (PDF combined view).
        let frame_cap = geo.frame_outer.width.max(geo.frame_outer.height) * CORNER_DETAIL_FRAME_CAP;
        let box_w = (target_w.min(frame_cap)).clamp(CORNER_DETAIL_MIN_WIDTH, CORNER_DETAIL_MAX_WIDTH);

        let box_h = box_w / CORNER_DETAIL_ASPECT_RATIO;

        // X position: nominally extends 15% of box_w to the LEFT of frame_outer.left()
        // so the L-corner aligns with the frame corner.  When a mat is present, the mat
        // cut extension lines are vertical lines at mat_opening.right(); the corner detail
        // box must not overlap them.  Shift the box LEFTWARD (there is always left margin
        // space on the break path) until its right edge clears those lines.
        let margin = 3.0;
        let natural_box_x = geo.frame_outer.left() - box_w * CORNER_DETAIL_X_OVERHANG;
        // Basic clearance from mat opening extension lines (the post-layout
        // collision pass in svg.rs handles arrow stub clearance dynamically).
        let clearance = 4.0;
        let box_x = if let Some(mat_opening) = &geo.mat_opening {
            let natural_box_right = natural_box_x + box_w;
            let needed_box_right = mat_opening.right() - clearance;
            if natural_box_right > needed_box_right {
                let shifted_x = needed_box_right - box_w;
                shifted_x.max(style.margin)
            } else {
                natural_box_x
            }
        } else {
            natural_box_x
        };

        // Cap: box right edge should not extend past the frame's vertical centerline.
        // On wide landscape frames this is a no-op (box is much smaller than half the frame).
        // On narrow portrait frames this shifts the box left so it doesn't dominate the frame.
        let frame_center_x = geo.frame_outer.x + geo.frame_outer.width / 2.0;
        let center_pad = 6.0;
        let box_x = if box_x + box_w > frame_center_x - center_pad {
            (frame_center_x - center_pad - box_w).max(style.margin)
        } else {
            box_x
        };

        // Y position: box should overlap the bottom-left corner of the frame.
        // Standard formula: frame_outer.bottom() is always inside the box (0 < 0.85 < 1).
        // For axis-break frames, blend toward artwork center to push the box lower
        // (more bottom-anchored) when space allows. But clamp at standard_y so the box
        // never rises above the frame bottom — for tall portrait frames the blend would
        // otherwise place the box in the middle of the frame, not at the corner.
        let standard_y = geo.frame_outer.bottom() - box_h * CORNER_DETAIL_Y_OFFSET;
        let artwork_center_y = geo.artwork.y + geo.artwork.height / 2.0;
        let box_y = if geo.use_axis_break_x || geo.use_axis_break_y {
            let center_weight = CORNER_DETAIL_CENTER_WEIGHT;
            let anchor_y = artwork_center_y * center_weight + geo.frame_outer.bottom() * (1.0 - center_weight);
            (anchor_y + margin).max(standard_y)
        } else {
            standard_y
        };

        // Cap: box top should not extend above the frame's horizontal centerline.
        // On short landscape frames the box would otherwise dominate the frame height.
        // Shift box down (extending below frame) rather than shrinking it.
        let frame_center_y = geo.frame_outer.y + geo.frame_outer.height / 2.0;
        let center_pad_y = 6.0;
        let box_y = if box_y < frame_center_y + center_pad_y {
            frame_center_y + center_pad_y
        } else {
            box_y
        };

        // Detail scale: zoom out so frame band is ~21% of box width.
        // Smaller ratio = frame material drawn thinner = more room for labels,
        // and allows the box itself to be slightly smaller without clipping.
        let target_frame_band = box_w * CORNER_DETAIL_FRAME_BAND_RATIO;
        let detail_scale = target_frame_band / design.frame_material_width;

        // Corner origin X: must leave room for "Rabbet" label to the left.
        // The label chain is: text(end-anchored) ← 4px gap ← dim_line(cx-6) ← corner(cx).
        // So we need: cx - 10 - text_width("Rabbet", label_font) >= box_x + padding.
        let label_font = (box_h * CORNER_DETAIL_LABEL_FONT_RATIO).min(style.dimension_font_size * 0.75);
        let rabbet_text_w = estimate_text_width("Rabbet", label_font);
        let min_corner_x = box_x + 6.0 + rabbet_text_w + 10.0 + 4.0; // pad + text + dim_offset + gap
        let corner_x = min_corner_x.max(box_x + box_w * CORNER_DETAIL_CORNER_X_MIN);
        let corner_y = box_y + box_h * CORNER_DETAIL_CORNER_Y;

        CornerDetailGeometry {
            box_rect: Rect::new(box_x, box_y, box_w, box_h),
            corner_origin: Point::new(corner_x, corner_y),
            detail_scale,
        }
    }

    /// Choose extent points for the mat cut width dimension callout.
    ///
    /// Tries bottom-left first. If the estimated label bounding box overlaps any
    /// occupied annotation rect, falls back to bottom-right. This decouples mat cut
    /// placement from knowing specifically *what* occupies the bottom-left.
    ///
    /// Note: the SVG renderer pins extension line start to mat_opening.bottom()+3 regardless
    /// of extent_y, and dimension line is anchored to frame_outer.bottom().  Overlap with
    /// the corner detail box is handled purely by z-ordering in svg.rs (mat cut geometry
    /// renders before corner detail; labels render after).
    pub fn choose_mat_cut_extent(
        frame_inner: &Rect,
        content_area: &Rect,
        mat_opening: &Rect,
        occupied: &[Rect],
        style: &DiagramStyle,
    ) -> (Point, Point) {
        let frame_half_stroke = style.frame_stroke_width / 2.0;
        let mat_half_stroke = style.mat_stroke_width / 2.0;
        let extent_y = frame_inner.bottom() - frame_half_stroke;

        // Estimate label bounds at bottom-left position
        let mat_cut_offset = style.mat_cut_label_offset();
        let label_width = estimate_text_width("Mat Cut: 2 3/8\" (2\" visible)", style.label_font_size);
        let label_height = style.two_line_label_bounds_height();

        let bottom_left_label = Rect::new(
            content_area.left(),
            frame_inner.bottom() + mat_cut_offset - label_height / 2.0,
            label_width,
            label_height,
        );

        let use_right = occupied.iter().any(|occ| bottom_left_label.overlaps_with_margin(occ, 6.0));

        if use_right {
            // Bottom-right: from mat opening right edge to content area right edge
            (
                Point::new(mat_opening.right() - mat_half_stroke, extent_y),
                Point::new(content_area.right(), extent_y),
            )
        } else {
            // Bottom-left: from content area left edge to mat opening left edge
            (
                Point::new(content_area.left(), extent_y),
                Point::new(mat_opening.left() + mat_half_stroke, extent_y),
            )
        }
    }

    /// Estimate the bounding box of the mat cut width label given its extent points.
    /// Used to reserve space for thumbnail placement.
    fn mat_cut_label_bounds_from_extent(
        frame_outer: &Rect,
        extent_start: &Point,
        extent_end: &Point,
        style: &DiagramStyle,
    ) -> Rect {
        let mat_cut_offset = style.mat_cut_label_offset();
        // MatCutWidth is priority 2, typically level 0 on the bottom side.
        let dim_line_y = frame_outer.bottom() + style.dimension_offset_base;
        let label_width = estimate_text_width("Mat Cut: 2 3/8\" (2\" visible)", style.label_font_size);
        let label_height = style.two_line_label_bounds_height();
        // Label anchors at the leftmost x of the extent (start anchor in svg_dimension)
        let label_x = extent_start.x.min(extent_end.x);
        let label_center_y = dim_line_y + mat_cut_offset;
        Rect::new(label_x, label_center_y - label_height / 2.0, label_width, label_height)
    }

    /// Convert a dimension value (inches) to canvas units
    pub fn scale_dimension(&self, value: f64) -> f64 {
        value * self.scale
    }

    /// Calculate geometry for preview mode (no callouts)
    ///
    /// Scales to maximize use of available canvas space while maintaining
    /// correct proportions. Diagram size will change when mat is toggled
    /// (because the actual frame size changes), but ratios remain accurate.
    pub fn from_design_preview(
        design: &FrameDesign,
        canvas_width: f64,
        canvas_height: f64,
        style: &DiagramStyle,
    ) -> Self {
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();

        // Minimal margins, no callout space needed
        let available_width = canvas_width - 2.0 * style.margin;
        let available_height = canvas_height - 2.0 * style.margin;

        let scale_x = available_width / frame_outer_width;
        let scale_y = available_height / frame_outer_height;
        let scale = scale_x.min(scale_y);

        let scaled_width = frame_outer_width * scale;
        let scaled_height = frame_outer_height * scale;
        let origin_x = (canvas_width - scaled_width) / 2.0;
        let origin_y = (canvas_height - scaled_height) / 2.0;

        Self::build_rects(design, scale, origin_x, origin_y, None)
    }

    /// Get a point on the frame outer boundary
    pub fn frame_outer_point(&self, t: f64, vertical: bool) -> Point {
        if vertical {
            Point::new(self.frame_outer.left(), self.frame_outer.top() + t * self.frame_outer.height)
        } else {
            Point::new(self.frame_outer.left() + t * self.frame_outer.width, self.frame_outer.top())
        }
    }
}
