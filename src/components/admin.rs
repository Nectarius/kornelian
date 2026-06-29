use dioxus::prelude::*;
use crate::models::{Quiz, Question, AnswerChoice, Account};
use crate::services::*;
use bson::oid::ObjectId;

#[component]
pub fn QuizAdminView() -> Element {
    let mut quizzes = use_resource(move || get_quizzes());
    let mut title = use_signal(|| "".to_string());
    let mut description = use_signal(|| "".to_string());
    let mut working_questions = use_signal(Vec::<Question>::new);

    let add_question = move |_| {
        working_questions.write().push(Question {
            id: ObjectId::new(),
            text: "".to_string(),
            answer_choices: vec![
                AnswerChoice { id: ObjectId::new(), text: "Choice A".to_string(), correct_response: true },
                AnswerChoice { id: ObjectId::new(), text: "Choice B".to_string(), correct_response: false },
            ],
        });
    };

    let save_quiz = move |_| {
        spawn(async move {
            let new_quiz = Quiz {
                id: None,
                title: title.cloned(),
                description: description.cloned(),
                questions: working_questions.cloned(),
            };
            if let Ok(_) = create_quiz(new_quiz).await {
                title.set("".to_string());
                description.set("".to_string());
                working_questions.set(Vec::new());
                quizzes.restart();
            }
        });
    };

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 2rem;",
            h1 { style: "font-size: 1.75rem; font-weight: 700; color: #1e293b;", "Quiz Management Workspace" }
            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 2rem;",
                div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; display: flex; flex-direction: column; gap: 1rem;",
                    h2 { style: "font-size: 1.25rem; font-weight: 600;", "Create New Base Quiz Struct" }
                    div { style: "display: flex; flex-direction: column; gap: 0.25rem;",
                        label { style: "font-size: 0.875rem; font-weight: 500; color: #64748b;", "Quiz Title" }
                        input { style: "padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem;", value: "{title}", oninput: move |e| title.set(e.value()) }
                    }
                    div { style: "display: flex; flex-direction: column; gap: 0.25rem;",
                        label { style: "font-size: 0.875rem; font-weight: 500; color: #64748b;", "Description Summary" }
                        textarea { style: "padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem; min-height: 80px;", value: "{description}", oninput: move |e| description.set(e.value()) }
                    }
                    div { style: "margin-top: 1rem;",
                        h3 { style: "font-size: 1rem; font-weight: 600; margin-bottom: 0.5rem;", "Nested Questions Matrix ({working_questions.read().len()})" }
                        button { style: "background: #f1f5f9; border: 1px dashed #cbd5e1; padding: 0.5rem; width: 100%; border-radius: 0.375rem; font-weight: 500; cursor: pointer;", onclick: add_question, "➕ Inject Linear Question Object" }
                        for (q_idx, question) in working_questions.read().iter().enumerate() {
                            div { style: "border-left: 3px solid #38bdf8; padding-left: 0.75rem; margin-top: 1rem;",
                                input { style: "padding: 0.35rem; border: 1px solid #e2e8f0; font-weight: 500; width: 100%;", placeholder: "Question prompt text...", value: "{question.text}", oninput: move |e| { working_questions.write()[q_idx].text = e.value(); } }
                            }
                        }
                    }
                    button { style: "background: #2563eb; color: white; padding: 0.75rem; border-radius: 0.375rem; border: none; font-weight: 600; margin-top: 1rem; cursor: pointer;", onclick: save_quiz, "Commit Quiz Structure to Cluster" }
                }
                div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0;",
                    h2 { style: "font-size: 1.25rem; font-weight: 600; margin-bottom: 1rem;", "Active Repository Catalog" }
                    {
                        let list: Vec<_> = quizzes.read().as_ref()
                            .and_then(|r| r.as_ref().ok())
                            .map(|l| l.clone())
                            .unwrap_or_default();
                        rsx! {
                            for quiz in list {
                                div { style: "padding: 1rem; border: 1px solid #f1f5f9; background: #fafafa; border-radius: 0.25rem; display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;",
                                    div {
                                        h4 { style: "font-weight: 600; color: #1e293b;", "{quiz.title}" }
                                        span { style: "font-size: 0.75rem; background: #e0f2fe; color: #0369a1; padding: 0.15rem 0.5rem; border-radius: 9999px;", "{quiz.questions.len()} Qs" }
                                    }
                                    button { style: "background: #ef4444; color: white; border: none; padding: 0.35rem 0.75rem; border-radius: 0.25rem; cursor: pointer;", onclick: move |_| { let q_id = quiz.id.unwrap(); spawn(async move { if let Ok(_) = delete_quiz(q_id).await { quizzes.restart(); } }); }, "Delete" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AccountManagementView() -> Element {
    let mut accounts = use_resource(move || get_accounts());
    let mut target_email = use_signal(|| "".to_string());
    let mut selected_role = use_signal(|| "user".to_string());

    let execute_upsert = move |_| {
        spawn(async move {
            let fresh_account = Account { id: None, email: target_email.cloned(), roles: vec![selected_role.cloned()] };
            if let Ok(_) = upsert_account(fresh_account).await {
                target_email.set("".to_string());
                accounts.restart();
            }
        });
    };

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 2rem;",
            h1 { style: "font-size: 1.75rem; font-weight: 700;", "Identity Configuration Engine" }
            div { style: "display: grid; grid-template-columns: 1fr 2fr; gap: 2rem;",
                div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; display: flex; flex-direction: column; gap: 1rem;",
                    h3 { style: "font-weight: 600;", "Provision User / Assign Roles" }
                    input { style: "padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem;", placeholder: "user@domain.com", value: "{target_email}", oninput: move |e| target_email.set(e.value()) }
                    select { style: "padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem; background: white;", value: "{selected_role}", onchange: move |e| selected_role.set(e.value()),
                        option { value: "user", "Standard Tester Role" }
                        option { value: "admin", "System Administrator" }
                    }
                    button { style: "background: #10b981; color: white; border: none; padding: 0.65rem; font-weight: 600; border-radius: 0.25rem; cursor: pointer;", onclick: execute_upsert, "Synchronize Identity" }
                }
                div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0;",
                    if let Some(Ok(users)) = &*accounts.read() {
                        table { style: "width: 100%; text-align: left; border-collapse: collapse;",
                            thead { style: "background: #f8fafc; border-bottom: 2px solid #e2e8f0;",
                                tr { th { style: "padding: 0.75rem;", "Ident User Email" }, th { style: "padding: 0.75rem;", "Security Claims" } }
                            }
                            tbody {
                                for user in users {
                                    tr { style: "border-bottom: 1px solid #f1f5f9;",
                                        td { style: "padding: 0.75rem; font-weight: 500;", "{user.email}" }
                                        td { style: "padding: 0.75rem;", span { style: "background: #ecfdf5; color: #065f46; padding: 0.2rem 0.5rem; border-radius: 0.25rem; font-size: 0.8rem; font-weight: 600;", "{user.roles.join(\", \")}" } }
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

#[component]
pub fn SettingsPage() -> Element {
    let mut current_settings = use_resource(move || get_global_settings());
    let mut edit_question_count = use_signal(|| 10i32);
    let mut edit_quiz_choice = use_signal(|| String::new());

    // Sync signals when settings load
    if let Some(Ok(config)) = &*current_settings.read() {
        let qc = config.question_count;
        let qm = config.quiz_choice.clone();
        // Only initialise once (signals keep their value across re-renders)
        if *edit_question_count.read() == 10 && edit_quiz_choice.read().is_empty() {
            edit_question_count.set(qc);
            edit_quiz_choice.set(qm);
        }
    }

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 2rem; max-width: 600px;",
            h1 { style: "font-size: 1.75rem; font-weight: 700;", "Global Engine Core Configuration" }
            div { style: "background: white; padding: 2rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; display: flex; flex-direction: column; gap: 1.25rem;",
                div {
                    label { style: "display: block; font-size: 0.875rem; font-weight: 600; margin-bottom: 0.25rem;", "Enforced Max Question Count" }
                    input { style: "width: 100%; padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem;", r#type: "number", value: "{edit_question_count}", oninput: move |e| { if let Ok(val) = e.value().parse::<i32>() { edit_question_count.set(val); } } }
                }
                div {
                    label { style: "display: block; font-size: 0.875rem; font-weight: 600; margin-bottom: 0.25rem;", "Global Operation Mode" }
                    input { style: "width: 100%; padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 0.25rem;", value: "{edit_quiz_choice}", oninput: move |e| { edit_quiz_choice.set(e.value()); } }
                }
                button {
                    style: "background: #4f46e5; color: white; border: none; padding: 0.75rem; border-radius: 0.375rem; font-weight: 600; cursor: pointer;",
                    onclick: move |_| {
                        let qc = *edit_question_count.read();
                        let qm = edit_quiz_choice.read().clone();
                        let settings_snap = current_settings.read();
                        if let Some(Ok(cfg)) = &*settings_snap {
                            let payload = crate::models::Settings {
                                id: cfg.id,
                                current: cfg.current.clone(),
                                applied_at: cfg.applied_at,
                                applied_by: cfg.applied_by.clone(),
                                question_count: qc,
                                quiz_choice: qm,
                            };
                            drop(settings_snap);
                            spawn(async move {
                                if let Ok(_) = update_global_settings(payload).await {
                                    current_settings.restart();
                                }
                            });
                        }
                    },
                    "Broadcast Updated Configurations"
                }
            }
        }
    }
}
