#[path = "../src/desktop_dump/shape.rs"]
mod shape;

use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};
use umbriel_protocol::{
    codec,
    model::{LayoutMode, Window, Windows, Workspace, Workspaces},
};

fn window(index: usize, rect: [i32; 4]) -> Window {
    let mut window = codec::decode::<Windows>(include_bytes!(
        "../../umbriel-protocol/tests/fixtures/models/windows.json"
    ))
    .unwrap()
    .remove(0);
    window.id = format!("w{index}").into();
    window.workspace = "ws".into();
    window.scratchpad.clear();
    window.floating = false;
    [window.x, window.y, window.w, window.h] = rect;
    window
}

fn workspace(mode: &str) -> Workspace {
    let mut workspace = codec::decode::<Workspaces>(include_bytes!(
        "../../umbriel-protocol/tests/fixtures/models/workspaces.json"
    ))
    .unwrap()
    .remove(0);
    workspace.id = "ws".into();
    workspace.layout = LayoutMode::from(mode);
    workspace
}

fn derive(windows: Windows, workspaces: Workspaces) -> Value {
    sonic_rs::to_value(&shape::derive(&windows, &workspaces)).unwrap()
}

#[test]
fn scrolling_columns_include_stacked_rows_and_offscreen_windows() {
    let value = derive(
        vec![
            window(0, [120, 60, 90, 40]),
            window(1, [-100, 0, 90, 100]),
            window(2, [120, 0, 90, 50]),
            window(3, [0, 0, 110, 100]),
        ],
        vec![workspace("scrolling")],
    );
    assert_eq!(value["basis"].as_str(), Some("window_geometry"));
    assert_eq!(value["exact_layout_tree"].as_bool(), Some(false));
    let tiled = &value["workspaces"][0]["tiled"];
    assert_eq!(tiled["kind"].as_str(), Some("side_by_side"));
    let columns = tiled["regions"].as_array().unwrap();
    assert_eq!(columns.len(), 3);
    assert_eq!(columns[0]["window_index"].as_u64(), Some(1));
    assert_eq!(columns[1]["window_index"].as_u64(), Some(3));
    assert_eq!(columns[2]["kind"].as_str(), Some("stacked"));
    assert_eq!(columns[2]["regions"][0]["window_index"].as_u64(), Some(2));
    assert_eq!(columns[2]["regions"][1]["window_index"].as_u64(), Some(0));
    assert_eq!(tiled["bounds"]["x"].as_i64(), Some(-100));
    assert_eq!(tiled["bounds"]["width"].as_i64(), Some(310));
}

#[test]
fn nested_spatial_regions_do_not_claim_to_be_a_dwindle_tree() {
    let value = derive(
        vec![
            window(0, [0, 0, 200, 100]),
            window(1, [0, 110, 90, 100]),
            window(2, [100, 110, 100, 100]),
        ],
        vec![workspace("dwindle")],
    );
    let tiled = &value["workspaces"][0]["tiled"];
    assert_eq!(tiled["kind"].as_str(), Some("stacked"));
    assert_eq!(tiled["regions"][1]["kind"].as_str(), Some("side_by_side"));
    assert_eq!(
        value["workspaces"][0]["layout_mode"].as_str(),
        Some("dwindle")
    );
    assert!(tiled.get("ratio").is_none());
}

#[test]
fn master_regions_and_unknown_modes_are_spatial_not_invented_roles() {
    for mode in ["master", "future-layout"] {
        let value = derive(
            vec![
                window(0, [0, 0, 100, 200]),
                window(1, [110, 0, 50, 90]),
                window(2, [110, 100, 50, 100]),
            ],
            vec![workspace(mode)],
        );
        assert_eq!(value["workspaces"][0]["layout_mode"].as_str(), Some(mode));
        let tiled = &value["workspaces"][0]["tiled"];
        assert_eq!(tiled["kind"].as_str(), Some("side_by_side"));
        assert_eq!(tiled["regions"][1]["kind"].as_str(), Some("stacked"));
        assert!(tiled.get("master").is_none());
    }
}

#[test]
fn overlapping_rectangles_remain_unresolved() {
    let value = derive(
        vec![window(0, [0, 0, 100, 100]), window(1, [50, 50, 100, 100])],
        vec![workspace("dwindle")],
    );
    let tiled = &value["workspaces"][0]["tiled"];
    assert_eq!(tiled["kind"].as_str(), Some("unresolved"));
    assert_eq!(
        tiled["reason"].as_str(),
        Some("overlapping_or_not_axis_separable")
    );
    assert_eq!(tiled["window_indices"], sonic_rs::json!([0, 1]));
}

