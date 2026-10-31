use super::*;

fn exact(j: &mut impl Access, cursor: &str, boundary: &'static str) -> Result<(), Error> {
    if cursor.is_empty() || cursor.contains('\0') { return Err(Error::CursorUnavailable(boundary)); }
    j.seek(cursor)?;
    if !j.next()? || !j.test(cursor)? { return Err(Error::CursorUnavailable(boundary)); }
    Ok(())
}

pub(super) fn bounded(j: &mut impl Access, feed: &Feed, start: Start, after: Option<&Cursor>, until: Option<u64>, limits: Limits, cancelled: &dyn Fn() -> bool) -> Result<Replay, Error> {
    feed.validate()?;
    if after.is_some_and(|c| &c.feed != feed) { return Err(Error::CursorFeedMismatch); }
    let mut budget = Budget { limits, started: Instant::now(), entries: 0, bytes: 0, cancelled };
    budget.check()?;
    if j.process()? && after.is_some() { return Err(Error::Invalidated); }
    // Capture the inclusive upper entry first. Until is an exclusive journal
    // receive-time bound (not the producer's application event_time).
    if let Some(until) = until { j.seek_time(until)?; } else { j.tail()?; }
    let upper = loop {
        budget.check()?;
        if !j.previous()? { break None; }
        if until.is_none_or(|until| j.time().is_ok_and(|t| t < until)) { break Some(j.cursor()?); }
        budget.entry()?;
    };
    let mut first = None;
    if let Some(after) = after {
        exact(j, &after.opaque, "lower")?;
    } else if let Some(upper) = &upper {
        match start {
            Start::Head => j.head()?,
            Start::Since(time) => {
                // If since exceeds this captured upper, the range is empty.
                exact(j, upper, "upper")?;
                if j.time()? >= time { j.seek_time(time)?; } else { return finish(j, feed, after, Some(upper.clone()), Vec::new(), &budget); }
            }
            Start::Tail(count) => {
                if count == 0 { return Err(Error::Budget("nonzero tail")); }
                exact(j, upper, "upper")?;
                for _ in 1..count {
                    budget.check()?;
                    let prior = j.cursor()?;
                    if !j.previous()? { exact(j, &prior, "tail start")?; break; }
                }
                first = Some(j.cursor()?);
            }
        }
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut ids: std::collections::BTreeMap<_, usize> = std::collections::BTreeMap::new();
    if let Some(upper) = &upper {
        if !(after.is_some() && j.test(upper)?) {
            loop {
                budget.check()?;
                if first.take().is_none() && !j.next()? { return Err(Error::UpperUnreachable); }
                budget.entry()?;
                let at_upper = j.test(upper)?;
                let entry = decode::decode(j.current(&mut budget)?, feed)?;
                let id = (entry.record.run_id.clone(), entry.record.seq);
                if let Some(&index) = ids.get(&id) {
                    if entries[index].record != entry.record { return Err(Error::Conflict); }
                } else { ids.insert(id, entries.len()); }
                entries.push(entry);
                if at_upper { break; }
            }
        }
    } else if after.is_some() { return Err(Error::UpperUnreachable); }
    finish(j, feed, after, upper, entries, &budget)
}

fn finish(j: &mut impl Access, feed: &Feed, after: Option<&Cursor>, upper: Option<String>, entries: Vec<Entry>, budget: &Budget<'_>) -> Result<Replay, Error> {
    budget.check()?;
    if j.process()? { return Err(Error::Invalidated); }
    if let Some(upper) = &upper { exact(j, upper, "upper")?; }
    if let Some(after) = after { exact(j, &after.opaque, "lower")?; }
    budget.check()?;
    Ok(Replay { entries, upper: upper.map(|opaque| Cursor { feed: feed.clone(), opaque }) })
}
