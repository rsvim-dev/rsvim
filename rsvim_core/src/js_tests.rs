use super::js::*;

#[test]
fn next_task_id1() {
  let mut last_id: Option<TaskId> = None;
  for _i in 0..100 {
    let id = TaskId::next();
    if let Some(last_id) = last_id {
      assert_eq!(Into::<usize>::into(last_id) + 1, Into::<usize>::into(id));
    }
    last_id = Some(id);
  }
}
