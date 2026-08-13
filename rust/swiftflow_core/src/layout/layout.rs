use crate::node::*;
use crate::types::*;

fn zero_origin(rect: SFRect) -> SFRect {
    SFRect::from_size(rect.width, rect.height)
}

pub fn alignment_fraction(alignment: SFAlignment) -> f32 {
    match alignment {
        SFAlignment::Leading => 0.0,
        SFAlignment::Center => 0.5,
        SFAlignment::Trailing => 1.0,
    }
}

fn align_offset(alignment: SFAlignment, span: f32, child: f32) -> f32 {
    ((span - child) * alignment_fraction(alignment)).max(0.0)
}

pub fn layout(node: &mut SFNode, available: SFRect) {
    match node.kind {
        SFNodeKind::Empty => {
            node.frame = SFRect::ZERO;
        }

        SFNodeKind::Rect => {
            node.frame = resolve_frame(node, available);
        }

        SFNodeKind::Spacer => {
            node.frame = available;
        }

        SFNodeKind::Text => {
            let font_size = node.font_size;
            let (measured_width, measured_height) = if !node.text.is_null() && node.text_len > 0 {
                let bytes = unsafe { std::slice::from_raw_parts(node.text, node.text_len) };
                let content = String::from_utf8_lossy(bytes);

                let scale = *crate::ffi::SCALE.lock().unwrap();
                let physical_size = font_size * scale;

                let max_width = available.width - node.padding.leading - node.padding.trailing;
                let (w, h) = crate::with_font_system(|fs| {
                    fs.measure_wrapped(
                        &content,
                        max_width,
                        physical_size,
                        node.font_weight,
                        node.font_family,
                        node.line_limit as usize,
                    )
                });
                (w, h)
            } else {
                (0.0, font_size)
            };

            node.frame = SFRect::new(
                node.frame.x + node.padding.leading,
                node.frame.y + node.padding.top,
                (measured_width + node.padding.trailing + node.padding.leading),
                (measured_height + node.padding.bottom + node.padding.top),
            );
        }

        SFNodeKind::Icon => {
            let scale = *crate::ffi::SCALE.lock().unwrap();
            let side = node.font_size * scale;
            node.frame = SFRect::new(
                node.frame.x + node.padding.leading,
                node.frame.y + node.padding.top,
                side + node.padding.leading + node.padding.trailing,
                side + node.padding.top + node.padding.bottom,
            );
        }

        SFNodeKind::Stack => {
            layout_stack(node, available);
        }

        SFNodeKind::Scroll => {
            node.frame = resolve_frame(node, available);

            if !node.children.is_null() && node.children_len > 0 {
                let children =
                    unsafe { std::slice::from_raw_parts_mut(node.children, node.children_len) };
                let content_area = node.frame.inset(node.padding);

                let axis = if node.axis == SFAxis::Horizontal {
                    SFAxis::Horizontal
                } else {
                    SFAxis::Vertical
                };

                let scrolled_area = match axis {
                    SFAxis::Horizontal => SFRect {
                        x: content_area.x - node.content_offset_x,
                        ..content_area
                    },
                    _ => SFRect {
                        y: content_area.y - node.content_offset_y,
                        ..content_area
                    },
                };

                for child in children.iter_mut() {
                    if child.kind == SFNodeKind::Stack && child.axis != SFAxis::Depth {
                        child.main_axis_alignment = SFAlignment::Leading;
                    }
                }

                layout_children_linear(
                    children,
                    scrolled_area,
                    axis,
                    node.spacing,
                    false,
                    true,
                    SFSizing::Fill,
                    SFSizing::Fill,
                    SFAlignment::Center,
                    SFAlignment::Leading,
                );

                let spacing_total = node.spacing * (children.len().saturating_sub(1)) as f32;
                let summed_width = children.iter().map(|c| c.frame.width).sum::<f32>();
                let summed_height = children.iter().map(|c| c.frame.height).sum::<f32>();
                let widest = children
                    .iter()
                    .map(|c| c.frame.width)
                    .fold(0.0_f32, f32::max);
                let tallest = children
                    .iter()
                    .map(|c| c.frame.height)
                    .fold(0.0_f32, f32::max);

                let (content_w, content_h) = match axis {
                    SFAxis::Horizontal => (summed_width + spacing_total, tallest),
                    _ => (widest, summed_height + spacing_total),
                };
                node.content_width = content_w + node.padding.leading + node.padding.trailing;
                node.content_height = content_h + node.padding.top + node.padding.bottom;
            }
        }

        SFNodeKind::Image => {
            node.frame = resolve_frame(node, available);
        }
    }
}

