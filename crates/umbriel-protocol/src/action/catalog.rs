//! Declarative name/category inventory for the implemented typed slice.
//! Extend alongside family builders; this is not a gate on raw actions.

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

pub const OVERVIEW_TOGGLE: Spec = Spec {
    name: "overview-toggle",
    category: Category::None,
};

pub const IMPLEMENTED: &[Spec] = &[OVERVIEW_TOGGLE];
