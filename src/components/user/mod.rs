use dioxus::prelude::*;
use crate::models::{Quiz, QuizAnswer, Answer};
use crate::services::*;
use bson::oid::ObjectId;
use chrono::Utc;
use std::time::Duration;

mod notes;
pub use notes::NotesView;

#[component]
pub fn Dashboard() -> Element {
    let quizzes = use_resource(move || get_quizzes());
    let submissions = use_resource(move || get_submissions(None));

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 2rem;",
            h1 { style: "font-size: 1.75rem; font-weight: 700;", "Cluster Dashboard" }
            div { style: "display: grid; grid-template-columns: repeat(3, 1fr); gap: 1.5rem;",
                div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0;",
                    h3 { style: "font-size: 0.875rem; color: #64748b; font-weight: 600;", "ACTIVE QUIZZES" }
                    p { style: "font-size: 2rem; font-weight: 700; color: #2563eb;", "{quizzes.read().as_ref().map_or(0, |r| r.as_ref().map_or(0, |l| l.len()))}" }
                }
                div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0;",
                    h3 { style: "font-size: 0.875rem; color: #64748b; font-weight: 600;", "TOTAL SUBMISSIONS" }
                    p { style: "font-size: 2rem; font-weight: 700; color: #10b981;", "{submissions.read().as_ref().map_or(0, |r| r.as_ref().map_or(0, |l| l.len()))}" }
                }
            }
        }
    }
}

async fn async_sleep(seconds: u64) {
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new((seconds * 1000) as u32).await;
    }
    #[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
    {
        tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
    }
    #[cfg(all(not(target_arch = "wasm32"), not(feature = "server")))]
    {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    }
}

