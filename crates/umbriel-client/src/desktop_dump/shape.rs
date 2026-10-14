//! Geometry-derived shape, deliberately distinct from compositor layout topology.
use std::collections::BTreeMap;

use serde::Serialize;
use umbriel_protocol::model::{LayoutMode, Window, Windows, Workspaces};

#[derive(Serialize)]
pub struct DesktopShape {
    basis: &'static str,
    exact_layout_tree: bool,
    window_reference: &'static str,
    notes: &'static [&'static str],
    workspaces: Vec<WorkspaceShape>,
    scratchpads: BTreeMap<String, Vec<usize>>,
    unassigned_windows: Vec<UnassignedWindow>,
}

#[derive(Serialize)]
struct WorkspaceShape {
    workspace_index: usize,
    workspace_id: String,
    name: String,
    output: String,
    layout_mode: LayoutMode,
    tiled: Region,
    floating_window_indices: Vec<usize>,
    invalid_tiled_window_indices: Vec<usize>,
}

#[derive(Serialize)]
struct UnassignedWindow {
    window_index: usize,
    reason: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct Rect {
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

#[derive(Serialize)]
struct Region {
    bounds: Option<Rect>,
    #[serde(flatten)]
    contents: Contents,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Contents {
    Empty,
    Window {
        window_index: usize,
        window_id: String,
    },
    SideBySide {
        regions: Vec<Region>,
    },
    Stacked {
        regions: Vec<Region>,
    },
    Unresolved {
        window_indices: Vec<usize>,
        reason: &'static str,
    },
}

#[derive(Clone, Copy)]
struct Item<'a> {
    index: usize,
    window: &'a Window,
    rect: Rect,
}

pub fn derive(windows: &Windows, workspaces: &Workspaces) -> DesktopShape {
    let mut membership: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, workspace) in workspaces.iter().enumerate() {
        if !workspace.id.as_str().is_empty() {
            membership
                .entry(workspace.id.as_str())
                .or_default()
                .push(index);
        }
    }
    let mut grouped = vec![Vec::new(); workspaces.len()];
    let mut scratchpads: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut unassigned_windows = Vec::new();
    for (index, window) in windows.iter().enumerate() {
        let id = window.workspace.as_str();
        if id.is_empty() && !window.scratchpad.is_empty() {
            scratchpads
                .entry(window.scratchpad.clone())
                .or_default()
                .push(index);
            continue;
        }
        let reason = match membership.get(id).map(Vec::as_slice) {
            Some([workspace]) => {
                grouped[*workspace].push(index);
                continue;
            }
            Some(_) => "ambiguous_workspace_id",
            None if id.is_empty() => "no_workspace_or_named_scratchpad",
            None => "workspace_not_in_snapshot",
        };
        unassigned_windows.push(UnassignedWindow {
            window_index: index,
            reason,
        });
    }
    let workspaces = workspaces
        .iter()
        .enumerate()
        .map(|(workspace_index, workspace)| {
            let mut floating_window_indices = Vec::new();
            let mut invalid_tiled_window_indices = Vec::new();
            let mut tiled = Vec::new();
            for &index in &grouped[workspace_index] {
                let window = &windows[index];
                if window.floating {
                    floating_window_indices.push(index);
                } else if window.w <= 0 || window.h <= 0 {
                    invalid_tiled_window_indices.push(index);
                } else {
                    tiled.push(Item {
                        index,
                        window,
                        rect: Rect {
                            x: i64::from(window.x),
                            y: i64::from(window.y),
                            width: i64::from(window.w),
                            height: i64::from(window.h),
                        },
                    });
                }
            }
            WorkspaceShape {
                workspace_index,
                workspace_id: workspace.id.to_string(),
                name: workspace.name.clone(),
                output: workspace.output.clone(),
                layout_mode: workspace.layout.clone(),
                tiled: partition(tiled, 0),
                floating_window_indices,
                invalid_tiled_window_indices,
            }
        })
        .collect();
    DesktopShape {
        basis: "window_geometry",
        exact_layout_tree: false,
        window_reference: "zero-based index into queries.windows.data",
        notes: &[
            "Workspace membership, output, layout mode, and floating flags come from IPC; spatial regions are derived.",
            "Groups follow separating lines between reported rectangles, preferring side-by-side over stacked when both are possible; multiple internal trees can yield the same shape.",
            "Origins are layout targets while sizes are committed content geometry; fullscreen, resizing, animation, and snapshot races can obscure the underlying structure.",
            "Bounds cover reported windows, including off-screen windows, not the output viewport; they do not reveal split ratios, column weights, master roles, or scroll state.",
            "Floating, scratchpad, invalid-geometry, and unresolved-membership windows remain separate; source indices preserve even empty or duplicate window IDs.",
        ],
        workspaces,
        scratchpads,
        unassigned_windows,
    }
}

fn bounds(items: &[Item<'_>]) -> Option<Rect> {
    let first = items.first()?.rect;
    let (mut left, mut top) = (first.x, first.y);
    let (mut right, mut bottom) = (first.x + first.width, first.y + first.height);
    for item in &items[1..] {
        left = left.min(item.rect.x);
        top = top.min(item.rect.y);
        right = right.max(item.rect.x + item.rect.width);
        bottom = bottom.max(item.rect.y + item.rect.height);
    }
    Some(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

/// Maximal clusters of overlapping intervals. Every gap is a valid separator;
/// touching intervals are also separable. No tolerance or guessed ratio is used.
fn groups<'a>(items: &[Item<'a>], horizontal: bool) -> Vec<Vec<Item<'a>>> {
    let interval = |item: &Item<'_>| {
        if horizontal {
            (item.rect.x, item.rect.x + item.rect.width)
        } else {
            (item.rect.y, item.rect.y + item.rect.height)
        }
    };
    let mut sorted = items.to_vec();
    sorted.sort_by_key(|item| (interval(item).0, item.rect.y, item.rect.x, item.index));
    let mut groups: Vec<Vec<Item<'a>>> = Vec::new();
    let mut end = i64::MIN;
    for item in sorted {
        let (start, stop) = interval(&item);
        if groups.is_empty() || start >= end {
            groups.push(Vec::new());
            end = stop;
        } else {
            end = end.max(stop);
        }
        groups.last_mut().expect("a group was created").push(item);
    }
    groups
}

fn partition(items: Vec<Item<'_>>, depth: usize) -> Region {
    let bounds = bounds(&items);
    let contents = if items.is_empty() {
        Contents::Empty
    } else if items.len() == 1 {
        Contents::Window {
            window_index: items[0].index,
            window_id: items[0].window.id.to_string(),
        }
    } else if depth < 32 {
        let columns = groups(&items, true);
        if columns.len() > 1 {
            Contents::SideBySide {
                regions: columns
                    .into_iter()
                    .map(|items| partition(items, depth + 1))
                    .collect(),
            }
        } else {
            let rows = groups(&items, false);
            if rows.len() > 1 {
                Contents::Stacked {
                    regions: rows
                        .into_iter()
                        .map(|items| partition(items, depth + 1))
                        .collect(),
                }
            } else {
                unresolved(&items, "overlapping_or_not_axis_separable")
            }
        }
    } else {
        // Bound recursion even for adversarial long alternating staircase input.
        unresolved(&items, "inference_depth_limit")
    };
    Region { bounds, contents }
}

fn unresolved(items: &[Item<'_>], reason: &'static str) -> Contents {
    let mut window_indices: Vec<_> = items.iter().map(|item| item.index).collect();
    window_indices.sort_unstable();
    Contents::Unresolved {
        window_indices,
        reason,
    }
}