fn layout_stack(node: &mut SFNode, available: SFRect) {
    if node.children.is_null() || node.children_len == 0 {
        node.frame = SFRect::ZERO;
        return;
    }

    let children = unsafe { std::slice::from_raw_parts_mut(node.children, node.children_len) };

    match node.axis {
        SFAxis::Vertical | SFAxis::Horizontal => {
            let resolved = resolve_frame(node, available);
            let constraint = SFRect::new(
                available.x,
                available.y,
                axis_pick(node.sizing_x, available.width, resolved.width),
                axis_pick(node.sizing_y, available.height, resolved.height),
            );
            node.frame = constraint;
            let content_area = constraint.inset(node.padding);

            let (main_sizing, cross_sizing) = match node.axis {
                SFAxis::Vertical => (node.sizing_y, node.sizing_x),
                _ => (node.sizing_x, node.sizing_y),
            };

            layout_children_linear(
                children,
                content_area,
                node.axis,
                node.spacing,
                true,
                false,
                main_sizing,
                cross_sizing,
                node.alignment,
                node.main_axis_alignment,
            );

            if node.sizing_x == SFSizing::Hug || node.sizing_y == SFSizing::Hug {
                let hugged = hug_frame(children, node.axis, node.spacing, node.padding, available);
                node.frame = SFRect::new(
                    available.x,
                    available.y,
                    axis_pick(node.sizing_x, hugged.width, constraint.width),
                    axis_pick(node.sizing_y, hugged.height, constraint.height),
                );
            }
        }

        SFAxis::Depth => {
            let hugs_x = node.sizing_x == SFSizing::Hug;
            let hugs_y = node.sizing_y == SFSizing::Hug;

            let resolved = resolve_frame(node, available);
            let box_rect = SFRect::new(
                available.x,
                available.y,
                axis_pick(node.sizing_x, available.width, resolved.width),
                axis_pick(node.sizing_y, available.height, resolved.height),
            );
            let content_area = box_rect.inset(node.padding);

            // TODO: a stack hugging both axes still can't measure a child that fills
            // one of them. Would need to lay the child out twice, and Text/Icon add
            // their padding to the frame they already have, so a second pass doubles
            // it. Make those idempotent first.
            let defers = |c: &SFNode| {
                (hugs_x && c.sizing_x == SFSizing::Fill) || (hugs_y && c.sizing_y == SFSizing::Fill)
            };

            for child in children.iter_mut() {
                if !defers(child) {
                    layout(child, zero_origin(content_area));
                }
            }

            let content_w = children
                .iter()
                .filter(|c| !defers(c) && c.sizing_x != SFSizing::Fill)
                .map(|c| c.frame.width)
                .fold(0.0_f32, f32::max);
            let content_h = children
                .iter()
                .filter(|c| !defers(c) && c.sizing_y != SFSizing::Fill)
                .map(|c| c.frame.height)
                .fold(0.0_f32, f32::max);

            node.frame = SFRect::new(
                available.x,
                available.y,
                if hugs_x {
                    content_w + node.padding.leading + node.padding.trailing
                } else {
                    box_rect.width
                },
                if hugs_y {
                    content_h + node.padding.top + node.padding.bottom
                } else {
                    box_rect.height
                },
            );
            let content_area = node.frame.inset(node.padding);

            for child in children.iter_mut() {
                if defers(child) {
                    layout(child, zero_origin(content_area));
                }
            }

            for child in children.iter_mut() {
                child.frame.x = content_area.x
                    + align_offset(node.alignment, content_area.width, child.frame.width);
                child.frame.y = content_area.y
                    + align_offset(
                        node.vertical_alignment,
                        content_area.height,
                        child.frame.height,
                    );
            }
        }
    }
}

fn axis_pick(sizing: SFSizing, offered: f32, resolved: f32) -> f32 {
    if sizing == SFSizing::Hug {
        offered
    } else {
        resolved
    }
}

