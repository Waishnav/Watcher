//! Sway foreground adapter. IPC subscription is event driven; no focus control.
use crate::CollectorEvent;
use std::{sync::mpsc::SyncSender, thread, time::Duration};
use swayipc::{Connection, Event, EventType, WindowChange};
use watcher_core::Observation;

fn app(node: &swayipc::Node) -> Option<String> {
    node.app_id
        .clone()
        .or_else(|| {
            node.window_properties
                .as_ref()
                .and_then(|p| p.class.clone())
        })
        .and_then(|s| {
            let name = s
                .chars()
                .filter(|c| !c.is_control())
                .take(128)
                .collect::<String>();
            (!name.trim().is_empty()).then_some(name)
        })
}

pub fn spawn_focus(tx: SyncSender<CollectorEvent>) {
    thread::spawn(move || {
        loop {
            let result = listen(&tx);
            let reason = result
                .err()
                .unwrap_or_else(|| "Sway event stream disconnected".into());
            if tx
                .send(CollectorEvent::failure(
                    Observation::FocusAvailable(false),
                    reason,
                ))
                .is_err()
            {
                return;
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}

fn listen(tx: &SyncSender<CollectorEvent>) -> Result<(), String> {
    // Subscribe first, then snapshot with an independent IPC connection so a
    // concurrent user focus change is eventually reconciled by a stream event.
    let stream = Connection::new()
        .map_err(|e| e.to_string())?
        .subscribe([EventType::Window, EventType::Workspace, EventType::Shutdown])
        .map_err(|e| e.to_string())?;
    let tree = Connection::new()
        .map_err(|e| e.to_string())?
        .get_tree()
        .map_err(|e| e.to_string())?;
    tx.send(CollectorEvent::new(Observation::FocusAvailable(true)))
        .map_err(|e| e.to_string())?;
    let mut focused_id = tree.find_as_ref(|n| n.focused).map(|n| n.id);
    let focused = tree.find_as_ref(|n| n.focused).and_then(app);
    tx.send(CollectorEvent::new(Observation::Focus(focused)))
        .map_err(|e| e.to_string())?;
    for event in stream {
        match event.map_err(|e| e.to_string())? {
            Event::Window(event) if event.change == WindowChange::Focus => {
                focused_id = Some(event.container.id);
                tx.send(CollectorEvent::new(Observation::Focus(app(
                    &event.container
                ))))
                .map_err(|e| e.to_string())?;
            }
            Event::Window(event)
                if event.change == WindowChange::Close
                    && focused_id == Some(event.container.id) =>
            {
                focused_id = None;
                tx.send(CollectorEvent::new(Observation::Focus(None)))
                    .map_err(|e| e.to_string())?;
            }
            Event::Workspace(event) => {
                if event.current.is_none() {
                    continue;
                }
                // A workspace transition can momentarily have no application.
                let tree = Connection::new()
                    .map_err(|e| e.to_string())?
                    .get_tree()
                    .map_err(|e| e.to_string())?;
                focused_id = tree.find_as_ref(|n| n.focused).map(|n| n.id);
                tx.send(CollectorEvent::new(Observation::Focus(
                    tree.find_as_ref(|n| n.focused).and_then(app),
                )))
                .map_err(|e| e.to_string())?;
            }
            Event::Shutdown(_) => return Err("Sway is shutting down".into()),
            _ => {}
        }
    }
    Err("Sway event stream ended".into())
}
