//! Declarative name/category inventory covering every action name the
//! compositor's keybind parser accepts. Typed builders land family by family;
//! a constant may exist before its family module does, so catalog presence
//! fixes the shared spelling and category but is not itself a typed
//! constructor. The inventory has never gated raw actions.
//!
//! Constants are named by the wire name uppercased with hyphens replaced by
//! underscores, so family modules can reference them independently.

/// Upstream argument categories. Individual families own their validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Category {
    None,
    Command,
    Fraction,
    FractionDelta,
    Workspace,
    OptionalOutput,
    OptionalScratchpad,
    WindowId,
    OptionalWindowId,
    SkipConfirmation,
    LayoutMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    pub name: &'static str,
    pub category: Category,
}

pub const CHEATSHEET_CLOSE: Spec = Spec {
    name: "cheatsheet-close",
    category: Category::None,
};

pub const CHEATSHEET_OPEN: Spec = Spec {
    name: "cheatsheet-open",
    category: Category::None,
};

pub const CHEATSHEET_TOGGLE: Spec = Spec {
    name: "cheatsheet-toggle",
    category: Category::None,
};

pub const COLUMN_CENTER: Spec = Spec {
    name: "column-center",
    category: Category::None,
};

pub const COLUMN_FOCUS_FIRST: Spec = Spec {
    name: "column-focus-first",
    category: Category::None,
};

pub const COLUMN_FOCUS_LAST: Spec = Spec {
    name: "column-focus-last",
    category: Category::None,
};

pub const COLUMN_MOVE_LEFT: Spec = Spec {
    name: "column-move-left",
    category: Category::None,
};

pub const COLUMN_MOVE_RIGHT: Spec = Spec {
    name: "column-move-right",
    category: Category::None,
};

pub const COLUMN_MOVE_TO_FIRST: Spec = Spec {
    name: "column-move-to-first",
    category: Category::None,
};

pub const COLUMN_MOVE_TO_LAST: Spec = Spec {
    name: "column-move-to-last",
    category: Category::None,
};

pub const COLUMN_MOVE_TO_OUTPUT_DOWN: Spec = Spec {
    name: "column-move-to-output-down",
    category: Category::None,
};

pub const COLUMN_MOVE_TO_OUTPUT_LEFT: Spec = Spec {
    name: "column-move-to-output-left",
    category: Category::None,
};

pub const COLUMN_MOVE_TO_OUTPUT_RIGHT: Spec = Spec {
    name: "column-move-to-output-right",
    category: Category::None,
};

pub const COLUMN_MOVE_TO_OUTPUT_UP: Spec = Spec {
    name: "column-move-to-output-up",
    category: Category::None,
};

pub const COLUMN_MOVE_TO_WORKSPACE: Spec = Spec {
    name: "column-move-to-workspace",
    category: Category::Workspace,
};

pub const COLUMN_MOVE_TO_WORKSPACE_NEXT: Spec = Spec {
    name: "column-move-to-workspace-next",
    category: Category::None,
};

pub const COLUMN_MOVE_TO_WORKSPACE_PREVIOUS: Spec = Spec {
    name: "column-move-to-workspace-previous",
    category: Category::None,
};

pub const CONFIG_RELOAD: Spec = Spec {
    name: "config-reload",
    category: Category::None,
};

pub const DPMS_OFF: Spec = Spec {
    name: "dpms-off",
    category: Category::OptionalOutput,
};

pub const DPMS_ON: Spec = Spec {
    name: "dpms-on",
    category: Category::OptionalOutput,
};

pub const KEYBOARD_LAYOUT_NEXT: Spec = Spec {
    name: "keyboard-layout-next",
    category: Category::None,
};

pub const LAYOUT_MASTER_COUNT_DECREASE: Spec = Spec {
    name: "layout-master-count-decrease",
    category: Category::None,
};

pub const LAYOUT_MASTER_COUNT_INCREASE: Spec = Spec {
    name: "layout-master-count-increase",
    category: Category::None,
};

pub const LAYOUT_SCROLL_DOWN: Spec = Spec {
    name: "layout-scroll-down",
    category: Category::None,
};

pub const LAYOUT_SCROLL_DRAG: Spec = Spec {
    name: "layout-scroll-drag",
    category: Category::None,
};

pub const LAYOUT_SCROLL_LEFT: Spec = Spec {
    name: "layout-scroll-left",
    category: Category::None,
};

