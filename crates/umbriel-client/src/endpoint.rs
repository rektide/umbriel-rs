//! Discovery configuration is injectable; no tests mutate process environment.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct Discovery {
    pub explicit: Option<PathBuf>,
    pub umbriel_socket: Option<OsString>,
    pub runtime_dir: Option<OsString>,
    pub wayland_display: Option<OsString>,
}

#[derive(Debug, Error)]
#[error("missing Umbriel endpoint: set UMBRIEL_SOCKET or both XDG_RUNTIME_DIR and WAYLAND_DISPLAY")]
pub struct MissingConfiguration;

impl Discovery {
    pub fn from_env() -> Self {
        Self {
            explicit: None,
            umbriel_socket: std::env::var_os("UMBRIEL_SOCKET"),
            runtime_dir: std::env::var_os("XDG_RUNTIME_DIR"),
            wayland_display: std::env::var_os("WAYLAND_DISPLAY"),
        }
    }

    pub fn resolve(&self) -> Result<Endpoint, MissingConfiguration> {
        if let Some(path) = &self.explicit {
            return Ok(Endpoint::new(path));
        }
        if let Some(socket) = self.umbriel_socket.as_ref().filter(|s| !s.is_empty()) {
            return Ok(Endpoint::new(PathBuf::from(socket)));
        }
        let runtime = self
            .runtime_dir
            .as_ref()
            .filter(|s| !s.is_empty())
            .ok_or(MissingConfiguration)?;
        let display = self
            .wayland_display
            .as_ref()
            .filter(|s| !s.is_empty())
            .ok_or(MissingConfiguration)?;
        // CLI concatenation, not Path::join(display): absolute display and embedded
        // slashes do not replace the runtime prefix; arbitrary Unix bytes survive.
        let mut path = runtime.clone();
        path.push("/umbriel-");
        path.push(display);
        path.push(".sock");
        Ok(Endpoint::new(PathBuf::from(path)))
    }
}

/// Immutable endpoint, not an open socket or server owner.
#[derive(Debug, Clone)]
pub struct Endpoint(PathBuf);

impl Endpoint {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}
