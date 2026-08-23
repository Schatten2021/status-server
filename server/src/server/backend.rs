use crate::server::backend_state::{AttributeChange, GlobalState, Task};
use crate::{AttributeEdit, AttributeValueChange, Notification, NotificationReason};
use std::sync::Arc;

#[expect(clippy::needless_pass_by_value, reason="this function is semantically taking ownership. Also, we don't need it at the other place.")]
pub fn backend_thread(channel: std::sync::mpsc::Receiver<Task>, backend: Arc<GlobalState>, tokio_handle: tokio::runtime::Handle) {
    while let Ok(task) = channel.recv() {
        match task {
            Task::SendNotification(notification) => {
                let backend = backend.clone();
                // use tokio here to ensure that the components have access to Tokio's async runtime.
                tokio_handle.spawn(async move {
                    // SAFETY: The correctness of the type is ensured at creation time of `NotificationProviderInfo`.
                    backend.components.read()
                        .entries()
                        .filter_map(|(component, data)| data.notification_provider_info.as_ref()
                            .zip(Some(component)))
                        .for_each(|(info, component)| unsafe { (info.notify)(component, notification.clone())});
                });
            }
            Task::EditAttribute(edit) => {
                match edit.change {
                    AttributeChange::Set(new) => {
                        let mut state_lock = backend.states.write();
                        let element = if let Some(element) = state_lock.get_mut(&edit.element_id) { element } else {
                            assert!(state_lock.insert(edit.element_id.clone(), crate::State::new()).is_none());
                            backend.tasks.send(Task::SendNotification(Notification {
                                component_id: edit.causing_component.to_string(),
                                element_id: edit.element_id.clone(),
                                reason: NotificationReason::NewElement(false),
                            }))
                                .expect("receiving channel dropped even though the receiver thread is still active and kicking? That doesn't make sense...");
                            state_lock.get_mut(&edit.element_id)
                                .expect("just inserted it ?!?")
                        };
                        let old_value = element.attributes.insert(edit.attribute_id.clone(), new.clone());
                        drop(state_lock);
                        let task = Task::SendNotification(Notification {
                            component_id: edit.causing_component.to_string(),
                            element_id: edit.element_id,
                            reason: NotificationReason::AttributeEdit(AttributeEdit {
                                id: edit.attribute_id,
                                change: match old_value {
                                    None => AttributeValueChange::Create(new),
                                    Some(old) => AttributeValueChange::Edit(old, new),
                                }
                            }),
                        });
                        backend.tasks.send(task)
                            .expect("receiving channel dropped even though the receiver thread is still active and kicking? That doesn't make sense...");
                    }
                    AttributeChange::Delete => {
                        let mut state_lock = backend.states.write();
                        let Some(element) = state_lock.get_mut(&edit.element_id) else { return; /* deleted non-existent attribute*/ };
                        let Some(old_value) = element.attributes.remove(&edit.attribute_id) else { return /* deleted non-existent attribute*/ };
                        drop(state_lock);
                        let task = Task::SendNotification(Notification {
                            component_id: edit.causing_component.to_string(),
                            element_id: edit.element_id,
                            reason: NotificationReason::AttributeEdit(AttributeEdit {
                                id: edit.attribute_id,
                                change: AttributeValueChange::Delete(old_value),
                            }),
                        });
                        backend.tasks.send(task)
                            .expect("receiving channel dropped even though the receiver thread is still active and kicking? That doesn't make sense...");
                    }
                }
            }
            Task::EditOnlineState(causing_component, element_id, state) => {
                let mut state_lock = backend.states.write();
                let reason = if let Some(element) = state_lock.get_mut(&element_id) {
                    if element.online == state { return; }
                    element.online = state;
                    NotificationReason::OnlineStatusChanged(state)
                } else {
                    assert!(state_lock.insert(element_id.clone(), crate::State::with_online(state)).is_none());
                    NotificationReason::NewElement(state)
                };
                backend.tasks.send(Task::SendNotification(Notification {
                    component_id: causing_component.to_string(),
                    element_id,
                    reason,
                }))
                    .expect("receiving channel dropped even though the receiver thread is still active and kicking? That doesn't make sense...");
            }
        }
    }
}
