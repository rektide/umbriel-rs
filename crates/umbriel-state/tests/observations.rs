use std::time::Duration;
use umbriel_protocol::{envelope::Family, model::*};
use umbriel_state::{
    active::{Error as ActiveError, StableActiveReducer},
    inventory::*,
};

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}
fn window(id: &str) -> Window {
    Window {
        id: id.into(),
        workspace: "ws".into(),
        scratchpad: String::new(),
        active: true,
        title: "first".into(),
        app_id: "app".into(),
        xdg_tag: String::new(),
        content_type: ContentType::None,
        floating: false,
        focused: false,
        urgent: false,
        xwayland: false,
        pid: -1,
        x: -5,
        y: 0,
        w: 10,
        h: 10,
    }
}
fn workspace(id: &str) -> Workspace {
    Workspace {
        id: id.into(),
        name: "1".into(),
        named: false,
        index: 1,
        output: "output".into(),
        active: true,
        focused: true,
        occupied: true,
        layout: LayoutMode::Scrolling,
    }
}
fn stamp(epoch: u64, position: u64) -> Stamp {
    Stamp {
        epoch,
        position,
        observed_at: ms(position),
    }
}

#[test]
fn title_churn_and_timer_only_progress_emit_latest_once() {
    let mut r = StableActiveReducer::default();
    let mut a = window("A");
    let mut remembered = window("remembered");
    remembered.active = false;
    remembered.focused = true;
    r.observe(1, ms(0), &[a.clone(), remembered]).unwrap();
    a.title = "second".into();
    r.observe(1, ms(100), &[a.clone()]).unwrap();
    a.title = "latest".into();
    a.workspace = "new-workspace".into();
    r.observe(1, ms(600), &[a]).unwrap();
    assert_eq!(r.deadline(), Some(ms(5000)));
    assert!(r.poll(ms(4999)).unwrap().is_none());
    let out = r.poll(ms(5000)).unwrap().unwrap();
    let latest = out.window.unwrap();
    assert_eq!(latest.title, "latest");
    assert_eq!(latest.workspace.as_str(), "new-workspace");
    assert_eq!(out.since, ms(0));
    r.observe(1, ms(5250), &[window("A")]).unwrap();
    assert_eq!(
        r.deadline(),
        None,
        "same-ID metadata does not create a new episode"
    );
    assert!(r.poll(ms(5750)).unwrap().is_none());
}

#[test]
fn a_b_a_and_repeat_episodes() {
    // Pinned dwell keeps these episode traces decoupled from the default.
    let mut r = StableActiveReducer::new(ms(750));
    r.observe(1, ms(0), &[window("A")]).unwrap();
    r.observe(1, ms(400), &[window("B")]).unwrap();
    r.observe(1, ms(450), &[window("A")]).unwrap();
    assert!(r.poll(ms(750)).unwrap().is_none());
    assert_eq!(
        r.poll(ms(1200))
            .unwrap()
            .unwrap()
            .window
            .unwrap()
            .id
            .as_str(),
        "A"
    );
    r.observe(1, ms(1300), &[window("B")]).unwrap();
    r.observe(1, ms(1400), &[window("A")]).unwrap();
    assert!(r.poll(ms(2149)).unwrap().is_none());
    assert!(r.poll(ms(2150)).unwrap().is_some());
}

#[test]
fn none_cancels_immediately_and_gaps_restart_dwell() {
    // Pinned dwell keeps these episode traces decoupled from the default.
    let mut r = StableActiveReducer::new(ms(750));
    r.observe(1, ms(0), &[window("A")]).unwrap();
    r.observe(1, ms(700), &[]).unwrap();
    assert!(r.poll(ms(750)).unwrap().is_none());
    assert!(r.poll(ms(1450)).unwrap().unwrap().window.is_none());
    r.observe(1, ms(1500), &[window("A")]).unwrap();
    r.invalidate(); // disconnect, decode failure, or lag
    assert!(r.poll(ms(2250)).unwrap().is_none());
    r.observe(2, ms(2300), &[window("A")]).unwrap();
    assert_eq!(r.deadline(), Some(ms(3050)));
    r.observe(3, ms(2500), &[window("A")]).unwrap(); // epoch alone breaks continuity
    assert!(r.poll(ms(3050)).unwrap().is_none());
    assert_eq!(r.poll(ms(3250)).unwrap().unwrap().epoch, 3);
}