pub const LAYOUT_SCROLL_RIGHT: Spec = Spec {
    name: "layout-scroll-right",
    category: Category::None,
};

pub const LAYOUT_SCROLL_UP: Spec = Spec {
    name: "layout-scroll-up",
    category: Category::None,
};

pub const OUTPUT_FOCUS_DOWN: Spec = Spec {
    name: "output-focus-down",
    category: Category::None,
};

pub const OUTPUT_FOCUS_LEFT: Spec = Spec {
    name: "output-focus-left",
    category: Category::None,
};

pub const OUTPUT_FOCUS_NEXT: Spec = Spec {
    name: "output-focus-next",
    category: Category::None,
};

pub const OUTPUT_FOCUS_PREVIOUS: Spec = Spec {
    name: "output-focus-previous",
    category: Category::None,
};

pub const OUTPUT_FOCUS_RIGHT: Spec = Spec {
    name: "output-focus-right",
    category: Category::None,
};

pub const OUTPUT_FOCUS_UP: Spec = Spec {
    name: "output-focus-up",
    category: Category::None,
};

pub const OVERVIEW_CLOSE: Spec = Spec {
    name: "overview-close",
    category: Category::None,
};

pub const OVERVIEW_OPEN: Spec = Spec {
    name: "overview-open",
    category: Category::None,
};

pub const OVERVIEW_TOGGLE: Spec = Spec {
    name: "overview-toggle",
    category: Category::None,
};

pub const SCRATCHPAD_FOCUS_NEXT: Spec = Spec {
    name: "scratchpad-focus-next",
    category: Category::OptionalScratchpad,
};

pub const SCRATCHPAD_TOGGLE: Spec = Spec {
    name: "scratchpad-toggle",
    category: Category::OptionalScratchpad,
};

pub const SESSION_QUIT: Spec = Spec {
    name: "session-quit",
    category: Category::SkipConfirmation,
};

pub const SHORTCUTS_INHIBIT_TOGGLE: Spec = Spec {
    name: "shortcuts-inhibit-toggle",
    category: Category::None,
};

pub const SPAWN: Spec = Spec {
    name: "spawn",
    category: Category::Command,
};

pub const SUBMAP: Spec = Spec {
    name: "submap",
    category: Category::Command,
};

pub const WINDOW_CENTER: Spec = Spec {
    name: "window-center",
    category: Category::None,
};

pub const WINDOW_CLOSE: Spec = Spec {
    name: "window-close",
    category: Category::OptionalWindowId,
};

pub const WINDOW_CONSUME_LEFT: Spec = Spec {
    name: "window-consume-left",
    category: Category::None,
};

pub const WINDOW_CONSUME_OR_EXPEL_LEFT: Spec = Spec {
    name: "window-consume-or-expel-left",
    category: Category::None,
};

pub const WINDOW_CONSUME_OR_EXPEL_RIGHT: Spec = Spec {
    name: "window-consume-or-expel-right",
    category: Category::None,
};

pub const WINDOW_CONSUME_RIGHT: Spec = Spec {
    name: "window-consume-right",
    category: Category::None,
};

pub const WINDOW_CYCLE_PRIMARY_EXTENT: Spec = Spec {
    name: "window-cycle-primary-extent",
    category: Category::None,
};

pub const WINDOW_CYCLE_PRIMARY_EXTENT_BACK: Spec = Spec {
    name: "window-cycle-primary-extent-back",
    category: Category::None,
};

pub const WINDOW_CYCLE_SECONDARY_EXTENT: Spec = Spec {
    name: "window-cycle-secondary-extent",
    category: Category::None,
};

pub const WINDOW_CYCLE_SECONDARY_EXTENT_BACK: Spec = Spec {
    name: "window-cycle-secondary-extent-back",
    category: Category::None,
};

pub const WINDOW_FOCUS: Spec = Spec {
    name: "window-focus",
    category: Category::WindowId,
};

pub const WINDOW_FOCUS_DOWN: Spec = Spec {
    name: "window-focus-down",
    category: Category::None,
};

pub const WINDOW_FOCUS_LAST: Spec = Spec {
    name: "window-focus-last",
    category: Category::None,
};

pub const WINDOW_FOCUS_LEFT: Spec = Spec {
    name: "window-focus-left",
    category: Category::None,
};

