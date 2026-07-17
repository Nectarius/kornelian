use dioxus::prelude::*;
use crate::models::{Quiz, Question, AnswerChoice, Account};
use crate::services::*;
use bson::oid::ObjectId;

use crate::components::auth::{use_auth, is_admin};

#[component]
pub fn QuizAdminView() -> Element {
    let auth = use_auth();
    let auth_state = auth.read();
    
    // Authorization check
    if let Some(user) = &auth_state.user {
        if !is_admin(&user.email) {
            return rsx! {
                div { style: "padding: 2rem; color: #ef4444; text-align: center; font-size: 1.25rem; font-weight: 600;",
                    "Unauthorized: Admin access required."
                }
            };
        }
    } else {
        return rsx! { div { "Loading..." } };
    }

    let mut quizzes = use_resource(move || get_quizzes());
    let mut title = use_signal(|| "".to_string());
    let mut description = use_signal(|| "".to_string());
    let mut working_questions = use_signal(Vec::<Question>::new);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let add_question = move |_| {
        working_questions.write().push(Question {
            id: ObjectId::new(),
            text: "".to_string(),
            answer_choices: vec![
                AnswerChoice { id: ObjectId::new(), text: "Choice A".to_string(), correct_response: true },
                AnswerChoice { id: ObjectId::new(), text: "Choice B".to_string(), correct_response: false },
                AnswerChoice { id: ObjectId::new(), text: "Choice C".to_string(), correct_response: false },
                AnswerChoice { id: ObjectId::new(), text: "Choice D".to_string(), correct_response: false },
            ],
        });
    };

    let save_quiz = move |_| {
        error_msg.set(None);
        let q_list = working_questions.cloned();
        
        // Client-side validations
        if title.read().trim().is_empty() {
            error_msg.set(Some("Quiz title cannot be empty".to_string()));
            return;
        }
        if q_list.is_empty() {
            error_msg.set(Some("Quiz must have at least one question".to_string()));
            return;
        }
        for (idx, q) in q_list.iter().enumerate() {
            if q.text.trim().is_empty() {
                error_msg.set(Some(format!("Question {} cannot have empty text", idx + 1)));
                return;
            }
            let correct_count = q.answer_choices.iter().filter(|c| c.correct_response).count();
            if correct_count != 1 {
                error_msg.set(Some(format!("Question {} must have exactly one correct answer choice", idx + 1)));
                return;
            }
            if q.answer_choices.len() != 4 {
                error_msg.set(Some(format!("Question {} must have exactly 4 choices", idx + 1)));
                return;
            }
        }

        spawn(async move {
            let new_quiz = Quiz {
                id: None,
                title: title.cloned(),
                description: description.cloned(),
                questions: working_questions.cloned(),
            };
            match create_quiz(new_quiz).await {
                Ok(_) => {
                    title.set("".to_string());
                    description.set("".to_string());
                    working_questions.set(Vec::new());
                    quizzes.restart();
                }
                Err(e) => {
                    error_msg.set(Some(format!("Server error: {}", e)));
                }
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
                                input { style: "padding: 0.35rem; border: 1px solid #e2e8f0; font-weight: 500; width: 100%; margin-bottom: 0.5rem;", placeholder: "Question prompt text...", value: "{question.text}", oninput: move |e| { working_questions.write()[q_idx].text = e.value(); } }
                                div { style: "margin-left: 0.5rem; display: flex; flex-direction: column; gap: 0.25rem;",
                                    for (c_idx, choice) in question.answer_choices.iter().enumerate() {
                                        div { style: "display: flex; align-items: center; gap: 0.5rem; padding: 0.25rem; background: #f8fafc; border-radius: 0.25rem;",
                                            input { 
                                                r#type: "radio", 
                                                name: format!("correct_q{}", q_idx),
                                                checked: choice.correct_response,
                                                onchange: move |_e| {
                                                    for (idx, c) in working_questions.write()[q_idx].answer_choices.iter_mut().enumerate() {
                                                        c.correct_response = idx == c_idx;
                                                    }
                                                }
                                            }
                                            input { 
                                                style: "flex: 1; padding: 0.25rem; border: 1px solid #e2e8f0; border-radius: 0.25rem; font-size: 0.875rem;",
                                                value: "{choice.text}",
                                                oninput: move |e| { working_questions.write()[q_idx].answer_choices[c_idx].text = e.value(); }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(msg) = error_msg.read().as_ref() {
                        div { style: "background: #fee2e2; color: #b91c1c; padding: 0.75rem; border-radius: 0.375rem; font-size: 0.875rem; font-weight: 500; border: 1px solid #fca5a5; margin-top: 1rem;",
                            "{msg}"
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
    let auth = use_auth();
    let auth_state = auth.read();
    
    // Authorization check
    if let Some(user) = &auth_state.user {
        if !is_admin(&user.email) {
            return rsx! {
                div { style: "padding: 2rem; color: #ef4444; text-align: center; font-size: 1.25rem; font-weight: 600;",
                    "Unauthorized: Admin access required."
                }
            };
        }
    } else {
        return rsx! { div { "Loading..." } };
    }

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

#[component]
pub fn AllResultsSummaryView() -> Element {
    let summaries_res = use_resource(move || get_all_results_summary());

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 2rem; max-width: 900px; margin: 0 auto;",
            h1 { style: "font-size: 1.75rem; font-weight: 700; color: #1e293b;", "Global User Summary Platform" }
            
            {
                let summaries: Vec<_> = summaries_res.read().as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .map(|l| l.clone())
                    .unwrap_or_default();

                let total_quizzes = summaries.len();
                let total_correct: i32 = summaries.iter().map(|s| s.score_correct).sum();
                let total_questions: i32 = summaries.iter().map(|s| s.score_total).sum();
                let total_timeouts: i32 = summaries.iter().map(|s| s.timed_out_count).sum();
                
                let accuracy = if total_questions > 0 {
                    (total_correct as f64 / total_questions as f64 * 100.0) as i32
                } else {
                    0
                };

                // Find most successful user
                use std::collections::HashMap;
                let mut user_stats: HashMap<String, (i32, i32, i32)> = HashMap::new();
                for s in &summaries {
                    let entry = user_stats.entry(s.user_email.clone()).or_insert((0, 0, 0));
                    entry.0 += s.score_correct; // correct
                    entry.1 += s.score_total;   // total
                    entry.2 += s.timed_out_count; // timeouts
                }
                
                let mut best_user: Option<(String, i32, i32, i32)> = None;
                for (email, (correct, total, timeouts)) in user_stats {
                    if let Some((_, best_correct, _, best_timeouts)) = best_user.clone() {
                        if correct > best_correct {
                            best_user = Some((email.clone(), correct, total, timeouts));
                        } else if correct == best_correct {
                            // If tied on correct, choose the one with fewest timeouts
                            if timeouts < best_timeouts {
                                best_user = Some((email.clone(), correct, total, timeouts));
                            }
                        }
                    } else {
                        best_user = Some((email.clone(), correct, total, timeouts));
                    }
                }

                // Find top achiever per quiz
                let mut quiz_top_achievers: HashMap<String, (String, i32, i32, i32)> = HashMap::new();
                for s in &summaries {
                    let title = s.quiz_title.clone();
                    let email = s.user_email.clone();
                    let correct = s.score_correct;
                    let total = s.score_total;
                    let timeouts = s.timed_out_count;
                    
                    if let Some((_, best_correct, _, best_timeouts)) = quiz_top_achievers.get(&title) {
                        if correct > *best_correct {
                            quiz_top_achievers.insert(title, (email, correct, total, timeouts));
                        } else if correct == *best_correct {
                            if timeouts < *best_timeouts {
                                quiz_top_achievers.insert(title, (email, correct, total, timeouts));
                            }
                        }
                    } else {
                        quiz_top_achievers.insert(title, (email, correct, total, timeouts));
                    }
                }

                rsx! {
                    div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 1.5rem;",
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
                            h3 { style: "font-size: 0.75rem; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "Total Global Attempts" }
                            p { style: "font-size: 2rem; font-weight: 800; color: #1e293b; margin-top: 0.5rem;", "{total_quizzes}" }
                        }
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
                            h3 { style: "font-size: 0.75rem; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "Total Correct" }
                            p { style: "font-size: 2rem; font-weight: 800; color: #10b981; margin-top: 0.5rem;", "{total_correct} / {total_questions}" }
                        }
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
                            h3 { style: "font-size: 0.75rem; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "Global Accuracy" }
                            p { style: "font-size: 2rem; font-weight: 800; color: #3b82f6; margin-top: 0.5rem;", "{accuracy}%" }
                        }
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
                            h3 { style: "font-size: 0.75rem; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "Global Timeouts" }
                            p { style: "font-size: 2rem; font-weight: 800; color: #ef4444; margin-top: 0.5rem;", "{total_timeouts}" }
                        }
                    }

                    if let Some((email, correct, total, timeouts)) = best_user {
                        div { style: "background: linear-gradient(135deg, #10b981 0%, #059669 100%); padding: 1.5rem; border-radius: 0.5rem; color: white; box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1); margin-top: 1rem; display: flex; justify-content: space-between; align-items: center;",
                            div {
                                h2 { style: "font-size: 1.25rem; font-weight: 700; margin: 0 0 0.25rem 0;", "🏆 Top Achiever" }
                                p { style: "margin: 0; font-size: 0.95rem; opacity: 0.9;", "Based on total correct answers and fewest timeouts" }
                            }
                            div { style: "text-align: right;",
                                div { style: "font-size: 1.25rem; font-weight: 800;", "{email}" }
                                div { style: "font-size: 0.9rem; font-weight: 600; background: rgba(255,255,255,0.2); padding: 0.25rem 0.5rem; border-radius: 0.25rem; display: inline-block; margin-top: 0.5rem;",
                                    "Score: {correct}/{total} | Timeouts: {timeouts}"
                                }
                            }
                        }
                    }

                    if !quiz_top_achievers.is_empty() {
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05); margin-top: 1rem;",
                            h2 { style: "font-size: 1.25rem; font-weight: 700; color: #0f172a; margin-bottom: 1.5rem;", "🏅 Top Achiever by Quest" }
                            div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 1rem;",
                                for (title, (email, correct, total, timeouts)) in quiz_top_achievers {
                                    div { key: "{title}", style: "padding: 1rem; border: 1px solid #e2e8f0; border-radius: 0.375rem; background: #f8fafc; border-left: 4px solid #3b82f6;",
                                        h4 { style: "font-size: 1.05rem; font-weight: 700; color: #1e293b; margin-bottom: 0.5rem;", "{title}" }
                                        p { style: "font-size: 0.9rem; color: #475569; margin: 0; font-weight: 600;", "{email}" }
                                        p { style: "font-size: 0.8rem; color: #64748b; margin-top: 0.25rem;", "Score: {correct}/{total}  |  Timeouts: {timeouts}" }
                                    }
                                }
                            }
                        }
                    }

                    div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05); margin-top: 1rem;",
                        h2 { style: "font-size: 1.25rem; font-weight: 700; color: #0f172a; margin-bottom: 1.5rem;", "All User Attempts Log" }
                        if summaries.is_empty() {
                            div { style: "text-align: center; padding: 3rem; color: #64748b; font-weight: 500;", "No records across the platform." }
                        } else {
                            div { style: "display: flex; flex-direction: column; gap: 1rem;",
                                for record in summaries {
                                    {
                                        let record_accuracy = if record.score_total > 0 {
                                            (record.score_correct as f64 / record.score_total as f64 * 100.0) as i32
                                        } else {
                                            0
                                        };
                                        let progress_color = if record_accuracy >= 80 { "#10b981" } else if record_accuracy >= 50 { "#f59e0b" } else { "#ef4444" };
                                        let formatted_date = record.completed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                                        rsx! {
                                            div { key: "{record.id}", style: "padding: 1.25rem; border: 1px solid #e2e8f0; border-radius: 0.375rem; display: flex; flex-direction: column; gap: 0.75rem;",
                                                div { style: "display: flex; justify-content: space-between; align-items: flex-start;",
                                                    div {
                                                        h4 { style: "font-size: 1.1rem; font-weight: 600; color: #1e293b;", "{record.quiz_title}" }
                                                        p { style: "font-size: 0.8rem; color: #94a3b8; margin-top: 0.25rem;", 
                                                            "By: " span { style: "color: #3b82f6; font-weight: 600;", "{record.user_email}" } "  |  Completed: {formatted_date}"
                                                        }
                                                    }
                                                    div { style: "display: flex; align-items: center; gap: 0.75rem;",
                                                        span { style: "background: #f1f5f9; padding: 0.25rem 0.50rem; border-radius: 0.25rem; font-size: 0.875rem; font-weight: 600; color: #475569;",
                                                            "Score: {record.score_correct} / {record.score_total}"
                                                        }
                                                        if record.timed_out_count > 0 {
                                                            span { style: "background: #fee2e2; color: #991b1b; padding: 0.25rem 0.50rem; border-radius: 0.25rem; font-size: 0.875rem; font-weight: 600;",
                                                                "⏱ {record.timed_out_count} Timeouts"
                                                            }
                                                        }
                                                    }
                                                }
                                                div { style: "display: flex; align-items: center; gap: 1rem;",
                                                    div { style: "flex-grow: 1; height: 8px; background: #e2e8f0; border-radius: 9999px; overflow: hidden;",
                                                        div { style: "width: {record_accuracy}%; height: 100%; background: {progress_color}; border-radius: 9999px;" }
                                                    }
                                                    span { style: "font-size: 0.875rem; font-weight: 700; color: {progress_color}; min-width: 40px; text-align: right;",
                                                        "{record_accuracy}%"
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