#[test]
fn invalid_time_and_census_fail_closed() {
    let mut r = StableActiveReducer::default();
    r.observe(1, ms(100), &[window("A")]).unwrap();
    assert_eq!(r.poll(ms(99)), Err(ActiveError::TimeWentBackwards));
    assert_eq!(r.deadline(), None);
    assert!(r.poll(ms(900)).unwrap().is_none());
    assert_eq!(
        r.observe(1, ms(901), &[window("A"), window("B")]),
        Err(ActiveError::MultipleActive)
    );
    assert_eq!(
        r.observe(1, ms(902), &[window("")]),
        Err(ActiveError::EmptyActiveId)
    );
    assert_eq!(
        r.observe(1, Duration::MAX, &[window("A")]),
        Err(ActiveError::DeadlineOverflow)
    );
    assert_eq!(r.deadline(), None);
}

#[test]
fn replacement_validation_empty_and_stale_are_coherent() {
    let mut inventory = Inventory::new(1);
    assert!(!inventory.status(Family::Windows).is_ready());
    inventory
        .install(stamp(1, 1), Event::Windows(vec![window("B"), window("A")]))
        .unwrap();
    let prior = inventory.snapshot();
    assert_eq!(
        prior.windows.source_order(),
        vec![WindowId::from("B"), WindowId::from("A")]
    );
    assert_eq!(
        inventory.install(stamp(1, 2), Event::Windows(vec![window("C"), window("C")])),
        Err(Error::DuplicateId)
    );
    let rejected = inventory.snapshot();
    assert_eq!(rejected.windows.entries().len(), 2);
    assert_eq!(rejected.windows.status.stamp, Some(stamp(1, 1)));
    assert_eq!(rejected.windows.status.last_attempt, Some(stamp(1, 2)));
    assert_eq!(
        rejected.windows.status.health,
        Health::Stale(StaleReason::InvalidSnapshot)
    );
    assert_eq!(
        inventory.install(
            stamp(1, 3),
            Event::Windows(vec![window("valid"), window("")])
        ),
        Err(Error::EmptyId)
    );
    assert_eq!(inventory.snapshot().windows.entries().len(), 2);
    inventory
        .install(stamp(1, 4), Event::Windows(vec![]))
        .unwrap();
    let empty = inventory.snapshot();
    assert!(empty.windows.entries().is_empty());
    assert!(empty.windows.status.is_ready());
    assert_eq!(empty.windows.status.stamp, Some(stamp(1, 4)));
    assert_eq!(prior.windows.entries().len(), 2); // retained captures remain unchanged
    inventory.gap();
    assert!(!inventory.snapshot().connected);
    assert_eq!(
        inventory.install(stamp(1, 5), Event::Windows(vec![])),
        Err(Error::SourceGap)
    );
    inventory.begin_epoch(2).unwrap();
    assert_eq!(
        inventory.status(Family::Windows).health,
        Health::Stale(StaleReason::EpochChanged)
    );
    inventory
        .install(stamp(2, 1), Event::Windows(vec![window("new")]))
        .unwrap();
    assert!(inventory.status(Family::Windows).is_ready());
    assert_eq!(inventory.snapshot().windows.entries().len(), 1);
}

#[test]
fn join_carries_both_family_stamps_and_every_unmatched_row() {
    let mut inventory = Inventory::new(1);
    inventory
        .install(stamp(1, 1), Event::Workspaces(vec![workspace("ws")]))
        .unwrap();
    let mut scratchpad = window("pad");
    scratchpad.workspace = "".into();
    scratchpad.scratchpad = "default".into();
    let mut unmatched = window("unmatched");
    unmatched.workspace = "missing".into();
    let mut detached = window("detached");
    detached.workspace = "".into();
    inventory
        .install(
            stamp(1, 2),
            Event::Windows(vec![window("matched"), scratchpad, unmatched, detached]),
        )
        .unwrap();
    let capture = inventory.snapshot();
    let joined = capture.join_windows();
    assert_eq!(joined.windows_status.stamp, Some(stamp(1, 2)));
    assert_eq!(joined.workspaces_status.stamp, Some(stamp(1, 1)));
    assert!(matches!(
        joined.rows[0].workspace,
        WorkspaceMatch::Matched(_)
    ));
    assert!(matches!(
        joined.rows[1].workspace,
        WorkspaceMatch::Scratchpad { .. }
    ));
    assert!(matches!(
        joined.rows[2].workspace,
        WorkspaceMatch::Unmatched(_)
    ));
    assert!(matches!(
        joined.rows[3].workspace,
        WorkspaceMatch::NoWorkspace
    ));
    assert_eq!(
        inventory.install(
            stamp(1, 3),
            Event::Workspaces(vec![workspace("ws"), workspace("ws")])
        ),
        Err(Error::DuplicateId)
    );
    assert_eq!(
        inventory.install(stamp(1, 4), Event::Workspaces(vec![workspace("")])),
        Err(Error::EmptyId)
    );
    inventory
        .install(stamp(1, 5), Event::Workspaces(vec![]))
        .unwrap();
    assert!(matches!(
        inventory.snapshot().join_windows().rows[0].workspace,
        WorkspaceMatch::Unmatched(_)
    ));
    assert!(matches!(
        capture.join_windows().rows[0].workspace,
        WorkspaceMatch::Matched(_)
    ));
    inventory.begin_epoch(2).unwrap();
    let stale = inventory.snapshot();
    assert_eq!(stale.windows.entries().len(), 4);
    assert!(!stale.windows.status.is_ready());
    inventory
        .install(stamp(2, 1), Event::Workspaces(vec![workspace("ws")]))
        .unwrap();
    let skew = inventory.snapshot().join_windows();
    assert_eq!(skew.windows_status.stamp, Some(stamp(1, 2)));
    assert_eq!(skew.workspaces_status.stamp, Some(stamp(2, 1)));
    assert_eq!(
        skew.windows_status.health,
        Health::Stale(StaleReason::EpochChanged)
    );
    assert!(skew.workspaces_status.is_ready());
}

