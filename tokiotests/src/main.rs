use std::sync::Arc;
use anyhow::{bail, Result};
use tokio::{sync::Mutex, task::JoinHandle, time::Duration};

#[allow(dead_code)]
enum TaskType {
    NonBlocking,
    Blocking,
    BadVeryBad,
}

#[tokio::main]
async fn main() -> Result<()> {
    let handle = make_handle("sweetson", TaskType::NonBlocking, false).await?;
    let blhandle = make_handle("Blocky", TaskType::Blocking, false).await?;
    //let _bad_handle = make_handle("badboy", TaskType::BadVeryBad, false).await?;

    let count = Arc::new(Mutex::new(0));
    for i in 0..5 {
        println!("Spawning counter task {}...", i);
        clone_count_and_spawn(&count, i).await;
    }

    let handle2 = make_handle("sweatsun", TaskType::NonBlocking, false).await?;
    let blhandle2 = make_handle("Blocky II", TaskType::Blocking, false).await?;

    handle.await.expect("Handle failed");
    handle2.await.expect("Handle failed");
    blhandle.await.expect("Blocking handle failed");
    blhandle2.await.expect("Blocking handle failed");

    println!("Total count: {}", *count.lock().await);
    Ok(())
}

async fn make_handle(name: &'static str, task_type: TaskType, do_a_panic: bool) -> Result<JoinHandle<()>> {
    println!("Spawning task \"{}\"...", name);

    match task_type {
        TaskType::NonBlocking => {
            Ok(tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                if do_a_panic {
                    panic!("AAAHHHHH from task \"{}\"", name);
                }
                println!("Here's task \"{}\"", name);
            }))
        }
        TaskType::Blocking => {
            Ok(tokio::task::spawn_blocking(move || {
                let _ = tokio::time::sleep(Duration::from_millis(1000));
                if do_a_panic {
                    panic!("AAAHHHH from blocking task \"{}\"", name);
                }
                println!("Here's blocking task \"{}\", COMIN' THROUGH!", name);
            }))
        }
        _ => bail!("Bad task type for handle \"{}\"!", name),
    }
}

async fn clone_count_and_spawn(count: &Arc<Mutex<i32>>, task_num: i32) {
    let count_clone = Arc::clone(count);

    tokio::spawn(async move {
        for i in 0..5 {
            let mut actual_value = count_clone.lock().await;
            *actual_value += 1;
            println!("Task {}, inner-count: {}, count: {}", task_num, i, actual_value);
        }
    });
}