#[test]
fn touching_rectangles_are_separable_and_empty_ids_keep_source_indices() {
    let mut a = window(0, [0, 0, 10, 10]);
    let mut b = window(1, [10, 0, 10, 10]);
    a.id = "".into();
    b.id = "".into();
    let value = derive(vec![a, b], vec![workspace("scrolling")]);
    let regions = &value["workspaces"][0]["tiled"]["regions"];
    for index in [0, 1] {
        assert_eq!(regions[index]["window_index"].as_u64(), Some(index as u64));
        assert_eq!(regions[index]["window_id"].as_str(), Some(""));
    }
}

#[test]
fn floating_scratchpad_invalid_and_missing_workspace_windows_are_not_lost() {
    let mut windows: Vec<_> = (0..6).map(|i| window(i, [0, 0, 10, 10])).collect();
    windows[1].floating = true;
    windows[2].workspace = "".into();
    windows[2].scratchpad = "notes".into();
    windows[3].workspace = "gone".into();
    windows[4].h = 0;
    windows[5].workspace = "".into();
    let value = derive(windows, vec![workspace("scrolling")]);
    let workspace = &value["workspaces"][0];
    assert_eq!(workspace["tiled"]["window_index"].as_u64(), Some(0));
    assert_eq!(workspace["floating_window_indices"], sonic_rs::json!([1]));
    assert_eq!(
        workspace["invalid_tiled_window_indices"],
        sonic_rs::json!([4])
    );
    assert_eq!(value["scratchpads"]["notes"], sonic_rs::json!([2]));
    assert_eq!(
        value["unassigned_windows"][0]["window_index"].as_u64(),
        Some(3)
    );
    assert_eq!(
        value["unassigned_windows"][0]["reason"].as_str(),
        Some("workspace_not_in_snapshot")
    );
    assert_eq!(
        value["unassigned_windows"][1]["window_index"].as_u64(),
        Some(5)
    );
}

#[test]
fn duplicate_workspace_ids_do_not_duplicate_or_arbitrarily_assign_windows() {
    let value = derive(
        vec![window(0, [0, 0, 10, 10])],
        vec![workspace("master"), workspace("scrolling")],
    );
    for index in [0, 1] {
        assert_eq!(
            value["workspaces"][index]["tiled"]["kind"].as_str(),
            Some("empty")
        );
    }
    assert_eq!(
        value["unassigned_windows"][0]["reason"].as_str(),
        Some("ambiguous_workspace_id")
    );
}

#[test]
fn empty_workspaces_and_extreme_i32_geometry_remain_representable() {
    let empty = derive(vec![], vec![workspace("scrolling")]);
    assert_eq!(
        empty["workspaces"][0]["tiled"]["kind"].as_str(),
        Some("empty")
    );
    assert!(empty["workspaces"][0]["tiled"]["bounds"].is_null());
    let value = derive(
        vec![
            window(0, [i32::MIN, 0, i32::MAX, 10]),
            window(1, [i32::MAX, 0, i32::MAX, 10]),
        ],
        vec![workspace("scrolling")],
    );
    assert_eq!(
        value["workspaces"][0]["tiled"]["bounds"]["width"].as_i64(),
        Some(6_442_450_942)
    );
}

fn collect_indices(region: &Value, indices: &mut Vec<u64>) -> usize {
    match region["kind"].as_str().unwrap() {
        "window" => {
            indices.push(region["window_index"].as_u64().unwrap());
            0
        }
        "unresolved" => {
            indices.extend(
                region["window_indices"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap()),
            );
            0
        }
        "empty" => 0,
        _ => region["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|child| collect_indices(child, indices) + 1)
            .max()
            .unwrap(),
    }
}

#[test]
fn deeply_nested_geometry_is_bounded_without_dropping_windows() {
    let (mut x, mut y, mut w, mut h) = (0, 0, 100, 100);
    let mut windows = Vec::new();
    for index in 0..40 {
        if index % 2 == 0 {
            windows.push(window(index, [x, y, 1, h]));
            x += 2;
            w -= 2;
        } else {
            windows.push(window(index, [x, y, w, 1]));
            y += 2;
            h -= 2;
        }
    }
    windows.push(window(40, [x, y, w, h]));
    let value = derive(windows, vec![workspace("dwindle")]);
    let mut indices = Vec::new();
    let depth = collect_indices(&value["workspaces"][0]["tiled"], &mut indices);
    assert_eq!(depth, 32);
    indices.sort_unstable();
    assert_eq!(indices, (0..41).collect::<Vec<_>>());
}