#[test]
fn singleton_null_and_local_ordering_are_explicit() {
    let mut inventory = Inventory::new(1);
    inventory.install(stamp(1, 1), Event::Submap(None)).unwrap();
    assert!(inventory.snapshot().submap.entries()[&()].is_none());
    assert!(inventory.status(Family::Submap).is_ready());
    assert!(!inventory.status(Family::KeyboardLayout).is_ready());
    assert_eq!(
        inventory.install(stamp(1, 1), Event::Windows(vec![])),
        Err(Error::NonIncreasingPosition)
    );
    assert_eq!(
        inventory.install(
            Stamp {
                observed_at: ms(0),
                ..stamp(1, 2)
            },
            Event::Windows(vec![])
        ),
        Err(Error::TimeWentBackwards)
    );
    assert_eq!(
        inventory.install(stamp(2, 2), Event::Windows(vec![])),
        Err(Error::WrongEpoch)
    );
    inventory.invalid_snapshot(Family::Submap);
    assert_eq!(
        inventory.status(Family::Submap).health,
        Health::Stale(StaleReason::InvalidSnapshot)
    );
    assert!(inventory.snapshot().submap.entries()[&()].is_none());
}

#[test]
fn window_receipts_share_installation_and_preserve_actual_occupancy() {
    let mut inventory = Inventory::new(1);
    let first = inventory
        .install_windows_changes(stamp(1, 1), vec![window("B"), window("A")])
        .unwrap();
    assert!(!first.is_continuous());
    assert_eq!(first.before.health, Health::Unseen);
    assert_eq!(first.commit.generation.get(), 1);
    assert!(
        first
            .commit
            .evidence
            .get(&"A".into())
            .unwrap()
            .before
            .is_none()
    );
    let before = inventory.snapshot();
    let mut moved = window("A");
    moved.workspace = "other".into();
    let second = inventory
        .install_windows_changes(stamp(1, 2), vec![moved.clone(), window("C")])
        .unwrap();
    assert!(second.is_continuous());
    let a = second.commit.evidence.get(&"A".into()).unwrap();
    assert_eq!(a.before.as_ref().unwrap().workspace.as_str(), "ws");
    assert_eq!(a.after.as_ref().unwrap().workspace.as_str(), "other");
    assert!(std::sync::Arc::ptr_eq(
        a.before.as_ref().unwrap(),
        &before.windows.entries()[&"A".into()]
    ));
    assert!(std::sync::Arc::ptr_eq(
        a.after.as_ref().unwrap(),
        &inventory.snapshot().windows.entries()[&"A".into()]
    ));
    assert!(
        second
            .commit
            .evidence
            .get(&"B".into())
            .unwrap()
            .after
            .is_none()
    );
    assert!(
        second
            .commit
            .evidence
            .get(&"C".into())
            .unwrap()
            .before
            .is_none()
    );
    assert_eq!(
        inventory.snapshot().windows.source_order(),
        &[WindowId::from("A"), WindowId::from("C")]
    );

    let mut equal = inventory
        .install_windows_changes(stamp(1, 3), vec![window("C"), moved])
        .unwrap();
    assert_eq!(
        equal.commit.evidence.len(),
        2,
        "actual equal touches retained"
    );
    equal.commit.evidence.retain(|row| row.before != row.after);
    assert!(equal.commit.evidence.is_empty());
    assert_eq!(
        inventory.snapshot().windows.source_order(),
        &[WindowId::from("C"), WindowId::from("A")]
    );
    let retained = inventory.snapshot();
    assert!(matches!(
        inventory
            .install_windows_changes(stamp(1, 4), vec![window("D"), window("D"), window("tail")]),
        Err(Error::DuplicateId)
    ));
    let rejected = inventory.snapshot();
    assert_eq!(rejected.windows.entries(), retained.windows.entries());
    assert_eq!(
        rejected.windows.source_order(),
        retained.windows.source_order()
    );
    assert_eq!(rejected.windows.status.stamp, Some(stamp(1, 3)));
    assert_eq!(rejected.windows.status.last_attempt, Some(stamp(1, 4)));
    let empty = inventory
        .install_windows_changes(stamp(1, 5), vec![])
        .unwrap();
    assert!(!empty.is_continuous());
    assert_eq!(
        empty.before.health,
        Health::Stale(StaleReason::InvalidSnapshot)
    );
    assert_eq!(
        empty.commit.generation.get(),
        4,
        "rejection does not commit"
    );
    assert!(
        empty
            .commit
            .evidence
            .iter()
            .all(|row| row.before.is_some() && row.after.is_none())
    );
    assert!(inventory.snapshot().windows.source_order().is_empty());
    assert_eq!(empty.after.health, Health::Current);
    assert!(
        inventory
            .install_windows_changes(stamp(1, 6), vec![])
            .unwrap()
            .commit
            .evidence
            .is_empty()
    );
}