#[component]
pub fn TakeQuizSelection() -> Element {
    let quizzes = use_resource(move || get_quizzes());
    let current_user = use_resource(move || get_current_user());
    let mut active_quiz = use_signal(|| Option::<Quiz>::None);
    let mut current_question_idx = use_signal(|| 0);
    let mut chosen_answers = use_signal(Vec::<Answer>::new);
    let mut runtime_start_time = use_signal(|| Utc::now());
    let mut quiz_submitted = use_signal(|| false);
    let mut timer_seconds = use_signal(|| 15i32);
    let mut timer_active = use_signal(|| false);

    use_effect(move || {
        let active = *timer_active.read();
        let seconds = *timer_seconds.read();
        
        if active && seconds > 0 {
            spawn(async move {
                async_sleep(1).await;
                if *timer_active.peek() && *timer_seconds.peek() == seconds {
                    timer_seconds.set(seconds - 1);
                }
            });
        } else if active && seconds == 0 {
            timer_active.set(false);
            
            // Trigger timeout and move to the next question / submit quiz
            let quiz = active_quiz.peek().clone().unwrap();
            let idx = *current_question_idx.peek();
            let q_id = quiz.questions[idx].id;
            
            let mut answers_vec = chosen_answers.peek().clone();
            let was_answered = answers_vec.iter().any(|a| a.question_id == q_id);
            if !was_answered {
                let timeout_answer = Answer { 
                    question_id: q_id, 
                    text: "No answer - timed out".to_string(), 
                    started: *runtime_start_time.peek(), 
                    completed: Utc::now(),
                    timed_out: true 
                };
                answers_vec.retain(|a| a.question_id != q_id);
                answers_vec.push(timeout_answer);
                chosen_answers.set(answers_vec.clone());
            }
            
            if idx + 1 < quiz.questions.len() {
                current_question_idx.set(idx + 1);
                runtime_start_time.set(Utc::now());
                timer_seconds.set(15);
                timer_active.set(true);
            } else {
                let user_snapshot = current_user.peek().clone();
                spawn(async move {
                    let (account_id, email) = match user_snapshot {
                        Some(Ok(Some(user))) => (user.id.unwrap(), user.email),
                        _ => (ObjectId::new(), "anonymous@domain.com".to_string()),
                    };
                    
                    let submission_doc = QuizAnswer {
                        id: None, 
                        quiz_id: quiz.id.unwrap(), 
                        account_id,
                        email,
                        quiz_title: quiz.title,
                        answers: answers_vec,
                    };
                    if let Ok(_) = submit_quiz_answer(submission_doc).await { 
                        quiz_submitted.set(true); 
                    }
                });
            }
        }
    });

    let mut start_quiz = move |quiz: Quiz| {
        active_quiz.set(Some(quiz));
        current_question_idx.set(0);
        chosen_answers.set(Vec::new());
        runtime_start_time.set(Utc::now());
        quiz_submitted.set(false);
        timer_seconds.set(15);
        timer_active.set(true);
    };

    let mut select_choice = move |q_id: ObjectId, text: String| {
        let answer = Answer { question_id: q_id, text: text.clone(), started: *runtime_start_time.peek(), completed: Utc::now(), timed_out: false };
        let mut answers_vec = chosen_answers.peek().clone();
        answers_vec.retain(|a| a.question_id != q_id);
        answers_vec.push(answer);
        chosen_answers.set(answers_vec.clone());
        timer_active.set(false);

        // Auto-advance or submit
        let quiz = active_quiz.peek().clone().unwrap();
        let idx = *current_question_idx.peek();
        if idx + 1 < quiz.questions.len() {
            current_question_idx.set(idx + 1);
            runtime_start_time.set(Utc::now());
            timer_seconds.set(15);
            timer_active.set(true);
        } else {
            let user_snapshot = current_user.peek().clone();
            spawn(async move {
                let (account_id, email) = match user_snapshot {
                    Some(Ok(Some(user))) => (user.id.unwrap(), user.email),
                    _ => (ObjectId::new(), "anonymous@domain.com".to_string()),
                };
                
                let submission_doc = QuizAnswer {
                    id: None, 
                    quiz_id: quiz.id.unwrap(), 
                    account_id,
                    email,
                    quiz_title: quiz.title,
                    answers: answers_vec,
                };
                if let Ok(_) = submit_quiz_answer(submission_doc).await { 
                    quiz_submitted.set(true); 
                }
            });
        }
    };

    let mut next_question = move |_| {
        let quiz = active_quiz.read().clone().unwrap();
        let idx = *current_question_idx.read();
        let q_id = quiz.questions[idx].id;
        
        // Check if question was answered
        let was_answered = chosen_answers.read().iter().any(|a| a.question_id == q_id);
        
        if !was_answered {
            // Mark as timed out
            let timeout_answer = Answer { 
                question_id: q_id, 
                text: "No answer - timed out".to_string(), 
                started: *runtime_start_time.read(), 
                completed: Utc::now(),
                timed_out: true 
            };
            let mut answers_vec = chosen_answers.read().clone();
            answers_vec.retain(|a| a.question_id != q_id);
            answers_vec.push(timeout_answer);
            chosen_answers.set(answers_vec);
        }
        
        if idx + 1 < quiz.questions.len() {
            current_question_idx.set(idx + 1);
            runtime_start_time.set(Utc::now());
            timer_seconds.set(15);
            timer_active.set(true);
        }
    };

    let handle_next_click = move |_| {
        next_question(());
    };

    let push_results = move |_| {
        let current_quiz = active_quiz.read().clone().unwrap();
        let user_snapshot = current_user.read().clone();
        
        let idx = *current_question_idx.read();
        let q_id = current_quiz.questions[idx].id;
        
        let was_answered = chosen_answers.read().iter().any(|a| a.question_id == q_id);
        let mut answers_vec = chosen_answers.read().clone();
        if !was_answered {
            let timeout_answer = Answer { 
                question_id: q_id, 
                text: "No answer - timed out".to_string(), 
                started: *runtime_start_time.read(), 
                completed: Utc::now(),
                timed_out: true 
            };
            answers_vec.retain(|a| a.question_id != q_id);
            answers_vec.push(timeout_answer);
            chosen_answers.set(answers_vec.clone());
        }
        
        timer_active.set(false);
        
        spawn(async move {
            let (account_id, email) = match user_snapshot {
                Some(Ok(Some(user))) => (user.id.unwrap(), user.email),
                _ => (ObjectId::new(), "anonymous@domain.com".to_string()),
            };
            
            let submission_doc = QuizAnswer {
                id: None, 
                quiz_id: current_quiz.id.unwrap(), 
                account_id,
                email,
                quiz_title: current_quiz.title,
                answers: answers_vec,
            };
            if let Ok(_) = submit_quiz_answer(submission_doc).await { quiz_submitted.set(true); }
        });
    };





    rsx! {
        style { "
            .quiz-choice-button:hover {{
                border-color: #93c5fd !important;
                box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
                background: #f0f9ff !important;
            }}
            .quiz-choice-button:active {{
                border-color: #2563eb !important;
                box-shadow: 0 2px 4px 0 rgba(0, 0, 0, 0.15);
            }}
        " }
        div { style: "max-width: 800px; margin: 0 auto;",
            if active_quiz.read().is_none() {
                h1 { style: "font-size: 1.75rem; font-weight: 700; margin-bottom: 1.5rem;", "Select Target Quiz Engine" }
                {
                    let items: Vec<_> = quizzes.read().as_ref()
                        .and_then(|r| r.as_ref().ok())
                        .map(|l| l.clone())
                        .unwrap_or_default();
                    rsx! {
                        for quiz in items {
                            div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",
                                div { h3 { style: "font-weight: 600;", "{quiz.title}" }, p { style: "color: #64748b;", "{quiz.description}" } }
                                button { style: "background: #2563eb; color: white; border: none; padding: 0.5rem 1rem; border-radius: 0.25rem; cursor: pointer;", onclick: move |_| start_quiz(quiz.clone()), "Launch" }
                            }
                        }
                    }
                }
            } else {
                {
                    let quiz = active_quiz.read().clone().unwrap();
                    if *quiz_submitted.read() {
                        rsx! {
                            div { style: "text-align: center; background: white; padding: 3rem; border-radius: 0.5rem;",
                                h2 { style: "color: #10b981;", "Submission Completed Successfully!" }
                                button { style: "margin-top: 1rem; background: #0f172a; color: white; padding: 0.5rem 1rem; border: none;", onclick: move |_| active_quiz.set(None), "Return" }
                            }
                        }
                    } else {
                        let idx = *current_question_idx.read();
                        let question = quiz.questions[idx].clone();
                        let time_left = *timer_seconds.read();
                        let timer_color = if time_left <= 5 { "#ef4444" } else if time_left <= 10 { "#f59e0b" } else { "#10b981" };
                        rsx! {
                            div { style: "background: white; padding: 2rem; border-radius: 0.5rem; border: 1px solid #e2e8f0;",
                                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.5rem;",
                                    h2 { style: "margin: 0;", "{question.text}" }
                                    div { style: "background: {timer_color}; color: white; padding: 0.5rem 1rem; border-radius: 0.5rem; font-weight: 700; font-size: 1.25rem;",
                                        "⏱ {time_left}s"
                                    }
                                }
                                div { style: "display: flex; flex-direction: column; gap: 0.5rem; margin-bottom: 2rem;",
                                    for choice in question.answer_choices.clone() {
                                        button { 
                                            style: "text-align: left; padding: 1rem; border: 2px solid #e2e8f0; border-radius: 0.5rem; background: white; cursor: pointer; transition: all 0.2s ease;", 
                                            class: "quiz-choice-button",
                                            onclick: move |_| select_choice(question.id, choice.text.clone()), 
                                            "{choice.text}" 
                                        }
                                    }
                                }
                                div { style: "display: flex; justify-content: space-between;",
                                    button { disabled: idx == 0, onclick: move |_| current_question_idx.set(idx - 1), "Back" }
                                    if idx + 1 < quiz.questions.len() {
                                        button { onclick: handle_next_click, "Next" }
                                    } else {
                                        button { style: "background: #10b981; color: white;", onclick: push_results, "Submit Answers" }
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
pub fn ResultsHistory() -> Element {
    let summaries_res = use_resource(move || get_user_results_summary());

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 2rem; max-width: 900px; margin: 0 auto;",
            h1 { style: "font-size: 1.75rem; font-weight: 700; color: #1e293b;", "Personal Quiz Performance Insights" }
            
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

                rsx! {
                    // Overall Stats Cards
                    div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 1.5rem;",
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
                            h3 { style: "font-size: 0.75rem; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "Quizzes Taken" }
                            p { style: "font-size: 2rem; font-weight: 800; color: #1e293b; margin-top: 0.5rem;", "{total_quizzes}" }
                        }
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
                            h3 { style: "font-size: 0.75rem; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "Total Correct" }
                            p { style: "font-size: 2rem; font-weight: 800; color: #10b981; margin-top: 0.5rem;", "{total_correct} / {total_questions}" }
                        }
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
                            h3 { style: "font-size: 0.75rem; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "Accuracy" }
                            p { style: "font-size: 2rem; font-weight: 800; color: #3b82f6; margin-top: 0.5rem;", "{accuracy}%" }
                        }
                        div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
                            h3 { style: "font-size: 0.75rem; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "Total Timeouts" }
                            p { style: "font-size: 2rem; font-weight: 800; color: #ef4444; margin-top: 0.5rem;", "{total_timeouts}" }
                        }
                    }

                    // Detailed History
                    div { style: "background: white; padding: 1.5rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05); margin-top: 1rem;",
                        h2 { style: "font-size: 1.25rem; font-weight: 700; color: #0f172a; margin-bottom: 1.5rem;", "Attempt History Log" }
                        if summaries.is_empty() {
                            div { style: "text-align: center; padding: 3rem; color: #64748b; font-weight: 500;", "No historical records found for this account" }
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
                                                            "Completed At: {formatted_date}"
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
                                                // Progress Bar representation of accuracy
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

#[component]
pub fn GlobalDiscussionsView() -> Element {
    let mut messages_res = use_resource(move || get_discussion_messages());
    let mut new_message = use_signal(|| String::new());
    
    // Auto-refresh polling every 5 seconds
    use_effect(move || {
        spawn(async move {
            loop {
                async_sleep(5).await;
                messages_res.restart();
            }
        });
    });

    let mut send_message = move |_| {
        let text = new_message.read().trim().to_string();
        if text.is_empty() { return; }
        new_message.set(String::new());
        spawn(async move {
            if let Ok(_) = create_discussion_message(text).await {
                messages_res.restart();
            }
        });
    };

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 1.5rem; max-width: 900px; margin: 0 auto; height: calc(100vh - 120px);",
            h1 { style: "font-size: 1.75rem; font-weight: 700; color: #1e293b;", "Global Chat Board" }
            
            div { style: "background: white; flex-grow: 1; border-radius: 0.5rem; border: 1px solid #e2e8f0; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                
                // Messages Area
                div { style: "flex-grow: 1; padding: 1.5rem; overflow-y: auto; display: flex; flex-direction: column; gap: 1rem; background: #f8fafc;",
                    if let Some(Ok(messages)) = &*messages_res.read() {
                        if messages.is_empty() {
                            div { style: "text-align: center; color: #94a3b8; font-style: italic; margin-top: 2rem;",
                                "No messages yet. Be the first to start the discussion!"
                            }
                        } else {
                            for msg in messages {
                                {
                                    let time_str = msg.created_at.format("%H:%M").to_string();
                                    rsx! {
                                        div { key: "{msg.id.map(|id| id.to_hex()).unwrap_or_default()}", style: "display: flex; flex-direction: column; gap: 0.25rem; background: white; padding: 1rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; align-self: flex-start; max-width: 85%;",
                                            div { style: "display: flex; justify-content: space-between; align-items: baseline; gap: 1rem;",
                                                span { style: "font-weight: 700; color: #3b82f6; font-size: 0.9rem;", "{msg.user_email}" }
                                                span { style: "font-size: 0.75rem; color: #94a3b8;", "{time_str}" }
                                            }
                                            p { style: "margin: 0; color: #334155; line-height: 1.4;", "{msg.content}" }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { "Loading messages..." }
                    }
                }
                
                // Input Area
                div { style: "padding: 1rem; background: white; border-top: 1px solid #e2e8f0; display: flex; gap: 1rem; align-items: center;",
                    input { 
                        style: "flex-grow: 1; padding: 0.75rem; border: 1px solid #cbd5e1; border-radius: 0.5rem; font-size: 1rem; outline: none;",
                        placeholder: "Type a message...",
                        value: "{new_message}",
                        oninput: move |e| new_message.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == dioxus::prelude::Key::Enter {
                                send_message(());
                            }
                        }
                    }
                    button { 
                        style: "background: #3b82f6; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 0.5rem; font-weight: 600; cursor: pointer; transition: background 0.2s;",
                        onclick: move |_| send_message(()),
                        "Send"
                    }
                }
            }
        }
    }
}