pub const WINDOW_FOCUS_NEXT: Spec = Spec {
    name: "window-focus-next",
    category: Category::None,
};

pub const WINDOW_FOCUS_OR_OUTPUT_DOWN: Spec = Spec {
    name: "window-focus-or-output-down",
    category: Category::None,
};

pub const WINDOW_FOCUS_OR_OUTPUT_LEFT: Spec = Spec {
    name: "window-focus-or-output-left",
    category: Category::None,
};

pub const WINDOW_FOCUS_OR_OUTPUT_RIGHT: Spec = Spec {
    name: "window-focus-or-output-right",
    category: Category::None,
};

pub const WINDOW_FOCUS_OR_OUTPUT_UP: Spec = Spec {
    name: "window-focus-or-output-up",
    category: Category::None,
};

pub const WINDOW_FOCUS_OR_WORKSPACE_DOWN: Spec = Spec {
    name: "window-focus-or-workspace-down",
    category: Category::None,
};

pub const WINDOW_FOCUS_OR_WORKSPACE_UP: Spec = Spec {
    name: "window-focus-or-workspace-up",
    category: Category::None,
};

pub const WINDOW_FOCUS_PREVIOUS: Spec = Spec {
    name: "window-focus-previous",
    category: Category::None,
};

pub const WINDOW_FOCUS_RIGHT: Spec = Spec {
    name: "window-focus-right",
    category: Category::None,
};

pub const WINDOW_FOCUS_SWITCH_FLOATING: Spec = Spec {
    name: "window-focus-switch-floating",
    category: Category::None,
};

pub const WINDOW_FOCUS_UP: Spec = Spec {
    name: "window-focus-up",
    category: Category::None,
};

pub const WINDOW_FOCUS_WARP: Spec = Spec {
    name: "window-focus-warp",
    category: Category::WindowId,
};

pub const WINDOW_MODIFY_HEIGHT_DOWN: Spec = Spec {
    name: "window-modify-height-down",
    category: Category::FractionDelta,
};

pub const WINDOW_MODIFY_HEIGHT_UP: Spec = Spec {
    name: "window-modify-height-up",
    category: Category::FractionDelta,
};

pub const WINDOW_MODIFY_PRIMARY_EXTENT: Spec = Spec {
    name: "window-modify-primary-extent",
    category: Category::FractionDelta,
};

pub const WINDOW_MODIFY_SECONDARY_EXTENT: Spec = Spec {
    name: "window-modify-secondary-extent",
    category: Category::FractionDelta,
};

pub const WINDOW_MODIFY_WIDTH_LEFT: Spec = Spec {
    name: "window-modify-width-left",
    category: Category::FractionDelta,
};

pub const WINDOW_MODIFY_WIDTH_RIGHT: Spec = Spec {
    name: "window-modify-width-right",
    category: Category::FractionDelta,
};

pub const WINDOW_MOVE_DOWN: Spec = Spec {
    name: "window-move-down",
    category: Category::None,
};

pub const WINDOW_MOVE_OR_OUTPUT_DOWN: Spec = Spec {
    name: "window-move-or-output-down",
    category: Category::None,
};

pub const WINDOW_MOVE_OR_OUTPUT_LEFT: Spec = Spec {
    name: "window-move-or-output-left",
    category: Category::None,
};

pub const WINDOW_MOVE_OR_OUTPUT_RIGHT: Spec = Spec {
    name: "window-move-or-output-right",
    category: Category::None,
};

pub const WINDOW_MOVE_OR_OUTPUT_UP: Spec = Spec {
    name: "window-move-or-output-up",
    category: Category::None,
};

pub const WINDOW_MOVE_OR_WORKSPACE_DOWN: Spec = Spec {
    name: "window-move-or-workspace-down",
    category: Category::None,
};

pub const WINDOW_MOVE_OR_WORKSPACE_UP: Spec = Spec {
    name: "window-move-or-workspace-up",
    category: Category::None,
};

pub const WINDOW_MOVE_TO_OUTPUT_DOWN: Spec = Spec {
    name: "window-move-to-output-down",
    category: Category::None,
};

pub const WINDOW_MOVE_TO_OUTPUT_LEFT: Spec = Spec {
    name: "window-move-to-output-left",
    category: Category::None,
};

pub const WINDOW_MOVE_TO_OUTPUT_NEXT: Spec = Spec {
    name: "window-move-to-output-next",
    category: Category::None,
};