fn layout_children_linear(
    children: &mut [SFNode],
    available: SFRect,
    axis: SFAxis,
    spacing: f32,
    center_main_axis: bool,
    is_scroll_content: bool,

    main_sizing: SFSizing,
    cross_sizing: SFSizing,
    alignment: SFAlignment,
    main_axis_alignment: SFAlignment,
) {
    let distributes_weight = main_sizing != SFSizing::Hug;
    let flex_weight = |c: &SFNode| -> f32 {
        if c.kind == SFNodeKind::Spacer {
            c.weight.max(1.0)
        } else if distributes_weight {
            c.weight.max(0.0)
        } else {
            0.0
        }
    };
    let total_weight: f32 = children.iter().map(&flex_weight).sum();

    let total_spacing = spacing * (children.len().saturating_sub(1)) as f32;
    let mut used = total_spacing;

    for child in children.iter_mut() {
        if flex_weight(child) > 0.0 {
            continue;
        }
        let child_available = match axis {
            SFAxis::Vertical => SFRect::from_size(available.width, available.height),
            SFAxis::Horizontal => SFRect::from_size(available.width, available.height),
            SFAxis::Depth => unreachable!(),
        };
        layout(child, child_available);

        if is_scroll_content {
            match axis {
                SFAxis::Vertical => child.frame.height = natural_extent(child, axis),
                SFAxis::Horizontal => child.frame.width = natural_extent(child, axis),
                SFAxis::Depth => unreachable!(),
            }
        }

        used += match axis {
            SFAxis::Vertical => child.frame.height,
            SFAxis::Horizontal => child.frame.width,
            SFAxis::Depth => unreachable!(),
        };
    }

    let remaining = match axis {
        SFAxis::Vertical => (available.height - used).max(0.0),
        SFAxis::Horizontal => (available.width - used).max(0.0),
        SFAxis::Depth => unreachable!(),
    };
    let share_of = |c: &SFNode| -> f32 {
        let weight = flex_weight(c);
        if weight <= 0.0 || total_weight <= 0.0 {
            0.0
        } else {
            remaining * weight / total_weight
        }
    };

    for child in children.iter_mut() {
        if child.kind == SFNodeKind::Spacer || flex_weight(child) <= 0.0 {
            continue;
        }
        let share = share_of(child);
        // Asking for a share of the leftover and then not filling it is never
        // what the caller meant. Without this the box is widened to `share`
        // below while the content inside stays at its natural size.
        match axis {
            SFAxis::Vertical => child.sizing_y = SFSizing::Fill,
            SFAxis::Horizontal => child.sizing_x = SFSizing::Fill,
            SFAxis::Depth => unreachable!(),
        }
        let child_available = match axis {
            SFAxis::Vertical => SFRect::from_size(available.width, share),
            SFAxis::Horizontal => SFRect::from_size(share, available.height),
            SFAxis::Depth => unreachable!(),
        };
        layout(child, child_available);
        match axis {
            SFAxis::Vertical => child.frame.height = share,
            SFAxis::Horizontal => child.frame.width = share,
            SFAxis::Depth => unreachable!(),
        }
    }

    let mut cursor = match axis {
        SFAxis::Vertical => available.y,
        SFAxis::Horizontal => available.x,
        SFAxis::Depth => unreachable!(),
    };

    for child in children.iter_mut() {
        if child.kind == SFNodeKind::Spacer {
            let size = share_of(child).max(child.min_length);
            child.frame = match axis {
                SFAxis::Vertical => SFRect::new(available.x, cursor, available.width, size),
                SFAxis::Horizontal => SFRect::new(cursor, available.y, size, available.height),
                SFAxis::Depth => unreachable!(),
            };
            cursor += size + spacing;
        } else {
            match axis {
                SFAxis::Vertical => {
                    child.frame.x = available.x;
                    child.frame.y = cursor;
                    cursor += child.frame.height + spacing;
                }
                SFAxis::Horizontal => {
                    child.frame.x = cursor;
                    child.frame.y = available.y;
                    cursor += child.frame.width + spacing;
                }
                SFAxis::Depth => unreachable!(),
            }
        }
    }

    if center_main_axis && total_weight <= 0.0 && main_sizing != SFSizing::Hug {
        let (total, room) = match axis {
            SFAxis::Vertical => (
                children.iter().map(|c| c.frame.height).sum::<f32>()
                    + spacing * (children.len().saturating_sub(1)) as f32,
                available.height,
            ),
            SFAxis::Horizontal => (
                children.iter().map(|c| c.frame.width).sum::<f32>()
                    + spacing * (children.len().saturating_sub(1)) as f32,
                available.width,
            ),
            SFAxis::Depth => unreachable!(),
        };
        let offset = align_offset(main_axis_alignment, room, total);
        if offset > 0.0 {
            for child in children.iter_mut() {
                match axis {
                    SFAxis::Vertical => child.frame.y += offset,
                    SFAxis::Horizontal => child.frame.x += offset,
                    SFAxis::Depth => unreachable!(),
                }
            }
        }
    }

    match axis {
        SFAxis::Vertical => {
            let cross_size = if cross_sizing == SFSizing::Hug {
                children
                    .iter()
                    .map(|c| c.frame.width)
                    .fold(0.0_f32, f32::max)
            } else {
                available.width
            };
            for child in children.iter_mut() {
                child.frame.x =
                    available.x + align_offset(alignment, cross_size, child.frame.width);
            }
        }
        SFAxis::Horizontal => {
            let cross_size = if cross_sizing == SFSizing::Hug {
                children
                    .iter()
                    .map(|c| c.frame.height)
                    .fold(0.0_f32, f32::max)
            } else {
                available.height
            };
            for child in children.iter_mut() {
                child.frame.y =
                    available.y + align_offset(alignment, cross_size, child.frame.height);
            }
        }
        SFAxis::Depth => {}
    }
}

