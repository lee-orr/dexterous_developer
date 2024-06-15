// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use xilem::view::{button, checkbox, flex, textbox};
use xilem::{Axis, EventLoop, Xilem};
use xilem_dexterous_developer::*;

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    description: String,
    done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskList {
    next_task: String,
    tasks: Vec<Task>,
}

impl TaskList {
    fn add_task(&mut self) {
        if self.next_task.is_empty() {
            return;
        }
        self.tasks.push(Task {
            description: std::mem::take(&mut self.next_task),
            done: false,
        });
    }
}

reloadable_app!(TaskList, app_logic (task_list) {
    println!("Running Loop");
    let next_task = task_list.next_task.clone();
    println!("Cloned Successfully");

    let input_box = textbox(
        task_list.next_task.clone(),
        |state: &mut Self::State, new_value| {
            println!("In Callback For Value");
            state.next_task = new_value;
        },
    )
    .on_enter(|state: &mut Self::State, _| {
        println!("In Enter Callback");
        state.add_task();
    });

    println!("Got here");

    let first_line = flex((
        input_box,
        button("Add task".to_string(), |state: &mut Self::State| {
            state.add_task();
        }),
    ))
    .direction(Axis::Vertical);

    let tasks = task_list
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let checkbox = checkbox(
                task.description.clone(),
                task.done,
                move |data: &mut Self::State, checked| {
                    data.tasks[i].done = checked;
                },
            );
            let delete_button = button("Delete", move |data: &mut Self::State| {
                data.tasks.remove(i);
            });
            flex((checkbox, delete_button)).direction(Axis::Horizontal)
        })
        .collect::<Vec<_>>();

    println!("Returning");
    flex((first_line, tasks))
});

reloadable_main!(main() {
    let serializable = TaskList {
        next_task: String::new(),
        tasks: vec![
            Task {
                description: "Buy milk".into(),
                done: false,
            },
            Task {
                description: "Buy eggs".into(),
                done: true,
            },
            Task {
                description: "Buy bread".into(),
                done: false,
            },
        ],
    };

    let app = Xilem::reloadable::<app_logic>(serializable);
    app.run_windowed(EventLoop::with_user_event(), "First Example".into())
    .unwrap();
});