pub const WINDOW_MOVE_TO_OUTPUT_PREVIOUS: Spec = Spec {
    name: "window-move-to-output-previous",
    category: Category::None,
};

pub const WINDOW_MOVE_TO_OUTPUT_RIGHT: Spec = Spec {
    name: "window-move-to-output-right",
    category: Category::None,
};

pub const WINDOW_MOVE_TO_OUTPUT_UP: Spec = Spec {
    name: "window-move-to-output-up",
    category: Category::None,
};

pub const WINDOW_MOVE_TO_SCRATCHPAD: Spec = Spec {
    name: "window-move-to-scratchpad",
    category: Category::OptionalScratchpad,
};

pub const WINDOW_MOVE_TO_WORKSPACE: Spec = Spec {
    name: "window-move-to-workspace",
    category: Category::Workspace,
};

pub const WINDOW_MOVE_TO_WORKSPACE_NEXT: Spec = Spec {
    name: "window-move-to-workspace-next",
    category: Category::None,
};

pub const WINDOW_MOVE_TO_WORKSPACE_PREVIOUS: Spec = Spec {
    name: "window-move-to-workspace-previous",
    category: Category::None,
};

pub const WINDOW_MOVE_UP: Spec = Spec {
    name: "window-move-up",
    category: Category::None,
};

pub const WINDOW_RESTORE_FROM_SCRATCHPAD: Spec = Spec {
    name: "window-restore-from-scratchpad",
    category: Category::OptionalScratchpad,
};

pub const WINDOW_SET_PRIMARY_EXTENT: Spec = Spec {
    name: "window-set-primary-extent",
    category: Category::Fraction,
};

pub const WINDOW_SET_SECONDARY_EXTENT: Spec = Spec {
    name: "window-set-secondary-extent",
    category: Category::Fraction,
};

pub const WINDOW_SWAP_NEXT: Spec = Spec {
    name: "window-swap-next",
    category: Category::None,
};

pub const WINDOW_SWAP_PREVIOUS: Spec = Spec {
    name: "window-swap-previous",
    category: Category::None,
};

pub const WINDOW_TOGGLE_FLOATING: Spec = Spec {
    name: "window-toggle-floating",
    category: Category::OptionalWindowId,
};

pub const WINDOW_TOGGLE_FULLSCREEN: Spec = Spec {
    name: "window-toggle-fullscreen",
    category: Category::None,
};

pub const WINDOW_TOGGLE_MAXIMIZE: Spec = Spec {
    name: "window-toggle-maximize",
    category: Category::None,
};

pub const WINDOW_TOGGLE_MAXIMIZE_TO_EDGES: Spec = Spec {
    name: "window-toggle-maximize-to-edges",
    category: Category::None,
};

pub const WINDOW_TOGGLE_PINNED: Spec = Spec {
    name: "window-toggle-pinned",
    category: Category::None,
};

pub const WINDOW_TOGGLE_SCRATCHPAD: Spec = Spec {
    name: "window-toggle-scratchpad",
    category: Category::OptionalScratchpad,
};

pub const WORKSPACE_FOCUS_LAST: Spec = Spec {
    name: "workspace-focus-last",
    category: Category::None,
};

pub const WORKSPACE_MOVE_DOWN: Spec = Spec {
    name: "workspace-move-down",
    category: Category::None,
};

pub const WORKSPACE_MOVE_TO_OUTPUT_DOWN: Spec = Spec {
    name: "workspace-move-to-output-down",
    category: Category::None,
};

pub const WORKSPACE_MOVE_TO_OUTPUT_LEFT: Spec = Spec {
    name: "workspace-move-to-output-left",
    category: Category::None,
};

pub const WORKSPACE_MOVE_TO_OUTPUT_RIGHT: Spec = Spec {
    name: "workspace-move-to-output-right",
    category: Category::None,
};

pub const WORKSPACE_MOVE_TO_OUTPUT_UP: Spec = Spec {
    name: "workspace-move-to-output-up",
    category: Category::None,
};

pub const WORKSPACE_MOVE_UP: Spec = Spec {
    name: "workspace-move-up",
    category: Category::None,
};

pub const WORKSPACE_NEXT: Spec = Spec {
    name: "workspace-next",
    category: Category::None,
};