fn natural_extent(node: &SFNode, axis: SFAxis) -> f32 {
    let own = match axis {
        SFAxis::Horizontal => node.frame.width,
        _ => node.frame.height,
    };

    if node.sizing_on(axis) == SFSizing::Fixed {
        return match axis {
            SFAxis::Horizontal => node.fixed_width,
            _ => node.fixed_height,
        };
    }
    if node.kind != SFNodeKind::Stack || node.children.is_null() || node.children_len == 0 {
        return own;
    }
    let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };
    let (lead, trail) = match axis {
        SFAxis::Horizontal => (node.padding.leading, node.padding.trailing),
        _ => (node.padding.top, node.padding.bottom),
    };
    let inner = if node.axis == axis {
        children
            .iter()
            .map(|c| natural_extent(c, axis))
            .sum::<f32>()
            + node.spacing * (children.len().saturating_sub(1)) as f32
    } else {
        children
            .iter()
            .map(|c| natural_extent(c, axis))
            .fold(0.0_f32, f32::max)
    };
    inner + lead + trail
}

fn resolve_axis(sizing: SFSizing, fixed: f32, available: f32) -> f32 {
    match sizing {
        SFSizing::Fixed => fixed,
        SFSizing::Fill | SFSizing::Hug => available,
    }
}

fn resolve_frame(node: &SFNode, available: SFRect) -> SFRect {
    SFRect::new(
        available.x,
        available.y,
        resolve_axis(node.sizing_x, node.fixed_width, available.width),
        resolve_axis(node.sizing_y, node.fixed_height, available.height),
    )
}

fn hug_frame(
    children: &[SFNode],
    axis: SFAxis,
    spacing: f32,
    padding: SFEdgeInsets,
    available: SFRect,
) -> SFRect {
    let gaps = spacing * (children.len().saturating_sub(1)) as f32;

    let (width, height) = match axis {
        SFAxis::Vertical => {
            let w = children
                .iter()
                .map(|c| c.frame.width)
                .fold(0.0_f32, f32::max);
            let h = children.iter().map(|c| c.frame.height).sum::<f32>() + gaps;
            (
                w + padding.leading + padding.trailing,
                h + padding.top + padding.bottom,
            )
        }
        SFAxis::Horizontal => {
            let w = children.iter().map(|c| c.frame.width).sum::<f32>() + gaps;
            let h = children
                .iter()
                .map(|c| c.frame.height)
                .fold(0.0_f32, f32::max);
            (
                w + padding.leading + padding.trailing,
                h + padding.top + padding.bottom,
            )
        }

        SFAxis::Depth => {
            let w = children
                .iter()
                .map(|c| c.frame.width)
                .fold(0.0_f32, f32::max);
            let h = children
                .iter()
                .map(|c| c.frame.height)
                .fold(0.0_f32, f32::max);
            (
                w + padding.leading + padding.trailing,
                h + padding.top + padding.bottom,
            )
        }
    };

    SFRect::new(available.x, available.y, width, height)
}
