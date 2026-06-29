use dioxus::prelude::*;
use crate::models::{Quiz, QuizAnswer, Answer};
use crate::services::*;
use bson::oid::ObjectId;
use chrono::Utc;

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

#[component]
pub fn TakeQuizSelection() -> Element {
    let quizzes = use_resource(move || get_quizzes());
    let mut active_quiz = use_signal(|| Option::<Quiz>::None);
    let mut current_question_idx = use_signal(|| 0);
    let mut chosen_answers = use_signal(Vec::<Answer>::new);
    let mut runtime_start_time = use_signal(|| Utc::now());
    let mut quiz_submitted = use_signal(|| false);

    let mut start_quiz = move |quiz: Quiz| {
        active_quiz.set(Some(quiz));
        current_question_idx.set(0);
        chosen_answers.set(Vec::new());
        runtime_start_time.set(Utc::now());
        quiz_submitted.set(false);
    };

    let mut select_choice = move |q_id: ObjectId, text: String| {
        let answer = Answer { question_id: q_id, text, started: *runtime_start_time.read(), completed: Utc::now() };
        let mut answers_vec = chosen_answers.read().clone();
        answers_vec.retain(|a| a.question_id != q_id);
        answers_vec.push(answer);
        chosen_answers.set(answers_vec);
    };

    let push_results = move |_| {
        let current_quiz = active_quiz.read().clone().unwrap();
        spawn(async move {
            let submission_doc = QuizAnswer {
                id: None, quiz_id: current_quiz.id.unwrap(), account_id: ObjectId::new(),
                email: "tester_session@domain.com".to_string(), quiz_title: current_quiz.title,
                answers: chosen_answers.read().clone(),
            };
            if let Ok(_) = submit_quiz_answer(submission_doc).await { quiz_submitted.set(true); }
        });
    };

    rsx! {
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
                        rsx! {
                            div { style: "background: white; padding: 2rem; border-radius: 0.5rem; border: 1px solid #e2e8f0;",
                                h2 { style: "margin-bottom: 1.5rem;", "{question.text}" }
                                div { style: "display: flex; flex-direction: column; gap: 0.5rem; margin-bottom: 2rem;",
                                    for choice in question.answer_choices.clone() {
                                        button { style: "text-align: left; padding: 1rem; border: 1px solid #e2e8f0; background: white; cursor: pointer;", onclick: move |_| select_choice(question.id, choice.text.clone()), "{choice.text}" }
                                    }
                                }
                                div { style: "display: flex; justify-content: space-between;",
                                    button { disabled: idx == 0, onclick: move |_| current_question_idx.set(idx - 1), "Back" }
                                    if idx + 1 < quiz.questions.len() {
                                        button { onclick: move |_| current_question_idx.set(idx + 1), "Next" }
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
    let history_logs = use_resource(move || get_submissions(None));
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 1.5rem;",
            h1 { style: "font-size: 1.75rem; font-weight: 700;", "Historical Record Submissions" }
            {
                let records: Vec<_> = history_logs.read().as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .map(|l| l.clone())
                    .unwrap_or_default();
                rsx! {
                    for record in records {
                        div { style: "background: white; padding: 1.25rem; border-radius: 0.5rem; border: 1px solid #e2e8f0; display: flex; justify-content: space-between;",
                            div { h4 { style: "font-weight: 600;", "{record.quiz_title}" }, p { style: "font-size: 0.8rem; color: #94a3b8;", "User: {record.email}" } }
                            span { style: "background: #f1f5f9; padding: 0.25rem 0.5rem; height: max-content;", "{record.answers.len()} Responses" }
                        }
                    }
                }
            }
        }
    }
}