pub const WORKSPACE_PREVIOUS: Spec = Spec {
    name: "workspace-previous",
    category: Category::None,
};

pub const WORKSPACE_SET_LAYOUT: Spec = Spec {
    name: "workspace-set-layout",
    category: Category::LayoutMode,
};

pub const WORKSPACE_SWAP_ACTIVE_OUTPUT_DOWN: Spec = Spec {
    name: "workspace-swap-active-output-down",
    category: Category::None,
};

pub const WORKSPACE_SWAP_ACTIVE_OUTPUT_LEFT: Spec = Spec {
    name: "workspace-swap-active-output-left",
    category: Category::None,
};

pub const WORKSPACE_SWAP_ACTIVE_OUTPUT_NEXT: Spec = Spec {
    name: "workspace-swap-active-output-next",
    category: Category::None,
};

pub const WORKSPACE_SWAP_ACTIVE_OUTPUT_PREVIOUS: Spec = Spec {
    name: "workspace-swap-active-output-previous",
    category: Category::None,
};

pub const WORKSPACE_SWAP_ACTIVE_OUTPUT_RIGHT: Spec = Spec {
    name: "workspace-swap-active-output-right",
    category: Category::None,
};

pub const WORKSPACE_SWAP_ACTIVE_OUTPUT_UP: Spec = Spec {
    name: "workspace-swap-active-output-up",
    category: Category::None,
};

pub const WORKSPACE_SWITCH: Spec = Spec {
    name: "workspace-switch",
    category: Category::Workspace,
};