#[test]
fn window_receipts_label_gaps_epochs_and_ordinary_installs() {
    let mut inventory = Inventory::new(1);
    inventory
        .install(stamp(1, 1), Event::Windows(vec![window("A")]))
        .unwrap();
    let prior = inventory.snapshot().windows.entries()[&"A".into()].clone();
    let weak = std::sync::Arc::downgrade(&prior);
    drop(prior);
    // The ordinary path does not retain a former-row receipt.
    inventory
        .install(stamp(1, 2), Event::Windows(vec![window("B")]))
        .unwrap();
    assert!(weak.upgrade().is_none());
    let receipt = inventory
        .install_windows_changes(stamp(1, 3), vec![window("C")])
        .unwrap();
    assert!(receipt.is_continuous());
    assert_eq!(receipt.commit.generation.get(), 3);
    assert_eq!(receipt.before.stamp, Some(stamp(1, 2)));
    inventory.gap();
    assert!(matches!(
        inventory.install_windows_changes(stamp(1, 4), vec![]),
        Err(Error::SourceGap)
    ));
    assert_eq!(
        inventory.status(Family::Windows).health,
        Health::Stale(StaleReason::Gap)
    );
    inventory.begin_epoch(2).unwrap();
    let reacquired = inventory
        .install_windows_changes(stamp(2, 1), vec![window("D")])
        .unwrap();
    assert!(!reacquired.is_continuous());
    assert_eq!(
        reacquired.before.health,
        Health::Stale(StaleReason::EpochChanged)
    );
    assert_eq!(reacquired.before.stamp, Some(stamp(1, 3)));
    assert!(
        reacquired
            .commit
            .evidence
            .get(&"C".into())
            .unwrap()
            .after
            .is_none()
    );
    assert!(
        inventory
            .install_windows_changes(stamp(2, 2), vec![window("D")])
            .unwrap()
            .is_continuous()
    );
    inventory.invalid_snapshot(Family::Windows);
    assert!(
        !inventory
            .install_windows_changes(stamp(2, 3), vec![])
            .unwrap()
            .is_continuous()
    );
    assert!(matches!(
        inventory.install_windows_changes(stamp(1, 99), vec![]),
        Err(Error::WrongEpoch)
    ));
    assert!(matches!(
        inventory.install_windows_changes(stamp(2, 3), vec![]),
        Err(Error::NonIncreasingPosition)
    ));
    assert!(matches!(
        inventory.install_windows_changes(
            Stamp {
                observed_at: Duration::ZERO,
                ..stamp(2, 4)
            },
            vec![]
        ),
        Err(Error::TimeWentBackwards)
    ));
    assert!(matches!(
        inventory.install_windows_changes(stamp(2, 5), vec![window("")]),
        Err(Error::EmptyId)
    ));
}
