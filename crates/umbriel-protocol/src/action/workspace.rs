//! Workspace reference grammar: first slash splits output; quotes force names.
use std::fmt;

use super::{Action, ValidationError, catalog};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reference<'a> {
    Index(u8),
    Name(&'a str),
}

/// A validated index or literal name, not interchangeable numeric strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceSelector<'a>(Reference<'a>);

impl<'a> WorkspaceSelector<'a> {
    pub fn index(index: usize) -> Result<Self, ValidationError> {
        if !(1..=64).contains(&index) {
            return Err(ValidationError::WorkspaceIndex { index });
        }
        Ok(Self(Reference::Index(index as u8)))
    }

    /// Accept every representable name. Interior quotes are legal in unquoted
    /// names; leading/trailing quotes cannot be represented without changing
    /// meaning. Slashes split before quote processing, so cannot be escaped.
    pub fn name(name: &'a str) -> Result<Self, ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::Empty {
                argument: "workspace name",
            });
        }
        if name.contains('/') || name.starts_with('"') || name.ends_with('"') {
            return Err(ValidationError::UnrepresentableWorkspaceName);
        }
        Ok(Self(Reference::Name(name)))
    }
}

impl fmt::Display for WorkspaceSelector<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Reference::Index(index) => write!(f, "{index}"),
            Reference::Name(name) if name.bytes().all(|byte| byte.is_ascii_digit()) => {
                write!(f, "\"{name}\"")
            }
            Reference::Name(name) => f.write_str(name),
        }
    }
}

/// A validated selector with an optional output qualifier. Cheap to copy and
/// reusable by all three upstream workspace-taking action families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceTarget<'a> {
    selector: WorkspaceSelector<'a>,
    output: Option<&'a str>,
}

impl<'a> WorkspaceTarget<'a> {
    pub const fn new(selector: WorkspaceSelector<'a>) -> Self {
        Self {
            selector,
            output: None,
        }
    }

    /// Validate grammar, not output existence. Colons, quotes, whitespace and
    /// non-ASCII text are legal here; only empty and slash-containing forms fail.
    pub fn on_output(mut self, output: &'a str) -> Result<Self, ValidationError> {
        if output.is_empty() {
            return Err(ValidationError::Empty { argument: "output" });
        }
        if output.contains('/') {
            return Err(ValidationError::InvalidWorkspaceOutput);
        }
        self.output = Some(output);
        Ok(self)
    }

    pub const fn selector(self) -> WorkspaceSelector<'a> {
        self.selector
    }

    pub const fn output(self) -> Option<&'a str> {
        self.output
    }
}

impl<'a> From<WorkspaceSelector<'a>> for WorkspaceTarget<'a> {
    fn from(selector: WorkspaceSelector<'a>) -> Self {
        Self::new(selector)
    }
}

impl fmt::Display for WorkspaceTarget<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.selector.fmt(f)?;
        if let Some(output) = self.output {
            write!(f, "/{output}")?;
        }
        Ok(())
    }
}

impl Action<'static> {
    /// Prepare owned text once; the target's borrowed inputs need not outlive
    /// this action. An unqualified index uses the cursor-preferred output;
    /// unqualified names resolve globally using compositor disambiguation.
    pub fn workspace_switch<'a>(target: impl Into<WorkspaceTarget<'a>>) -> Self {
        Self::parameterized(catalog::WORKSPACE_SWITCH, target.into())
    }
}