/// Every upstream action, sorted and unique by name. Family builders may land
/// after their entries appear here.
pub const IMPLEMENTED: &[Spec] = &[
    CHEATSHEET_CLOSE,
    CHEATSHEET_OPEN,
    CHEATSHEET_TOGGLE,
    COLUMN_CENTER,
    COLUMN_FOCUS_FIRST,
    COLUMN_FOCUS_LAST,
    COLUMN_MOVE_LEFT,
    COLUMN_MOVE_RIGHT,
    COLUMN_MOVE_TO_FIRST,
    COLUMN_MOVE_TO_LAST,
    COLUMN_MOVE_TO_OUTPUT_DOWN,
    COLUMN_MOVE_TO_OUTPUT_LEFT,
    COLUMN_MOVE_TO_OUTPUT_RIGHT,
    COLUMN_MOVE_TO_OUTPUT_UP,
    COLUMN_MOVE_TO_WORKSPACE,
    COLUMN_MOVE_TO_WORKSPACE_NEXT,
    COLUMN_MOVE_TO_WORKSPACE_PREVIOUS,
    CONFIG_RELOAD,
    DPMS_OFF,
    DPMS_ON,
    KEYBOARD_LAYOUT_NEXT,
    LAYOUT_MASTER_COUNT_DECREASE,
    LAYOUT_MASTER_COUNT_INCREASE,
    LAYOUT_SCROLL_DOWN,
    LAYOUT_SCROLL_DRAG,
    LAYOUT_SCROLL_LEFT,
    LAYOUT_SCROLL_RIGHT,
    LAYOUT_SCROLL_UP,
    OUTPUT_FOCUS_DOWN,
    OUTPUT_FOCUS_LEFT,
    OUTPUT_FOCUS_NEXT,
    OUTPUT_FOCUS_PREVIOUS,
    OUTPUT_FOCUS_RIGHT,
    OUTPUT_FOCUS_UP,
    OVERVIEW_CLOSE,
    OVERVIEW_OPEN,
    OVERVIEW_TOGGLE,
    SCRATCHPAD_FOCUS_NEXT,
    SCRATCHPAD_TOGGLE,
    SESSION_QUIT,
    SHORTCUTS_INHIBIT_TOGGLE,
    SPAWN,
    SUBMAP,
    WINDOW_CENTER,
    WINDOW_CLOSE,
    WINDOW_CONSUME_LEFT,
    WINDOW_CONSUME_OR_EXPEL_LEFT,
    WINDOW_CONSUME_OR_EXPEL_RIGHT,
    WINDOW_CONSUME_RIGHT,
    WINDOW_CYCLE_PRIMARY_EXTENT,
    WINDOW_CYCLE_PRIMARY_EXTENT_BACK,
    WINDOW_CYCLE_SECONDARY_EXTENT,
    WINDOW_CYCLE_SECONDARY_EXTENT_BACK,
    WINDOW_FOCUS,
    WINDOW_FOCUS_DOWN,
    WINDOW_FOCUS_LAST,
    WINDOW_FOCUS_LEFT,
    WINDOW_FOCUS_NEXT,
    WINDOW_FOCUS_OR_OUTPUT_DOWN,
    WINDOW_FOCUS_OR_OUTPUT_LEFT,
    WINDOW_FOCUS_OR_OUTPUT_RIGHT,
    WINDOW_FOCUS_OR_OUTPUT_UP,
    WINDOW_FOCUS_OR_WORKSPACE_DOWN,
    WINDOW_FOCUS_OR_WORKSPACE_UP,
    WINDOW_FOCUS_PREVIOUS,
    WINDOW_FOCUS_RIGHT,
    WINDOW_FOCUS_SWITCH_FLOATING,
    WINDOW_FOCUS_UP,
    WINDOW_FOCUS_WARP,
    WINDOW_MODIFY_HEIGHT_DOWN,
    WINDOW_MODIFY_HEIGHT_UP,
    WINDOW_MODIFY_PRIMARY_EXTENT,
    WINDOW_MODIFY_SECONDARY_EXTENT,
    WINDOW_MODIFY_WIDTH_LEFT,
    WINDOW_MODIFY_WIDTH_RIGHT,
    WINDOW_MOVE_DOWN,
    WINDOW_MOVE_OR_OUTPUT_DOWN,
    WINDOW_MOVE_OR_OUTPUT_LEFT,
    WINDOW_MOVE_OR_OUTPUT_RIGHT,
    WINDOW_MOVE_OR_OUTPUT_UP,
    WINDOW_MOVE_OR_WORKSPACE_DOWN,
    WINDOW_MOVE_OR_WORKSPACE_UP,
    WINDOW_MOVE_TO_OUTPUT_DOWN,
    WINDOW_MOVE_TO_OUTPUT_LEFT,
    WINDOW_MOVE_TO_OUTPUT_NEXT,
    WINDOW_MOVE_TO_OUTPUT_PREVIOUS,
    WINDOW_MOVE_TO_OUTPUT_RIGHT,
    WINDOW_MOVE_TO_OUTPUT_UP,
    WINDOW_MOVE_TO_SCRATCHPAD,
    WINDOW_MOVE_TO_WORKSPACE,
    WINDOW_MOVE_TO_WORKSPACE_NEXT,
    WINDOW_MOVE_TO_WORKSPACE_PREVIOUS,
    WINDOW_MOVE_UP,
    WINDOW_RESTORE_FROM_SCRATCHPAD,
    WINDOW_SET_PRIMARY_EXTENT,
    WINDOW_SET_SECONDARY_EXTENT,
    WINDOW_SWAP_NEXT,
    WINDOW_SWAP_PREVIOUS,
    WINDOW_TOGGLE_FLOATING,
    WINDOW_TOGGLE_FULLSCREEN,
    WINDOW_TOGGLE_MAXIMIZE,
    WINDOW_TOGGLE_MAXIMIZE_TO_EDGES,
    WINDOW_TOGGLE_PINNED,
    WINDOW_TOGGLE_SCRATCHPAD,
    WORKSPACE_FOCUS_LAST,
    WORKSPACE_MOVE_DOWN,
    WORKSPACE_MOVE_TO_OUTPUT_DOWN,
    WORKSPACE_MOVE_TO_OUTPUT_LEFT,
    WORKSPACE_MOVE_TO_OUTPUT_RIGHT,
    WORKSPACE_MOVE_TO_OUTPUT_UP,
    WORKSPACE_MOVE_UP,
    WORKSPACE_NEXT,
    WORKSPACE_PREVIOUS,
    WORKSPACE_SET_LAYOUT,
    WORKSPACE_SWAP_ACTIVE_OUTPUT_DOWN,
    WORKSPACE_SWAP_ACTIVE_OUTPUT_LEFT,
    WORKSPACE_SWAP_ACTIVE_OUTPUT_NEXT,
    WORKSPACE_SWAP_ACTIVE_OUTPUT_PREVIOUS,
    WORKSPACE_SWAP_ACTIVE_OUTPUT_RIGHT,
    WORKSPACE_SWAP_ACTIVE_OUTPUT_UP,
    WORKSPACE_SWITCH,
];
