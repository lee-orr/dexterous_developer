
// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use xilem::view::{button, checkbox, flex, textbox};
use xilem::{Axis, EventLoop, WidgetView, Xilem};
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

struct SharedCounter(u32);

reloadable_app!(TaskList, SharedCounter, app_logic (state) {
    let ( counter, task_list ) = interpret!(state).unwrap();

    let input_box = textbox(
        task_list.next_task.clone(),
        |state: state!(), new_value| {
            let ( counter, task_list ) = interpret!(state).unwrap();
            task_list.next_task = new_value;
        },
    )
    .on_enter(|state: state!(), _| {
        let ( counter, task_list ) = interpret!(state).unwrap();
        task_list.add_task();
    });
    let first_line = flex((
        input_box,
        button("Add task".to_string(), |state: state!()| {
            let ( counter, task_list ) = interpret!(state).unwrap();
            task_list.add_task();
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
                move |data: state!(), checked| {
                    let ( _, data ) = interpret!(data).unwrap();
                    data.tasks[i].done = checked;
                },
            );
            let delete_button = button("Delete", move |data: state!()| {
                let ( _, data ) = interpret!(data).unwrap();
                data.tasks.remove(i);
            });
            flex((checkbox, delete_button)).direction(Axis::Horizontal)
        })
        .collect::<Vec<_>>();

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

    let counter = SharedCounter(1);
    let app = Xilem::reloadable::<app_logic>(serializable, counter);
    app.run_windowed(EventLoop::with_user_event(), "First Example".into())
    .unwrap();
});