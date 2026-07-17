use dioxus::prelude::*;
use crate::models::Note;
use crate::services::*;
use bson::oid::ObjectId;

#[component]
pub fn NotesView() -> Element {
    let nav = navigator();
    let current_user = use_resource(move || get_current_user());
    let mut notes = use_resource(move || async move {
        match get_current_user().await {
            Ok(Some(user)) => {
                if let Some(account_id) = user.id {
                    get_notes(account_id).await
                } else {
                    Ok(Vec::new())
                }
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    });

    let mut new_title = use_signal(|| String::new());
    let mut new_content = use_signal(|| String::new());
    let mut editing_id = use_signal(|| Option::<ObjectId>::None);
    let mut edit_title = use_signal(|| String::new());
    let mut edit_content = use_signal(|| String::new());

    match current_user.read().as_ref() {
        None => {
            return rsx! {
                div { style: "text-align: center; padding: 3rem; color: #64748b;",
                    p { "Loading user session..." }
                }
            };
        }
        Some(Err(err)) => {
            return rsx! {
                div { style: "text-align: center; padding: 3rem; color: #dc2626;",
                    p { "Failed to load user session: {err}" }
                }
            };
        }
        Some(Ok(None)) => {
            nav.push("/login");
            return rsx! {
                div { style: "text-align: center; padding: 3rem; color: #64748b;",
                    p { "Redirecting to login..." }
                }
            };
        }
        Some(Ok(Some(user))) => {
            let account_id = match user.id {
                Some(id) => id,
                None => {
                    return rsx! {
                        div { style: "text-align: center; padding: 3rem; color: #dc2626;",
                            p { "Your account is missing an identifier. Please contact support." }
                        }
                    };
                }
            };

            let create_note_handler = move |_| {
                let title = new_title.cloned();
                let content = new_content.cloned();
                if title.trim().is_empty() {
                    return;
                }
                spawn(async move {
                    if create_note(account_id, title, content).await.is_ok() {
                        new_title.set(String::new());
                        new_content.set(String::new());
                        notes.restart();
                    }
                });
            };

            rsx! {
                div { style: "display: flex; flex-direction: column; gap: 2rem;",
                    h1 { style: "font-size: 1.75rem; font-weight: 700;", "User Notes" }
                    p { style: "color: #64748b;", "Personal notes for {user.email}" }

                    div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; display: flex; flex-direction: column; gap: 1rem;",
                        h2 { style: "font-size: 1.25rem; font-weight: 600;", "Create Note" }
                        div { style: "display: flex; flex-direction: column; gap: 0.25rem;",
                            label { style: "font-size: 0.875rem; font-weight: 500; color: #64748b;", "Title" }
                            input {
                                style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem;",
                                placeholder: "Note title",
                                value: "{new_title}",
                                oninput: move |e| new_title.set(e.value()),
                            }
                        }
                        div { style: "display: flex; flex-direction: column; gap: 0.25rem;",
                            label { style: "font-size: 0.875rem; font-weight: 500; color: #64748b;", "Content" }
                            textarea {
                                style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem; min-height: 100px;",
                                placeholder: "Write your note here...",
                                value: "{new_content}",
                                oninput: move |e| new_content.set(e.value()),
                            }
                        }
                        button {
                            style: "background: #2563eb; color: white; padding: 0.75rem; border-radius: 0.375rem; border: none; font-weight: 600; cursor: pointer; align-self: flex-start;",
                            onclick: create_note_handler,
                            "Create Note"
                        }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 1rem;",
                        h2 { style: "font-size: 1.25rem; font-weight: 600;", "Your Notes" }
                        {
                            let note_list: Vec<Note> = notes.read().as_ref()
                                .and_then(|r| r.as_ref().ok())
                                .cloned()
                                .unwrap_or_default();

                            if note_list.is_empty() {
                                rsx! {
                                    div { style: "background: white; padding: 2rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; text-align: center; color: #94a3b8;",
                                        p { "No notes yet. Create your first note above." }
                                    }
                                }
                            } else {
                                rsx! {
                                    for note in note_list {
                                        {
                                            let note_id = note.id.unwrap_or_else(ObjectId::new);
                                            let is_editing = *editing_id.read() == Some(note_id);

                                            if is_editing {
                                                rsx! {
                                                    div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 2px solid #2563eb; display: flex; flex-direction: column; gap: 1rem;",
                                                        input {
                                                            style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem; font-weight: 600;",
                                                            value: "{edit_title}",
                                                            oninput: move |e| edit_title.set(e.value()),
                                                        }
                                                        textarea {
                                                            style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem; min-height: 80px;",
                                                            value: "{edit_content}",
                                                            oninput: move |e| edit_content.set(e.value()),
                                                        }
                                                        div { style: "display: flex; flex-wrap: wrap; gap: 0.5rem;",
                                                            button {
                                                                style: "background: #10b981; color: white; border: none; padding: 0.5rem 1rem; border-radius: 0.25rem; cursor: pointer; font-weight: 600;",
                                                                onclick: move |_| {
                                                                    let title = edit_title.cloned();
                                                                    let content = edit_content.cloned();
                                                                    spawn(async move {
                                                                        if update_note(account_id, note_id, title, content).await.is_ok() {
                                                                            editing_id.set(None);
                                                                            notes.restart();
                                                                        }
                                                                    });
                                                                },
                                                                "Save"
                                                            }
                                                            button {
                                                                style: "background: #f1f5f9; color: #475569; border: 1px solid #cbd5e1; padding: 0.5rem 1rem; border-radius: 0.25rem; cursor: pointer;",
                                                                onclick: move |_| editing_id.set(None),
                                                                "Cancel"
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                let updated_label = note.updated_at.format("%Y-%m-%d %H:%M UTC").to_string();
                                                rsx! {
                                                    div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0;",
                                                        div { style: "display: flex; flex-wrap: wrap; gap: 0.5rem; justify-content: space-between; align-items: flex-start; margin-bottom: 0.75rem;",
                                                            h3 { style: "font-weight: 600; font-size: 1.1rem;", "{note.title}" }
                                                            div { style: "display: flex; gap: 0.5rem;",
                                                                button {
                                                                    style: "background: #f1f5f9; color: #2563eb; border: 1px solid #cbd5e1; padding: 0.35rem 0.75rem; border-radius: 0.25rem; cursor: pointer; font-size: 0.875rem;",
                                                                    onclick: move |_| {
                                                                        edit_title.set(note.title.clone());
                                                                        edit_content.set(note.content.clone());
                                                                        editing_id.set(Some(note_id));
                                                                    },
                                                                    "Edit"
                                                                }
                                                                button {
                                                                    style: "background: #ef4444; color: white; border: none; padding: 0.35rem 0.75rem; border-radius: 0.25rem; cursor: pointer; font-size: 0.875rem;",
                                                                    onclick: move |_| {
                                                                        spawn(async move {
                                                                            if delete_note(account_id, note_id).await.is_ok() {
                                                                                notes.restart();
                                                                            }
                                                                        });
                                                                    },
                                                                    "Delete"
                                                                }
                                                            }
                                                        }
                                                        p { style: "color: #475569; white-space: pre-wrap; margin-bottom: 0.5rem;", "{note.content}" }
                                                        span { style: "font-size: 0.75rem; color: #94a3b8;", "Updated: {updated_label}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
